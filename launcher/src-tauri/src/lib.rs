mod config;
mod domain;
mod error;
mod manifest;
mod paths;
mod release;

pub use domain::{Channel, InstalledBuild, LauncherConfig};
pub use error::{LauncherError, LauncherErrorCode};
pub use manifest::{
    ArchiveMetadata, ManagedFile, Manifest, ManifestError, build_manifest, manifest_json,
};
pub use paths::LauncherPaths;
pub use release::{
    GitHubAsset, GitHubRelease, PreviewRelease, ReleaseAsset, ReleaseClient, ReleaseError,
    select_release,
};

#[cfg(test)] mod target_contract;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run Voxtera Launcher");
}
