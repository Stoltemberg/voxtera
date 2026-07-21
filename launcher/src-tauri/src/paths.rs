use std::path::PathBuf;

use crate::LauncherError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherPaths {
    pub root: PathBuf,
    pub config_file: PathBuf,
    pub logs_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub default_install_dir: PathBuf,
    pub legacy_config: PathBuf,
}

impl LauncherPaths {
    pub fn discover() -> Result<Self, LauncherError> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or_else(LauncherError::config_invalid)?;
        let legacy_config = std::env::current_dir()
            .map_err(|_| LauncherError::config_invalid())?
            .join("launcher")
            .join("voxtera_config.json");
        let root = local_app_data.join("Voxtera");

        Ok(Self {
            config_file: root.join("launcher.json"),
            logs_dir: root.join("logs"),
            cache_dir: root.join("cache"),
            default_install_dir: root.join("game"),
            legacy_config,
            root,
        })
    }
}
