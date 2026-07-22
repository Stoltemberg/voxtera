use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{Manifest, integrity::is_link_or_reparse, verify_file};

const PROMOTION_PRESERVED_ROOTS: [&str; 3] = ["userdata", "screenshots", "settings"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairPlan {
    pub checked_files: usize,
    pub checked_bytes: u64,
    pub invalid_files: Vec<String>,
    pub invalid_bytes: u64,
    pub deletions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedRepairPlan {
    pub invalid_files: Vec<String>,
    pub invalid_bytes: u64,
}

impl RepairPlan {
    pub fn is_clean(&self) -> bool { self.invalid_files.is_empty() }

    pub fn confirm(self) -> ConfirmedRepairPlan {
        ConfirmedRepairPlan {
            invalid_files: self.invalid_files,
            invalid_bytes: self.invalid_bytes,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RepairError {
    #[error("repair manifest is invalid")]
    InvalidManifest,
    #[error("repair worker count must be greater than zero")]
    InvalidWorkerCount,
    #[error("repair worker failed")]
    WorkerFailed,
    #[error("repair filesystem operation failed")]
    Io(#[source] std::io::Error),
    #[error("repair path is unsafe: {0}")]
    Unsafe(String),
}

pub fn prepare_repair_staging(
    installation_root: &Path,
    staging_root: &Path,
    manifest: &Manifest,
    confirmed: &ConfirmedRepairPlan,
) -> Result<usize, RepairError> {
    manifest
        .validate()
        .map_err(|_| RepairError::InvalidManifest)?;
    validate_confirmation(manifest, confirmed)?;
    validate_real_directory(installation_root)?;
    validate_real_directory(staging_root)?;
    for file in &manifest.files {
        verify_file(&staging_root.join(&file.path), file.size, &file.sha256)
            .map_err(|_| RepairError::Unsafe("staging managed file is invalid".to_owned()))?;
    }

    let managed = manifest
        .files
        .iter()
        .map(|file| file.path.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut copied = 0_usize;
    for entry in WalkDir::new(installation_root)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry
                .path()
                .strip_prefix(installation_root)
                .ok()
                .and_then(|path| path.components().next())
                .and_then(|component| component.as_os_str().to_str())
                .is_none_or(|root| {
                    !PROMOTION_PRESERVED_ROOTS.contains(&root.to_ascii_lowercase().as_str())
                })
        })
    {
        let entry = entry
            .map_err(|error| RepairError::Unsafe(format!("live traversal failed: {error}")))?;
        if entry.file_type().is_symlink()
            || is_link_or_reparse(entry.path()).map_err(RepairError::Io)?
        {
            return Err(RepairError::Unsafe(
                "live installation contains a link or reparse point".to_owned(),
            ));
        }
        let relative = entry.path().strip_prefix(installation_root).map_err(|_| {
            RepairError::Unsafe("live entry escaped the installation root".to_owned())
        })?;
        let normalized = relative.to_string_lossy().replace('\\', "/");
        if managed.contains(&normalized.to_ascii_lowercase()) {
            continue;
        }
        let target = staging_root.join(relative);
        if entry.file_type().is_dir() {
            if !target.exists() {
                fs::create_dir(&target).map_err(RepairError::Io)?;
            }
        } else if entry.file_type().is_file() {
            if target.exists() {
                return Err(RepairError::Unsafe(
                    "unmanaged repair copy would overwrite staging".to_owned(),
                ));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(RepairError::Io)?;
            }
            fs::copy(entry.path(), target).map_err(RepairError::Io)?;
            copied += 1;
        } else {
            return Err(RepairError::Unsafe(
                "live installation contains a special entry".to_owned(),
            ));
        }
    }
    Ok(copied)
}

fn validate_confirmation(
    manifest: &Manifest,
    confirmed: &ConfirmedRepairPlan,
) -> Result<(), RepairError> {
    if confirmed.invalid_files.is_empty() {
        return Err(RepairError::Unsafe(
            "repair confirmation contains no invalid files".to_owned(),
        ));
    }
    let expected = manifest
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.size))
        .collect::<std::collections::HashMap<_, _>>();
    let mut total = 0_u64;
    let mut seen = HashSet::new();
    for path in &confirmed.invalid_files {
        if !seen.insert(path) {
            return Err(RepairError::Unsafe(
                "repair confirmation repeats a managed path".to_owned(),
            ));
        }
        let size = expected.get(path.as_str()).ok_or_else(|| {
            RepairError::Unsafe("repair confirmation contains an unmanaged path".to_owned())
        })?;
        total = total
            .checked_add(*size)
            .ok_or_else(|| RepairError::Unsafe("repair byte total overflow".to_owned()))?;
    }
    if total != confirmed.invalid_bytes {
        return Err(RepairError::Unsafe(
            "repair confirmation byte total is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_real_directory(path: &Path) -> Result<(), RepairError> {
    let metadata = fs::symlink_metadata(path).map_err(RepairError::Io)?;
    if !metadata.is_dir() || is_link_or_reparse(path).map_err(RepairError::Io)? {
        return Err(RepairError::Unsafe(
            "repair root is a link or reparse point".to_owned(),
        ));
    }
    Ok(())
}

pub fn plan_repair(
    installation_root: &Path,
    manifest: &Manifest,
    max_workers: usize,
) -> Result<RepairPlan, RepairError> {
    manifest
        .validate()
        .map_err(|_| RepairError::InvalidManifest)?;
    if max_workers == 0 {
        return Err(RepairError::InvalidWorkerCount);
    }
    let checked_bytes = manifest.files.iter().map(|file| file.size).sum();
    let workers = max_workers.min(manifest.files.len().max(1));
    let chunk_size = manifest.files.len().div_ceil(workers);
    let root = PathBuf::from(installation_root);
    let mut invalid = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in manifest.files.chunks(chunk_size.max(1)) {
            let root = &root;
            handles.push(scope.spawn(move || {
                chunk
                    .iter()
                    .filter(|file| {
                        verify_file(&root.join(&file.path), file.size, &file.sha256).is_err()
                    })
                    .map(|file| (file.path.clone(), file.size))
                    .collect::<Vec<_>>()
            }));
        }
        let mut invalid = Vec::new();
        for handle in handles {
            invalid.extend(handle.join().map_err(|_| RepairError::WorkerFailed)?);
        }
        Ok::<_, RepairError>(invalid)
    })?;
    invalid.sort_by(|left, right| left.0.cmp(&right.0));
    let invalid_bytes = invalid.iter().map(|(_, size)| size).sum();
    Ok(RepairPlan {
        checked_files: manifest.files.len(),
        checked_bytes,
        invalid_files: invalid.into_iter().map(|(path, _)| path).collect(),
        invalid_bytes,
        deletions: Vec::new(),
    })
}
