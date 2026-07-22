use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::{IntegrityError, Manifest, integrity::is_link_or_reparse, verify_file};

const EOCD_MIN_BYTES: usize = 22;
const MAX_ZIP_COMMENT_BYTES: usize = u16::MAX as usize;
const MAX_EOCD_SEARCH_BYTES: usize = EOCD_MIN_BYTES + MAX_ZIP_COMMENT_BYTES;
const MAX_CENTRAL_DIRECTORY_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PREFLIGHT_ENTRIES: usize = 8_192;
const CONSERVATIVE_ENTRY_METADATA_BYTES: usize = 2_048;
const ZIP_PARSER_MEMORY_BUDGET_BYTES: usize = MAX_CENTRAL_DIRECTORY_BYTES as usize
    + MAX_PREFLIGHT_ENTRIES * CONSERVATIVE_ENTRY_METADATA_BYTES
    + MAX_EOCD_SEARCH_BYTES
    + 64 * 1024;
const _: () = assert!(ZIP_PARSER_MEMORY_BUDGET_BYTES < 32 * 1024 * 1024);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    pub max_entries: usize,
    pub max_uncompressed_bytes: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_uncompressed_bytes: 32 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionReceipt {
    pub staging_dir: PathBuf,
    pub files_extracted: usize,
    pub bytes_extracted: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("archive integrity validation failed")]
    Integrity(#[from] IntegrityError),
    #[error("manifest is invalid")]
    Manifest,
    #[error("archive is unsafe: {0}")]
    Unsafe(String),
    #[error("ZIP preflight rejected the archive: {0}")]
    Preflight(String),
    #[error("archive I/O failed")]
    Io(#[source] std::io::Error),
    #[error("ZIP decoding failed")]
    Zip(#[source] zip::result::ZipError),
}

pub fn extract_to_staging(
    archive_path: &Path,
    staging_dir: &Path,
    manifest: &Manifest,
    limits: ArchiveLimits,
) -> Result<ExtractionReceipt, ArchiveError> {
    manifest.validate().map_err(|_| ArchiveError::Manifest)?;
    verify_file(
        archive_path,
        manifest.archive.size,
        &manifest.archive.sha256,
    )?;
    preflight_zip(archive_path, limits)?;
    if limits.max_entries == 0 {
        return Err(ArchiveError::Unsafe("entry ceiling is zero".to_owned()));
    }
    if fs::symlink_metadata(staging_dir).is_ok() {
        return Err(ArchiveError::Unsafe(
            "staging directory already exists".to_owned(),
        ));
    }
    let parent = staging_dir
        .parent()
        .ok_or_else(|| ArchiveError::Unsafe("staging directory has no parent".to_owned()))?;
    ensure_real_directory(parent)?;
    fs::create_dir(staging_dir).map_err(ArchiveError::Io)?;
    if is_link_or_reparse(staging_dir).map_err(ArchiveError::Io)? {
        let _ = fs::remove_dir(staging_dir);
        return Err(ArchiveError::Unsafe(
            "staging directory is a link or reparse point".to_owned(),
        ));
    }

    let result = extract_verified(archive_path, staging_dir, manifest, limits);
    if result.is_err() {
        let _ = fs::remove_dir_all(staging_dir);
    }
    result
}

fn extract_verified(
    archive_path: &Path,
    staging_dir: &Path,
    manifest: &Manifest,
    limits: ArchiveLimits,
) -> Result<ExtractionReceipt, ArchiveError> {
    let archive_file = File::open(archive_path).map_err(ArchiveError::Io)?;
    let mut archive = ZipArchive::new(archive_file).map_err(ArchiveError::Zip)?;
    if archive.len() > limits.max_entries {
        return Err(ArchiveError::Unsafe("too many archive entries".to_owned()));
    }

    let expected = manifest
        .files
        .iter()
        .map(|file| (file.path.to_ascii_lowercase(), file))
        .collect::<HashMap<_, _>>();
    let mut seen_entries = HashSet::new();
    let mut extracted = HashSet::new();
    let mut file_paths = HashSet::new();
    let mut total = 0_u64;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(ArchiveError::Zip)?;
        reject_special_entry(&entry)?;
        let is_directory = entry.is_dir();
        let normalized = normalize_windows_entry(entry.name(), is_directory)?;
        let key = normalized.to_ascii_lowercase();
        if !seen_entries.insert(key.clone()) {
            return Err(ArchiveError::Unsafe(
                "duplicate normalized archive path".to_owned(),
            ));
        }
        reject_file_directory_collision(&key, is_directory, &file_paths, &seen_entries)?;
        if !is_directory {
            file_paths.insert(key.clone());
        }

        total = total
            .checked_add(entry.size())
            .ok_or_else(|| ArchiveError::Unsafe("uncompressed size overflow".to_owned()))?;
        if total > limits.max_uncompressed_bytes {
            return Err(ArchiveError::Unsafe(
                "uncompressed byte ceiling exceeded".to_owned(),
            ));
        }

        if is_directory {
            if !expected
                .keys()
                .any(|path| path.starts_with(&format!("{key}/")))
            {
                return Err(ArchiveError::Unsafe(
                    "archive contains an unmanaged directory".to_owned(),
                ));
            }
            create_real_directories(staging_dir, &normalized)?;
            continue;
        }

        let managed = expected
            .get(&key)
            .ok_or_else(|| ArchiveError::Unsafe("archive contains an unmanaged file".to_owned()))?;
        let destination = staging_dir.join(managed.path.replace('/', "\\"));
        let relative_parent = Path::new(&managed.path)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        create_real_directories(staging_dir, &relative_parent.to_string_lossy())?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&destination)
            .map_err(ArchiveError::Io)?;
        let mut hasher = Sha256::new();
        let mut written = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = entry.read(&mut buffer).map_err(ArchiveError::Io)?;
            if count == 0 {
                break;
            }
            written = written
                .checked_add(count as u64)
                .ok_or_else(|| ArchiveError::Unsafe("file size overflow".to_owned()))?;
            if written > managed.size || written > limits.max_uncompressed_bytes {
                return Err(ArchiveError::Unsafe(
                    "extracted file exceeds declared size".to_owned(),
                ));
            }
            hasher.update(&buffer[..count]);
            output
                .write_all(&buffer[..count])
                .map_err(ArchiveError::Io)?;
        }
        output.flush().map_err(ArchiveError::Io)?;
        output.sync_all().map_err(ArchiveError::Io)?;
        if written != managed.size || hex::encode(hasher.finalize()) != managed.sha256 {
            return Err(ArchiveError::Unsafe(
                "extracted file does not match manifest".to_owned(),
            ));
        }
        extracted.insert(key);
    }

    if extracted.len() != expected.len() || expected.keys().any(|key| !extracted.contains(key)) {
        return Err(ArchiveError::Unsafe(
            "archive is missing managed files".to_owned(),
        ));
    }
    Ok(ExtractionReceipt {
        staging_dir: staging_dir.to_owned(),
        files_extracted: extracted.len(),
        bytes_extracted: total,
    })
}

fn normalize_windows_entry(raw: &str, is_directory: bool) -> Result<String, ArchiveError> {
    if raw.is_empty()
        || raw.starts_with('/')
        || raw.starts_with('\\')
        || raw.contains(':')
        || raw.contains('\0')
    {
        return Err(ArchiveError::Unsafe("absolute archive path".to_owned()));
    }
    let slash = raw.replace('\\', "/");
    let body = if is_directory {
        slash.strip_suffix('/').unwrap_or(&slash)
    } else {
        slash.as_str()
    };
    let mut components = Vec::new();
    for component in body.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.ends_with([' ', '.'])
            || is_windows_device_name(component)
        {
            return Err(ArchiveError::Unsafe(
                "unsafe Windows archive path".to_owned(),
            ));
        }
        components.push(component);
    }
    if components.is_empty() {
        return Err(ArchiveError::Unsafe("empty archive path".to_owned()));
    }
    Ok(components.join("/"))
}

fn is_windows_device_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|number| {
                matches!(
                    number,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
}

fn reject_special_entry(entry: &zip::read::ZipFile<'_, File>) -> Result<(), ArchiveError> {
    if let Some(mode) = entry.unix_mode() {
        let kind = mode & 0o170000;
        if kind != 0 && kind != 0o040000 && kind != 0o100000 {
            return Err(ArchiveError::Unsafe(
                "links and special entries are forbidden".to_owned(),
            ));
        }
    }
    Ok(())
}

fn reject_file_directory_collision(
    key: &str,
    is_directory: bool,
    file_paths: &HashSet<String>,
    seen: &HashSet<String>,
) -> Result<(), ArchiveError> {
    let mut ancestor = String::new();
    let components = key.split('/').collect::<Vec<_>>();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        if !ancestor.is_empty() {
            ancestor.push('/');
        }
        ancestor.push_str(component);
        if file_paths.contains(&ancestor) {
            return Err(ArchiveError::Unsafe(
                "file and directory paths collide".to_owned(),
            ));
        }
    }
    if !is_directory
        && seen
            .iter()
            .any(|prior| prior.starts_with(&format!("{key}/")))
    {
        return Err(ArchiveError::Unsafe(
            "file and directory paths collide".to_owned(),
        ));
    }
    Ok(())
}

fn create_real_directories(root: &Path, relative: &str) -> Result<(), ArchiveError> {
    let normalized = relative.replace('\\', "/");
    let mut current = root.to_owned();
    for component in normalized.split('/').filter(|part| !part.is_empty()) {
        current.push(component);
        match fs::create_dir(&current) {
            Ok(()) => {},
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {},
            Err(error) => return Err(ArchiveError::Io(error)),
        }
        if is_link_or_reparse(&current).map_err(ArchiveError::Io)? {
            return Err(ArchiveError::Unsafe(
                "staging path traverses a link or reparse point".to_owned(),
            ));
        }
        if !fs::metadata(&current).map_err(ArchiveError::Io)?.is_dir() {
            return Err(ArchiveError::Unsafe(
                "staging parent is not a directory".to_owned(),
            ));
        }
    }
    Ok(())
}

fn ensure_real_directory(path: &Path) -> Result<(), ArchiveError> {
    fs::create_dir_all(path).map_err(ArchiveError::Io)?;
    if !fs::metadata(path).map_err(ArchiveError::Io)?.is_dir()
        || is_link_or_reparse(path).map_err(ArchiveError::Io)?
    {
        return Err(ArchiveError::Unsafe("staging parent is unsafe".to_owned()));
    }
    Ok(())
}

fn preflight_zip(path: &Path, limits: ArchiveLimits) -> Result<(), ArchiveError> {
    let mut file = File::open(path).map_err(ArchiveError::Io)?;
    let length = file.metadata().map_err(ArchiveError::Io)?.len();
    let search_len = usize::try_from(length.min(MAX_EOCD_SEARCH_BYTES as u64))
        .map_err(|_| ArchiveError::Preflight("archive length overflow".to_owned()))?;
    if search_len < EOCD_MIN_BYTES {
        return Err(ArchiveError::Preflight("missing EOCD".to_owned()));
    }
    file.seek(SeekFrom::End(-(search_len as i64)))
        .map_err(ArchiveError::Io)?;
    let mut tail = vec![0_u8; search_len];
    file.read_exact(&mut tail).map_err(ArchiveError::Io)?;
    let eocd_in_tail = tail
        .windows(4)
        .rposition(|window| window == b"PK\x05\x06")
        .ok_or_else(|| ArchiveError::Preflight("missing EOCD".to_owned()))?;
    if eocd_in_tail + EOCD_MIN_BYTES > tail.len() {
        return Err(ArchiveError::Preflight("truncated EOCD".to_owned()));
    }
    let eocd = &tail[eocd_in_tail..];
    let comment_length = read_u16(eocd, 20)? as usize;
    if eocd_in_tail + EOCD_MIN_BYTES + comment_length != tail.len() {
        return Err(ArchiveError::Preflight(
            "EOCD does not terminate the archive".to_owned(),
        ));
    }
    if eocd_in_tail >= 20 && &tail[eocd_in_tail - 20..eocd_in_tail - 16] == b"PK\x06\x07" {
        return Err(ArchiveError::Preflight("ZIP64 is not supported".to_owned()));
    }
    let disk = read_u16(eocd, 4)?;
    let central_disk = read_u16(eocd, 6)?;
    let entries_on_disk = read_u16(eocd, 8)?;
    let entries = read_u16(eocd, 10)?;
    let central_size = read_u32(eocd, 12)?;
    let central_offset = read_u32(eocd, 16)?;
    if disk != 0 || central_disk != 0 || entries_on_disk != entries {
        return Err(ArchiveError::Preflight(
            "multi-disk ZIP is not supported".to_owned(),
        ));
    }
    if entries == u16::MAX || central_size == u32::MAX || central_offset == u32::MAX {
        return Err(ArchiveError::Preflight("ZIP64 is not supported".to_owned()));
    }
    let entries = entries as usize;
    if entries > limits.max_entries || entries > MAX_PREFLIGHT_ENTRIES {
        return Err(ArchiveError::Preflight(
            "central directory entry ceiling exceeded".to_owned(),
        ));
    }
    if central_size as u64 > MAX_CENTRAL_DIRECTORY_BYTES {
        return Err(ArchiveError::Preflight(
            "central directory byte ceiling exceeded".to_owned(),
        ));
    }
    let eocd_absolute = length - search_len as u64 + eocd_in_tail as u64;
    let central_end = (central_offset as u64)
        .checked_add(central_size as u64)
        .ok_or_else(|| ArchiveError::Preflight("central directory overflow".to_owned()))?;
    if central_end != eocd_absolute {
        return Err(ArchiveError::Preflight(
            "central directory bounds are inconsistent".to_owned(),
        ));
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ArchiveError> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| ArchiveError::Preflight("truncated EOCD".to_owned()))?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ArchiveError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| ArchiveError::Preflight("truncated EOCD".to_owned()))?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}
