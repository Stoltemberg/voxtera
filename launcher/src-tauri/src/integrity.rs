use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedFile {
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, thiserror::Error)]
pub enum IntegrityError {
    #[error("integrity file is inaccessible")]
    Io(#[source] std::io::Error),
    #[error("file size does not match the manifest")]
    SizeMismatch,
    #[error("file SHA-256 does not match the manifest")]
    HashMismatch,
    #[error("invalid expected SHA-256")]
    InvalidHash,
    #[error("integrity file is a link or reparse point")]
    UnsafeFile,
}

pub fn verify_file(
    path: &Path,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<VerifiedFile, IntegrityError> {
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(IntegrityError::InvalidHash);
    }
    if path_has_link_or_reparse_component(path).map_err(IntegrityError::Io)? {
        return Err(IntegrityError::UnsafeFile);
    }
    let metadata = std::fs::symlink_metadata(path).map_err(IntegrityError::Io)?;
    if !metadata.is_file() {
        return Err(IntegrityError::UnsafeFile);
    }
    if metadata.len() != expected_size {
        return Err(IntegrityError::SizeMismatch);
    }
    let file = File::open(path).map_err(IntegrityError::Io)?;

    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(IntegrityError::Io)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual = hex::encode(hasher.finalize());
    if actual != expected_sha256 {
        return Err(IntegrityError::HashMismatch);
    }
    Ok(VerifiedFile {
        size: metadata.len(),
        sha256: actual,
    })
}

pub(crate) fn is_link_or_reparse(path: &Path) -> std::io::Result<bool> {
    let metadata = std::fs::symlink_metadata(path)?;
    Ok(metadata.file_type().is_symlink() || is_link_or_reparse_metadata(&metadata))
}

pub(crate) fn path_has_link_or_reparse_component(path: &Path) -> std::io::Result<bool> {
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata)
                if metadata.file_type().is_symlink() || is_link_or_reparse_metadata(&metadata) =>
            {
                return Ok(true);
            },
            Ok(_) => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error),
        }
    }
    Ok(false)
}

fn is_link_or_reparse_metadata(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
