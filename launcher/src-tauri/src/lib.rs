mod archive;
mod config;
mod domain;
mod download;
mod error;
mod install;
mod integrity;
mod manifest;
mod paths;
mod progress;
mod release;
mod repair;

pub use archive::{ArchiveError, ArchiveLimits, ExtractionReceipt, extract_to_staging};
pub use domain::{Channel, InstalledBuild, LauncherConfig};
pub use download::{
    CancellationToken, DownloadError, DownloadManager, DownloadOutcome, DownloadRequest,
};
pub use error::{LauncherError, LauncherErrorCode};
pub use install::{FailurePoint, InstallError, InstallManager, PromotionReceipt, PromotionRequest};
pub use integrity::{IntegrityError, VerifiedFile, verify_file};
pub use manifest::{
    ArchiveMetadata, ManagedFile, Manifest, ManifestError, build_manifest, manifest_json,
};
pub use paths::LauncherPaths;
pub use progress::{DownloadProgress, ProgressThrottle};
pub use release::{
    GitHubAsset, GitHubRelease, PreviewRelease, ReleaseAsset, ReleaseClient, ReleaseError,
    select_release,
};
pub use repair::{ConfirmedRepairPlan, RepairError, RepairPlan, plan_repair};

#[cfg(test)] mod target_contract;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run Voxtera Launcher");
}
