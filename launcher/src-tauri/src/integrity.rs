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
    let file = File::open(path).map_err(IntegrityError::Io)?;
    let metadata = file.metadata().map_err(IntegrityError::Io)?;
    if !metadata.is_file() || metadata.len() != expected_size {
        return Err(IntegrityError::SizeMismatch);
    }

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
