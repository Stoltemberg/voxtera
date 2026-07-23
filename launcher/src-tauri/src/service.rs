use std::{
    fs,
    future::Future,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
};

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    ArchiveLimits, CancellationToken, DownloadManager, DownloadRequest, GameLauncher, InstallError,
    InstallManager, LauncherConfig, LauncherError, LauncherErrorCode, LauncherPaths,
    LauncherProgressEvent, Manifest, PreviewRelease, ProcessRunner, ProgressSink, PromotionReceipt,
    PromotionRequest, ReleaseClient, RepairPlan, SystemProcessRunner, extract_to_staging,
    prepare_repair_staging,
};

pub type ServiceFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, LauncherError>> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct LocalBuild {
    pub valid: bool,
    pub manifest: Option<Manifest>,
}

impl LocalBuild {
    pub fn valid(manifest: Manifest) -> Self {
        Self {
            valid: true,
            manifest: Some(manifest),
        }
    }

    pub fn invalid() -> Self {
        Self {
            valid: false,
            manifest: None,
        }
    }

    pub fn invalid_with_manifest(manifest: Manifest) -> Self {
        Self {
            valid: false,
            manifest: Some(manifest),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseCheck {
    pub manifest: Manifest,
    archive_url: Option<String>,
}

impl ReleaseCheck {
    pub fn new(manifest: Manifest) -> Self {
        Self {
            manifest,
            archive_url: None,
        }
    }

    fn production(manifest: Manifest, archive_url: String) -> Self {
        Self {
            manifest,
            archive_url: Some(archive_url),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallOutcome {
    pub manifest: Manifest,
}

impl InstallOutcome {
    pub fn new(manifest: Manifest) -> Self { Self { manifest } }
}

pub trait LauncherWorkflow: Send + Sync + 'static {
    fn recover(&self, config: &LauncherConfig) -> Result<(), LauncherError>;
    fn inspect_local(&self, config: &LauncherConfig) -> LocalBuild;
    fn check_release(&self, cancellation: CancellationToken) -> ServiceFuture<'_, ReleaseCheck>;
    fn install_or_update(
        &self,
        config: &LauncherConfig,
        release: ReleaseCheck,
        cancellation: CancellationToken,
    ) -> ServiceFuture<'_, InstallOutcome>;
    fn scan_repair(
        &self,
        config: &LauncherConfig,
        manifest: &Manifest,
    ) -> Result<RepairPlan, LauncherError>;
    fn repair(
        &self,
        config: &LauncherConfig,
        manifest: Manifest,
        plan: RepairPlan,
        cancellation: CancellationToken,
    ) -> ServiceFuture<'_, InstallOutcome>;
    fn confirm_first_launch(&self) -> Result<(), LauncherError>;
    fn rollback_pending(&self) -> Result<(), LauncherError>;
    fn launch_game(
        &self,
        config: &LauncherConfig,
        manifest: &Manifest,
    ) -> Result<u32, LauncherError>;
    fn has_pending_promotion(&self) -> bool;
}

struct PendingPromotion {
    receipt: PromotionReceipt,
}

pub struct ProductionWorkflow<R = SystemProcessRunner> {
    paths: LauncherPaths,
    release_client: ReleaseClient,
    manifest_client: reqwest::Client,
    game: GameLauncher<R>,
    pending: Mutex<Option<PendingPromotion>>,
    latest_release: Mutex<Option<ReleaseCheck>>,
    progress: Arc<dyn ProgressSink>,
}

impl<R> ProductionWorkflow<R>
where
    R: ProcessRunner,
{
    pub fn new(
        paths: LauncherPaths,
        runner: R,
        progress: Arc<dyn ProgressSink>,
    ) -> Result<Self, LauncherError> {
        let manifest_client = reqwest::Client::builder()
            .https_only(true)
            .user_agent(concat!("VoxteraLauncher/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|_| LauncherError::network())?;
        Ok(Self {
            paths,
            release_client: ReleaseClient::new().map_err(|_| LauncherError::network())?,
            manifest_client,
            game: GameLauncher::new(runner),
            pending: Mutex::new(None),
            latest_release: Mutex::new(None),
            progress,
        })
    }

    async fn fetch_release(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<ReleaseCheck, LauncherError> {
        let release = tokio::select! {
            _ = cancellation.cancelled() => return Err(LauncherError::cancelled()),
            release = self.release_client.latest_preview() => {
                release.map_err(|error| {
                    if error.code() == "network" {
                        LauncherError::network()
                    } else {
                        LauncherError::release_unavailable()
                    }
                })?
            }
        };
        let manifest = tokio::select! {
            _ = cancellation.cancelled() => return Err(LauncherError::cancelled()),
            response = self.manifest_client.get(&release.manifest.url).send() => {
                response
                    .and_then(reqwest::Response::error_for_status)
                    .map_err(|_| LauncherError::network())?
                    .json::<Manifest>()
                    .await
                    .map_err(|_| LauncherError::integrity())?
            }
        };
        validate_release_manifest(&release, &manifest)?;
        Ok(ReleaseCheck::production(manifest, release.game_archive.url))
    }

    async fn extract_release(
        &self,
        config: &LauncherConfig,
        release: &ReleaseCheck,
        cancellation: CancellationToken,
        operation: LauncherOperation,
    ) -> Result<(InstallManager, PathBuf), LauncherError> {
        let manager = InstallManager::new(config.install_dir.clone());
        let staging = manager.create_staging().map_err(map_install_error)?;
        fs::remove_dir(&staging).map_err(map_io_error)?;
        let archive = self
            .paths
            .cache_dir
            .join(format!("Voxtera-{}.zip", release.manifest.version));
        let download = DownloadManager::new(cancellation.clone())
            .map_err(map_download_error)?
            .download(
                DownloadRequest {
                    asset: crate::ReleaseAsset {
                        name: release.manifest.archive.name.clone(),
                        size: release.manifest.archive.size,
                        url: self.release_asset_url(release)?,
                    },
                    expected_sha256: release.manifest.archive.sha256.clone(),
                    cache_path: archive,
                    bandwidth_limit_kib: config.bandwidth_limit_kib,
                },
                {
                    let progress = Arc::clone(&self.progress);
                    move |event| {
                        progress.emit(LauncherProgressEvent::download(operation, event));
                    }
                },
            )
            .await
            .map_err(map_download_error)?;
        if cancellation.is_cancelled() {
            return Err(LauncherError::cancelled());
        }
        self.progress
            .emit(LauncherProgressEvent::stage(operation, "extracting"));
        extract_to_staging(
            &download.part_path,
            &staging,
            &release.manifest,
            ArchiveLimits::default(),
        )
        .map_err(map_archive_error)?;
        Ok((manager, staging))
    }

    fn release_asset_url(&self, release: &ReleaseCheck) -> Result<String, LauncherError> {
        let latest = self
            .latest_release
            .lock()
            .map_err(|_| LauncherError::release_unavailable())?;
        let cached = latest
            .as_ref()
            .filter(|cached| cached.manifest.version == release.manifest.version)
            .ok_or_else(LauncherError::release_unavailable)?;
        // The URL is stored separately from the public release snapshot so the frontend
        // can never supply an arbitrary download target.
        cached
            .archive_url
            .clone()
            .ok_or_else(LauncherError::release_unavailable)
    }

    fn promote(
        &self,
        manager: InstallManager,
        staging: PathBuf,
        manifest: &Manifest,
    ) -> Result<InstallOutcome, LauncherError> {
        self.backup_installed_manifest()?;
        let receipt = manager
            .promote(PromotionRequest {
                staging_dir: staging,
                failure_point: None,
            })
            .map_err(map_install_error)?;
        if let Err(error) = self.save_installed_manifest(manifest) {
            let _ = manager.rollback(&receipt);
            let _ = self.restore_installed_manifest();
            return Err(error);
        }
        *self
            .pending
            .lock()
            .map_err(|_| LauncherError::integrity())? = Some(PendingPromotion { receipt });
        Ok(InstallOutcome::new(manifest.clone()))
    }

    fn installed_manifest_path(&self) -> PathBuf { self.paths.root.join("installed-manifest.json") }

    fn rollback_manifest_path(&self) -> PathBuf {
        self.paths.root.join("installed-manifest.rollback.json")
    }

    fn backup_installed_manifest(&self) -> Result<(), LauncherError> {
        fs::create_dir_all(&self.paths.root).map_err(map_io_error)?;
        let source = self.installed_manifest_path();
        let backup = self.rollback_manifest_path();
        match fs::read(source) {
            Ok(bytes) => write_atomic_bytes(&backup, &bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                remove_if_present(&backup)
            },
            Err(error) => Err(map_io_error(error)),
        }
    }

    fn save_installed_manifest(&self, manifest: &Manifest) -> Result<(), LauncherError> {
        let bytes = serde_json::to_vec_pretty(manifest).map_err(|_| LauncherError::integrity())?;
        write_atomic_bytes(&self.installed_manifest_path(), &bytes)
    }

    fn restore_installed_manifest(&self) -> Result<(), LauncherError> {
        let backup = self.rollback_manifest_path();
        match fs::read(&backup) {
            Ok(bytes) => {
                write_atomic_bytes(&self.installed_manifest_path(), &bytes)?;
                remove_if_present(&backup)
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                remove_if_present(&self.installed_manifest_path())
            },
            Err(error) => Err(map_io_error(error)),
        }
    }
}

impl<R> LauncherWorkflow for ProductionWorkflow<R>
where
    R: ProcessRunner + 'static,
{
    fn recover(&self, config: &LauncherConfig) -> Result<(), LauncherError> {
        let manager = InstallManager::new(config.install_dir.clone());
        manager.recover().map_err(map_install_error)?;
        let pending = manager.pending_promotion().map_err(map_install_error)?;
        *self
            .pending
            .lock()
            .map_err(|_| LauncherError::integrity())? =
            pending.map(|receipt| PendingPromotion { receipt });

        if self
            .pending
            .lock()
            .map_err(|_| LauncherError::integrity())?
            .is_none()
            && self.rollback_manifest_path().exists()
        {
            let current_version = read_manifest(&self.installed_manifest_path())
                .ok()
                .map(|manifest| manifest.version.to_string());
            if current_version.as_deref() == config.installed_version.as_deref() {
                remove_if_present(&self.rollback_manifest_path())?;
            } else {
                self.restore_installed_manifest()?;
            }
        }
        Ok(())
    }

    fn inspect_local(&self, config: &LauncherConfig) -> LocalBuild {
        let Some(installed_version) = config.installed_version.as_deref() else {
            return LocalBuild::invalid();
        };
        let Ok(manifest) = read_manifest(&self.installed_manifest_path()) else {
            return LocalBuild::invalid();
        };
        if manifest.version.to_string() != installed_version || manifest.validate().is_err() {
            return LocalBuild::invalid();
        }
        if !plan_repair_is_clean(&config.install_dir, &manifest) {
            return LocalBuild::invalid_with_manifest(manifest);
        }
        LocalBuild::valid(manifest)
    }

    fn check_release(&self, cancellation: CancellationToken) -> ServiceFuture<'_, ReleaseCheck> {
        Box::pin(async move {
            let release = self.fetch_release(&cancellation).await?;
            *self
                .latest_release
                .lock()
                .map_err(|_| LauncherError::release_unavailable())? = Some(release.clone());
            Ok(release)
        })
    }

    fn install_or_update(
        &self,
        config: &LauncherConfig,
        release: ReleaseCheck,
        cancellation: CancellationToken,
    ) -> ServiceFuture<'_, InstallOutcome> {
        let config = config.clone();
        Box::pin(async move {
            self.progress.emit(LauncherProgressEvent::stage(
                LauncherOperation::Installing,
                "preparing",
            ));
            let (manager, staging) = self
                .extract_release(
                    &config,
                    &release,
                    cancellation,
                    LauncherOperation::Installing,
                )
                .await?;
            self.progress.emit(LauncherProgressEvent::stage(
                LauncherOperation::Installing,
                "promoting",
            ));
            self.promote(manager, staging, &release.manifest)
        })
    }

    fn scan_repair(
        &self,
        config: &LauncherConfig,
        manifest: &Manifest,
    ) -> Result<RepairPlan, LauncherError> {
        crate::plan_repair(
            &config.install_dir,
            manifest,
            std::thread::available_parallelism().map_or(1, usize::from),
        )
        .map_err(|_| LauncherError::integrity())
    }

    fn repair(
        &self,
        config: &LauncherConfig,
        manifest: Manifest,
        plan: RepairPlan,
        cancellation: CancellationToken,
    ) -> ServiceFuture<'_, InstallOutcome> {
        let config = config.clone();
        Box::pin(async move {
            let release = self
                .latest_release
                .lock()
                .map_err(|_| LauncherError::release_unavailable())?
                .clone()
                .filter(|release| release.manifest.version == manifest.version)
                .ok_or_else(LauncherError::release_unavailable)?;
            let (manager, staging) = self
                .extract_release(
                    &config,
                    &release,
                    cancellation,
                    LauncherOperation::Repairing,
                )
                .await?;
            prepare_repair_staging(&config.install_dir, &staging, &manifest, &plan.confirm())
                .map_err(|_| LauncherError::integrity())?;
            self.promote(manager, staging, &manifest)
        })
    }

    fn confirm_first_launch(&self) -> Result<(), LauncherError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| LauncherError::integrity())?
            .take();
        let Some(pending) = pending else {
            return Ok(());
        };
        let manager = InstallManager::new(pending.receipt.installation_dir.clone());
        if let Err(error) = manager.confirm_first_launch(&pending.receipt) {
            *self
                .pending
                .lock()
                .map_err(|_| LauncherError::integrity())? = Some(pending);
            return Err(map_install_error(error));
        }
        remove_if_present(&self.rollback_manifest_path())
    }

    fn rollback_pending(&self) -> Result<(), LauncherError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| LauncherError::integrity())?
            .take();
        let Some(pending) = pending else {
            return Ok(());
        };
        let manager = InstallManager::new(pending.receipt.installation_dir.clone());
        if pending.receipt.rollback_dir.exists()
            && let Err(error) = manager.rollback(&pending.receipt)
        {
            *self
                .pending
                .lock()
                .map_err(|_| LauncherError::integrity())? = Some(pending);
            return Err(map_install_error(error));
        }
        self.restore_installed_manifest()
    }

    fn launch_game(
        &self,
        config: &LauncherConfig,
        manifest: &Manifest,
    ) -> Result<u32, LauncherError> {
        self.game.launch(&config.install_dir, manifest, || {
            self.confirm_first_launch()
        })
    }

    fn has_pending_promotion(&self) -> bool {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }
}

fn validate_release_manifest(
    release: &PreviewRelease,
    manifest: &Manifest,
) -> Result<(), LauncherError> {
    if manifest.version != release.version
        || manifest.archive.name != release.game_archive.name
        || manifest.archive.size != release.game_archive.size
    {
        return Err(LauncherError::integrity());
    }
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("the package version is a valid semantic version");
    if manifest.minimum_launcher_version <= current {
        manifest
            .validate()
            .map_err(|_| LauncherError::integrity())?;
    }
    Ok(())
}

fn plan_repair_is_clean(install_dir: &Path, manifest: &Manifest) -> bool {
    crate::plan_repair(
        install_dir,
        manifest,
        std::thread::available_parallelism().map_or(1, usize::from),
    )
    .is_ok_and(|plan| plan.is_clean())
}

fn read_manifest(path: &Path) -> Result<Manifest, LauncherError> {
    serde_json::from_slice(&fs::read(path).map_err(map_io_error)?)
        .map_err(|_| LauncherError::integrity())
}

fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), LauncherError> {
    let parent = path.parent().ok_or_else(LauncherError::config_invalid)?;
    fs::create_dir_all(parent).map_err(map_io_error)?;
    let temp = path.with_extension("json.tmp");
    let file = fs::File::create(&temp).map_err(map_io_error)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(bytes).map_err(map_io_error)?;
    writer.flush().map_err(map_io_error)?;
    writer.get_ref().sync_all().map_err(map_io_error)?;
    drop(writer);
    crate::config::replace_file_atomic(&temp, path).map_err(map_io_error)
}

fn remove_if_present(path: &Path) -> Result<(), LauncherError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(map_io_error(error)),
    }
}

fn map_io_error(error: std::io::Error) -> LauncherError {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        LauncherError::permission_denied()
    } else if error.raw_os_error() == Some(112) {
        LauncherError::disk_space()
    } else {
        LauncherError::integrity()
    }
}

fn map_download_error(error: crate::DownloadError) -> LauncherError {
    if error.is_cancelled() {
        LauncherError::cancelled()
    } else if error.is_network() {
        LauncherError::network()
    } else {
        LauncherError::integrity()
    }
}

fn map_archive_error(error: crate::ArchiveError) -> LauncherError {
    match error {
        crate::ArchiveError::Unsafe(_) | crate::ArchiveError::Preflight(_) => {
            LauncherError::unsafe_archive()
        },
        crate::ArchiveError::Io(error) => map_io_error(error),
        _ => LauncherError::integrity(),
    }
}

fn map_install_error(error: InstallError) -> LauncherError {
    match error {
        InstallError::GameRunning => LauncherError::game_running(),
        InstallError::PendingConfirmation => LauncherError::busy(),
        InstallError::Io(error) => map_io_error(error),
        _ => LauncherError::integrity(),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LauncherPhase {
    NeedsInstall,
    Ready,
    UpdateAvailable,
    Offline,
    RepairRequired,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LauncherOperation {
    CheckingRelease,
    Installing,
    ScanningRepair,
    Repairing,
    SavingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LauncherSettingsInput {
    pub install_dir: PathBuf,
    pub bandwidth_limit_kib: Option<u64>,
    pub start_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LauncherSnapshot {
    pub phase: LauncherPhase,
    pub installed_version: Option<String>,
    pub available_version: Option<String>,
    pub install_dir: std::path::PathBuf,
    pub local_build_valid: bool,
    pub operation: Option<LauncherOperation>,
    pub last_error: Option<LauncherError>,
}

struct ServiceState {
    config: LauncherConfig,
    snapshot: LauncherSnapshot,
    local_manifest: Option<Manifest>,
    release: Option<ReleaseCheck>,
    repair_plan: Option<RepairPlan>,
}

struct ActiveOperation {
    cancellation: CancellationToken,
}

pub struct LauncherService<W> {
    paths: LauncherPaths,
    workflow: W,
    state: Mutex<ServiceState>,
    operation: Mutex<Option<ActiveOperation>>,
}

impl<W> LauncherService<W>
where
    W: LauncherWorkflow,
{
    pub fn from_config(
        paths: LauncherPaths,
        config: LauncherConfig,
        workflow: W,
    ) -> Result<Self, LauncherError> {
        workflow.recover(&config)?;
        let local = workflow.inspect_local(&config);
        let phase = if local.valid {
            LauncherPhase::Ready
        } else {
            LauncherPhase::NeedsInstall
        };
        let snapshot = LauncherSnapshot {
            phase,
            installed_version: config.installed_version.clone(),
            available_version: None,
            install_dir: config.install_dir.clone(),
            local_build_valid: local.valid,
            operation: None,
            last_error: None,
        };
        Ok(Self {
            paths,
            workflow,
            state: Mutex::new(ServiceState {
                config,
                snapshot,
                local_manifest: local.manifest,
                release: None,
                repair_plan: None,
            }),
            operation: Mutex::new(None),
        })
    }

    pub fn get_snapshot(&self) -> LauncherSnapshot {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .snapshot
            .clone()
    }

    pub async fn check_release(&self) -> Result<LauncherSnapshot, LauncherError> {
        let cancellation = self.begin(LauncherOperation::CheckingRelease)?;
        let result = self.workflow.check_release(cancellation).await;
        self.end_operation();
        match result {
            Ok(release) => {
                self.accept_release(release)?;
                Ok(self.get_snapshot())
            },
            Err(error) if error.code == LauncherErrorCode::Network => {
                let mut state = self.lock_state();
                if state.snapshot.local_build_valid {
                    state.snapshot.phase = LauncherPhase::Offline;
                    state.snapshot.last_error = None;
                    Ok(state.snapshot.clone())
                } else {
                    state.snapshot.phase = LauncherPhase::Error;
                    state.snapshot.last_error = Some(error.clone());
                    Err(error)
                }
            },
            Err(error) => {
                self.record_error(&error);
                Err(error)
            },
        }
    }

    pub fn accept_release(&self, release: ReleaseCheck) -> Result<LauncherSnapshot, LauncherError> {
        let current = Version::parse(env!("CARGO_PKG_VERSION"))
            .expect("the package version is a valid semantic version");
        if release.manifest.minimum_launcher_version > current {
            let error = LauncherError::launcher_incompatible();
            self.record_error(&error);
            return Err(error);
        }
        release
            .manifest
            .validate()
            .map_err(|_| LauncherError::integrity())
            .inspect_err(|error| self.record_error(error))?;
        let mut state = self.lock_state();
        let available = release.manifest.version.to_string();
        state.snapshot.available_version = Some(available.clone());
        state.snapshot.phase = if state.snapshot.local_build_valid {
            if state.snapshot.installed_version.as_deref() == Some(available.as_str()) {
                LauncherPhase::Ready
            } else {
                LauncherPhase::UpdateAvailable
            }
        } else {
            LauncherPhase::NeedsInstall
        };
        state.snapshot.last_error = None;
        state.release = Some(release);
        Ok(state.snapshot.clone())
    }

    pub async fn install_or_update(&self) -> Result<LauncherSnapshot, LauncherError> {
        let release = self
            .lock_state()
            .release
            .clone()
            .ok_or_else(LauncherError::release_unavailable)?;
        let cancellation = self.begin(LauncherOperation::Installing)?;
        let config = self.lock_state().config.clone();
        let result = self
            .workflow
            .install_or_update(&config, release, cancellation)
            .await;
        self.end_operation();
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(error) => {
                self.record_error(&error);
                return Err(error);
            },
        };

        let mut state = self.lock_state();
        let previous = state.config.clone();
        state.config.installed_version = Some(outcome.manifest.version.to_string());
        if let Err(error) = state.config.save_atomic(&self.paths) {
            state.config = previous;
            drop(state);
            let final_error = self.workflow.rollback_pending().err().unwrap_or(error);
            self.record_error(&final_error);
            return Err(final_error);
        }
        state.snapshot.installed_version = state.config.installed_version.clone();
        state.snapshot.available_version = state.config.installed_version.clone();
        state.snapshot.local_build_valid = true;
        state.snapshot.phase = LauncherPhase::Ready;
        state.snapshot.last_error = None;
        state.local_manifest = Some(outcome.manifest);
        Ok(state.snapshot.clone())
    }

    pub fn scan_repair(&self) -> Result<RepairPlan, LauncherError> {
        let _cancellation = self.begin(LauncherOperation::ScanningRepair)?;
        let manifest = self
            .lock_state()
            .local_manifest
            .clone()
            .ok_or_else(LauncherError::repair_required);
        let config = self.lock_state().config.clone();
        let result = manifest.and_then(|manifest| self.workflow.scan_repair(&config, &manifest));
        self.end_operation();
        match result {
            Ok(plan) => {
                let mut state = self.lock_state();
                let clean = plan.is_clean();
                state.snapshot.local_build_valid = clean;
                state.snapshot.phase = if clean {
                    LauncherPhase::Ready
                } else {
                    LauncherPhase::RepairRequired
                };
                state.repair_plan = Some(plan.clone());
                Ok(plan)
            },
            Err(error) => {
                self.record_error(&error);
                Err(error)
            },
        }
    }

    pub async fn repair(&self) -> Result<LauncherSnapshot, LauncherError> {
        let (manifest, plan) = {
            let state = self.lock_state();
            (
                state
                    .local_manifest
                    .clone()
                    .ok_or_else(LauncherError::repair_required)?,
                state
                    .repair_plan
                    .clone()
                    .ok_or_else(LauncherError::repair_required)?,
            )
        };
        if plan.is_clean() {
            return Ok(self.get_snapshot());
        }
        let cancellation = self.begin(LauncherOperation::Repairing)?;
        let config = self.lock_state().config.clone();
        let result = self
            .workflow
            .repair(&config, manifest, plan, cancellation)
            .await;
        self.end_operation();
        match result {
            Ok(outcome) => {
                let mut state = self.lock_state();
                state.local_manifest = Some(outcome.manifest);
                state.snapshot.local_build_valid = true;
                state.snapshot.phase = LauncherPhase::Ready;
                state.snapshot.last_error = None;
                Ok(state.snapshot.clone())
            },
            Err(error) => {
                self.record_error(&error);
                Err(error)
            },
        }
    }

    pub fn cancel_operation(&self) -> Result<(), LauncherError> {
        let operation = self.operation.lock().map_err(|_| LauncherError::busy())?;
        if let Some(active) = operation.as_ref() {
            active.cancellation.cancel();
        }
        Ok(())
    }

    pub fn launch_game(&self) -> Result<u32, LauncherError> {
        let operation = self.operation.lock().map_err(|_| LauncherError::busy())?;
        if operation.is_some() {
            return Err(LauncherError::busy());
        }
        let (config, manifest) = {
            let state = self.lock_state();
            if !state.snapshot.local_build_valid {
                return Err(LauncherError::repair_required());
            }
            (
                state.config.clone(),
                state
                    .local_manifest
                    .clone()
                    .ok_or_else(LauncherError::repair_required)?,
            )
        };
        self.workflow.launch_game(&config, &manifest)
    }

    pub fn save_settings(
        &self,
        settings: LauncherSettingsInput,
    ) -> Result<LauncherSnapshot, LauncherError> {
        let _cancellation = self.begin(LauncherOperation::SavingSettings)?;
        let result = self.save_settings_inner(settings);
        self.end_operation();
        match result {
            Ok(_) => Ok(self.get_snapshot()),
            Err(error) => {
                self.record_error(&error);
                Err(error)
            },
        }
    }

    fn save_settings_inner(
        &self,
        settings: LauncherSettingsInput,
    ) -> Result<LauncherSnapshot, LauncherError> {
        let install_dir = validate_install_dir(&settings.install_dir)?;
        let mut next = self.lock_state().config.clone();
        if install_dir != next.install_dir && self.workflow.has_pending_promotion() {
            return Err(LauncherError::busy());
        }
        next.install_dir = install_dir;
        next.bandwidth_limit_kib = settings.bandwidth_limit_kib;
        next.start_minimized = settings.start_minimized;
        self.workflow.recover(&next)?;
        let local = self.workflow.inspect_local(&next);
        next.save_atomic(&self.paths)?;

        let mut state = self.lock_state();
        state.config = next;
        state.local_manifest = local.manifest;
        state.release = None;
        state.repair_plan = None;
        state.snapshot.install_dir = state.config.install_dir.clone();
        state.snapshot.installed_version = state.config.installed_version.clone();
        state.snapshot.available_version = None;
        state.snapshot.local_build_valid = local.valid;
        state.snapshot.operation = Some(LauncherOperation::SavingSettings);
        state.snapshot.last_error = None;
        state.snapshot.phase = if local.valid {
            LauncherPhase::Ready
        } else {
            LauncherPhase::NeedsInstall
        };
        Ok(state.snapshot.clone())
    }

    fn begin(&self, kind: LauncherOperation) -> Result<CancellationToken, LauncherError> {
        let mut operation = self.operation.lock().map_err(|_| LauncherError::busy())?;
        if operation.is_some() {
            return Err(LauncherError::busy());
        }
        let cancellation = CancellationToken::new();
        *operation = Some(ActiveOperation {
            cancellation: cancellation.clone(),
        });
        self.lock_state().snapshot.operation = Some(kind);
        Ok(cancellation)
    }

    fn end_operation(&self) {
        let mut operation = self
            .operation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *operation = None;
        self.lock_state().snapshot.operation = None;
    }

    fn record_error(&self, error: &LauncherError) {
        let mut state = self.lock_state();
        state.snapshot.last_error = Some(error.clone());
        state.snapshot.phase = if state.snapshot.local_build_valid {
            LauncherPhase::Ready
        } else {
            LauncherPhase::Error
        };
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, ServiceState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

pub fn validate_install_dir(path: &Path) -> Result<PathBuf, LauncherError> {
    if !path.is_absolute() || path.parent().is_none() || path.file_name().is_none() {
        return Err(LauncherError::config_invalid());
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_dir() => return Err(LauncherError::config_invalid()),
        Ok(_) => {},
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or_else(LauncherError::config_invalid)?;
            if !parent.is_dir() {
                return Err(LauncherError::config_invalid());
            }
        },
        Err(error) => return Err(map_io_error(error)),
    }
    if crate::integrity::path_has_link_or_reparse_component(path).map_err(map_io_error)? {
        return Err(LauncherError::config_invalid());
    }
    Ok(path.to_path_buf())
}
