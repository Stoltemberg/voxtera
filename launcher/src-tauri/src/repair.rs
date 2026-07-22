use std::path::{Path, PathBuf};

use crate::{Manifest, verify_file};

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
