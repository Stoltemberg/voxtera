mod config;
mod domain;
mod error;
mod paths;

pub use domain::{Channel, InstalledBuild, LauncherConfig};
pub use error::{LauncherError, LauncherErrorCode};
pub use paths::LauncherPaths;

#[cfg(test)] mod target_contract;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run Voxtera Launcher");
}
