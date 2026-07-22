use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    os::windows::ffi::OsStrExt,
    path::Path,
};

use serde::Deserialize;
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW, ReplaceFileW,
};

use crate::{LauncherConfig, LauncherError, LauncherPaths};

const SCHEMA_VERSION: u32 = 1;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyConfig {
    install_dir: std::path::PathBuf,
    installed_version: Option<String>,
}

impl LauncherConfig {
    pub fn default_for(paths: &LauncherPaths) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            install_dir: paths.default_install_dir.clone(),
            installed_version: None,
            bandwidth_limit_kib: None,
            start_minimized: false,
        }
    }

    pub fn load_or_migrate(paths: &LauncherPaths) -> Result<Self, LauncherError> {
        if paths.config_file.exists() {
            let bytes = fs::read(&paths.config_file).map_err(map_io_error)?;
            return serde_json::from_slice(&bytes).map_err(|_| LauncherError::config_invalid());
        }

        let config = if paths.legacy_config.exists() {
            let bytes = fs::read(&paths.legacy_config).map_err(map_io_error)?;
            let legacy: LegacyConfig =
                serde_json::from_slice(&bytes).map_err(|_| LauncherError::config_invalid())?;

            Self {
                schema_version: SCHEMA_VERSION,
                install_dir: legacy.install_dir,
                installed_version: normalize_version(legacy.installed_version),
                bandwidth_limit_kib: None,
                start_minimized: false,
            }
        } else {
            Self::default_for(paths)
        };

        config.save_atomic(paths)?;
        Ok(config)
    }

    pub fn save_atomic(&self, paths: &LauncherPaths) -> Result<(), LauncherError> {
        fs::create_dir_all(&paths.root).map_err(map_io_error)?;
        let temp_file = paths.root.join("launcher.json.tmp");
        let file = File::create(&temp_file).map_err(map_io_error)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, self)
            .map_err(|_| LauncherError::config_invalid())?;
        writer.flush().map_err(map_io_error)?;
        writer.get_ref().sync_all().map_err(map_io_error)?;
        drop(writer);
        replace_file_atomic(&temp_file, &paths.config_file).map_err(map_io_error)
    }
}

pub(crate) fn replace_file_atomic(temp_file: &Path, destination: &Path) -> std::io::Result<()> {
    let destination_exists = destination.exists();
    let temp_file = wide_path(temp_file);
    let destination = wide_path(destination);
    let replaced = if destination_exists {
        // SAFETY: Both paths are valid, null-terminated UTF-16 buffers that remain
        // alive for the duration of the call. The other optional pointers are
        // intentionally null.
        unsafe {
            ReplaceFileW(
                destination.as_ptr(),
                temp_file.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
            )
        }
    } else {
        // SAFETY: Both paths are valid, null-terminated UTF-16 buffers that remain
        // alive for the duration of the call. The flags request same-volume
        // replacement with write-through.
        unsafe {
            MoveFileExW(
                temp_file.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
    };

    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn wide_path(path: &Path) -> Vec<u16> { path.as_os_str().encode_wide().chain(Some(0)).collect() }

fn normalize_version(version: Option<String>) -> Option<String> {
    version.and_then(|version| {
        let trimmed = version.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn map_io_error(error: std::io::Error) -> LauncherError {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        LauncherError::permission_denied()
    } else {
        LauncherError::config_invalid()
    }
}
