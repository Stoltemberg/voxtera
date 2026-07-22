mod archive;
mod commands;
mod config;
mod domain;
mod download;
mod error;
mod events;
mod game;
mod install;
mod integrity;
mod manifest;
mod paths;
mod progress;
mod release;
mod repair;
mod service;

pub use archive::{ArchiveError, ArchiveLimits, ExtractionReceipt, extract_to_staging};
pub use domain::{Channel, InstalledBuild, LauncherConfig};
pub use download::{
    CancellationToken, DownloadError, DownloadManager, DownloadOutcome, DownloadRequest,
};
pub use error::{LauncherError, LauncherErrorCode};
pub use events::{
    LAUNCHER_PROGRESS_EVENT, LauncherProgressEvent, NoopProgressSink, ProgressSink,
    noop_progress_sink,
};
pub use game::{GameLauncher, ManagedProcess, ProcessRunner, SystemProcessRunner};
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
pub use repair::{
    ConfirmedRepairPlan, RepairError, RepairPlan, plan_repair, prepare_repair_staging,
};
pub use service::{
    InstallOutcome, LauncherOperation, LauncherPhase, LauncherService, LauncherSettingsInput,
    LauncherSnapshot, LauncherWorkflow, LocalBuild, ProductionWorkflow, ReleaseCheck,
    ServiceFuture, validate_install_dir,
};

#[cfg(test)] mod target_contract;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    commands::register(tauri::Builder::default())
        .run(tauri::generate_context!())
        .expect("failed to run Voxtera Launcher");
}
