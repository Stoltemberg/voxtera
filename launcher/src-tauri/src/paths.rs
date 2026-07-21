use std::path::{Path, PathBuf};

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
        let current_exe = std::env::current_exe().map_err(|_| LauncherError::config_invalid())?;

        Self::from_locations(&local_app_data, &current_exe)
    }

    fn from_locations(local_app_data: &Path, current_exe: &Path) -> Result<Self, LauncherError> {
        let executable_dir = current_exe
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .ok_or_else(LauncherError::config_invalid)?;
        let root = local_app_data.join("Voxtera");

        Ok(Self {
            config_file: root.join("launcher.json"),
            logs_dir: root.join("logs"),
            cache_dir: root.join("cache"),
            default_install_dir: root.join("game"),
            legacy_config: executable_dir.join("voxtera_config.json"),
            root,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::LauncherPaths;

    #[test]
    fn anchors_legacy_config_next_to_current_executable() {
        let paths = LauncherPaths::from_locations(
            Path::new(r"C:\Users\Player\AppData\Local"),
            Path::new(r"D:\Apps\Voxtera\voxtera-launcher.exe"),
        )
        .unwrap();

        assert_eq!(
            paths.legacy_config,
            Path::new(r"D:\Apps\Voxtera\voxtera_config.json")
        );
        assert_eq!(
            paths.root,
            Path::new(r"C:\Users\Player\AppData\Local\Voxtera")
        );
    }
}
