use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use launcher_core::{
    ArchiveMetadata, CancellationToken, Channel, InstallOutcome, LauncherConfig, LauncherError,
    LauncherErrorCode, LauncherOperation, LauncherPaths, LauncherPhase, LauncherService,
    LauncherSettingsInput, LauncherWorkflow, LocalBuild, ManagedFile, Manifest, ReleaseCheck,
    RepairPlan, ServiceFuture,
};
use semver::Version;

#[derive(Default)]
struct FakeState {
    recoveries: usize,
    block_install: bool,
    fail_install: bool,
    network_failure: bool,
    launches: usize,
    confirmations: usize,
    pending_promotion: bool,
}

#[derive(Clone)]
struct FakeWorkflow {
    state: Arc<Mutex<FakeState>>,
    local: LocalBuild,
}

impl FakeWorkflow {
    fn valid(version: &str) -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState::default())),
            local: LocalBuild::valid(manifest(version)),
        }
    }
}

impl LauncherWorkflow for FakeWorkflow {
    fn recover(&self, _config: &LauncherConfig) -> Result<(), LauncherError> {
        self.state.lock().unwrap().recoveries += 1;
        Ok(())
    }

    fn inspect_local(&self, _config: &LauncherConfig) -> LocalBuild { self.local.clone() }

    fn check_release(&self, cancellation: CancellationToken) -> ServiceFuture<'_, ReleaseCheck> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(LauncherError::cancelled());
            }
            if state.lock().unwrap().network_failure {
                Err(LauncherError::network())
            } else {
                Ok(ReleaseCheck::new(manifest("0.4.0-preview.1")))
            }
        })
    }

    fn install_or_update(
        &self,
        _config: &LauncherConfig,
        release: ReleaseCheck,
        cancellation: CancellationToken,
    ) -> ServiceFuture<'_, InstallOutcome> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            loop {
                let (block, fail) = {
                    let state = state.lock().unwrap();
                    (state.block_install, state.fail_install)
                };
                if cancellation.is_cancelled() {
                    return Err(LauncherError::cancelled());
                }
                if !block {
                    return if fail {
                        Err(LauncherError::integrity())
                    } else {
                        Ok(InstallOutcome::new(release.manifest))
                    };
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
    }

    fn scan_repair(
        &self,
        _config: &LauncherConfig,
        _manifest: &Manifest,
    ) -> Result<RepairPlan, LauncherError> {
        Ok(RepairPlan {
            checked_files: 1,
            checked_bytes: 4,
            invalid_files: Vec::new(),
            invalid_bytes: 0,
            deletions: Vec::new(),
        })
    }

    fn repair(
        &self,
        _config: &LauncherConfig,
        _manifest: Manifest,
        _plan: RepairPlan,
        _cancellation: CancellationToken,
    ) -> ServiceFuture<'_, InstallOutcome> {
        Box::pin(async { Err(LauncherError::integrity()) })
    }

    fn confirm_first_launch(&self) -> Result<(), LauncherError> { Ok(()) }

    fn rollback_pending(&self) -> Result<(), LauncherError> { Ok(()) }

    fn launch_game(
        &self,
        _config: &LauncherConfig,
        _manifest: &Manifest,
    ) -> Result<u32, LauncherError> {
        let mut state = self.state.lock().unwrap();
        state.launches += 1;
        state.confirmations += 1;
        Ok(77)
    }

    fn has_pending_promotion(&self) -> bool { self.state.lock().unwrap().pending_promotion }
}

fn paths(base: &Path) -> LauncherPaths {
    let root = base.join("Voxtera");
    LauncherPaths {
        config_file: root.join("launcher.json"),
        logs_dir: root.join("logs"),
        cache_dir: root.join("cache"),
        default_install_dir: root.join("game"),
        legacy_config: root.join("legacy.json"),
        root,
    }
}

fn config(paths: &LauncherPaths, version: Option<&str>) -> LauncherConfig {
    LauncherConfig {
        schema_version: 1,
        install_dir: paths.default_install_dir.clone(),
        installed_version: version.map(str::to_owned),
        bandwidth_limit_kib: None,
        start_minimized: false,
    }
}

fn manifest(version: &str) -> Manifest {
    Manifest {
        schema_version: 1,
        version: Version::parse(version).unwrap(),
        channel: Channel::Preview,
        archive: ArchiveMetadata {
            name: "Voxtera-windows-x64.zip".to_owned(),
            size: 1,
            sha256: "0".repeat(64),
        },
        executable: "Voxtera.exe".to_owned(),
        files: vec![ManagedFile {
            path: "Voxtera.exe".to_owned(),
            size: 4,
            sha256: "0".repeat(64),
        }],
        preserved_paths: vec![
            "userdata/".to_owned(),
            "screenshots/".to_owned(),
            "settings/".to_owned(),
        ],
        minimum_launcher_version: Version::parse("0.1.0").unwrap(),
    }
}

#[tokio::test]
async fn release_network_failure_becomes_offline_when_a_valid_local_build_exists() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let workflow = FakeWorkflow::valid("0.3.0-preview.1");
    workflow.state.lock().unwrap().network_failure = true;
    let service = LauncherService::from_config(
        paths.clone(),
        config(&paths, Some("0.3.0-preview.1")),
        workflow,
    )
    .unwrap();

    let snapshot = service.check_release().await.unwrap();

    assert_eq!(snapshot.phase, LauncherPhase::Offline);
    assert!(snapshot.local_build_valid);
    assert_eq!(
        snapshot.installed_version.as_deref(),
        Some("0.3.0-preview.1")
    );
}

#[tokio::test]
async fn incompatible_launcher_version_is_an_actionable_stable_error() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let workflow = FakeWorkflow::valid("0.3.0-preview.1");
    let service = LauncherService::from_config(
        paths.clone(),
        config(&paths, Some("0.3.0-preview.1")),
        workflow,
    )
    .unwrap();
    let mut incompatible = manifest("0.4.0-preview.1");
    incompatible.minimum_launcher_version = Version::parse("99.0.0").unwrap();

    let error = service
        .accept_release(ReleaseCheck::new(incompatible))
        .unwrap_err();

    assert_eq!(error.code, LauncherErrorCode::LauncherIncompatible);
    assert_eq!(
        error.message,
        "Esta atualização exige uma versão mais nova do launcher."
    );
}

#[tokio::test]
async fn mutable_operations_are_exclusive_and_cooperatively_cancelled() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let workflow = FakeWorkflow::valid("0.3.0-preview.1");
    workflow.state.lock().unwrap().block_install = true;
    let service = Arc::new(
        LauncherService::from_config(
            paths.clone(),
            config(&paths, Some("0.3.0-preview.1")),
            workflow,
        )
        .unwrap(),
    );
    service
        .accept_release(ReleaseCheck::new(manifest("0.4.0-preview.1")))
        .unwrap();
    let installing = {
        let service = Arc::clone(&service);
        tokio::spawn(async move { service.install_or_update().await })
    };
    tokio::time::sleep(Duration::from_millis(30)).await;

    let busy = service.scan_repair().unwrap_err();
    assert_eq!(busy.code, LauncherErrorCode::Busy);
    assert_eq!(
        service.get_snapshot().operation,
        Some(LauncherOperation::Installing)
    );

    service.cancel_operation().unwrap();
    let cancelled = installing.await.unwrap().unwrap_err();
    assert_eq!(cancelled.code, LauncherErrorCode::Cancelled);
    assert_eq!(service.get_snapshot().operation, None);
}

#[tokio::test]
async fn failed_update_preserves_the_installed_version_on_disk_and_in_memory() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let existing = config(&paths, Some("0.3.0-preview.1"));
    existing.save_atomic(&paths).unwrap();
    let workflow = FakeWorkflow::valid("0.3.0-preview.1");
    workflow.state.lock().unwrap().fail_install = true;
    let service = LauncherService::from_config(paths.clone(), existing, workflow).unwrap();
    service
        .accept_release(ReleaseCheck::new(manifest("0.4.0-preview.1")))
        .unwrap();

    service.install_or_update().await.unwrap_err();

    assert_eq!(
        service.get_snapshot().installed_version.as_deref(),
        Some("0.3.0-preview.1")
    );
    let saved: LauncherConfig =
        serde_json::from_slice(&std::fs::read(paths.config_file).unwrap()).unwrap();
    assert_eq!(saved.installed_version.as_deref(), Some("0.3.0-preview.1"));
}

#[test]
fn startup_recovers_the_install_journal_before_inspecting_local_state() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let workflow = FakeWorkflow::valid("0.3.0-preview.1");
    let state = Arc::clone(&workflow.state);

    LauncherService::from_config(
        paths.clone(),
        config(&paths, Some("0.3.0-preview.1")),
        workflow,
    )
    .unwrap();

    assert_eq!(state.lock().unwrap().recoveries, 1);
}

#[test]
fn capabilities_do_not_grant_generic_process_filesystem_or_http_access() {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
    let permissions = value["permissions"].as_array().unwrap();
    let forbidden = [
        "shell:allow-execute",
        "fs:default",
        "http:default",
        "http:allow-fetch",
    ];

    for permission in permissions {
        let permission = permission.as_str().unwrap_or_default();
        assert!(!forbidden.contains(&permission));
        assert!(!permission.starts_with("shell:"));
        assert!(!permission.starts_with("fs:"));
        assert!(!permission.starts_with("http:"));
    }
}

#[test]
fn user_facing_errors_are_serializable_and_do_not_leak_internal_details() {
    let error = LauncherError::busy();
    let json = serde_json::to_string(&error).unwrap();

    assert_eq!(
        error.message,
        "Outra operação do launcher já está em andamento."
    );
    assert!(json.contains("busy"));
    assert!(!json.contains("backtrace"));
}

#[test]
fn service_launches_only_a_valid_local_build_and_confirms_after_spawn() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let workflow = FakeWorkflow::valid("0.3.0-preview.1");
    let state = Arc::clone(&workflow.state);
    let service = LauncherService::from_config(
        paths.clone(),
        config(&paths, Some("0.3.0-preview.1")),
        workflow,
    )
    .unwrap();

    assert_eq!(service.launch_game().unwrap(), 77);
    assert_eq!(state.lock().unwrap().launches, 1);
    assert_eq!(state.lock().unwrap().confirmations, 1);

    let invalid_workflow = FakeWorkflow {
        state: Arc::new(Mutex::new(FakeState::default())),
        local: LocalBuild::invalid(),
    };
    let invalid = LauncherService::from_config(
        paths.clone(),
        config(&paths, Some("0.3.0-preview.1")),
        invalid_workflow,
    )
    .unwrap();
    assert_eq!(
        invalid.launch_game().unwrap_err().code,
        LauncherErrorCode::RepairRequired
    );
}

#[test]
fn settings_accept_only_an_absolute_validated_install_directory() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let service = LauncherService::from_config(
        paths.clone(),
        config(&paths, Some("0.3.0-preview.1")),
        FakeWorkflow::valid("0.3.0-preview.1"),
    )
    .unwrap();

    let error = service
        .save_settings(LauncherSettingsInput {
            install_dir: PathBuf::from("relative/game"),
            bandwidth_limit_kib: None,
            start_minimized: false,
        })
        .unwrap_err();
    assert_eq!(error.code, LauncherErrorCode::ConfigInvalid);

    let selected = temp.path().join("selected-game");
    std::fs::create_dir(&selected).unwrap();
    let snapshot = service
        .save_settings(LauncherSettingsInput {
            install_dir: selected.clone(),
            bandwidth_limit_kib: Some(2048),
            start_minimized: true,
        })
        .unwrap();
    assert_eq!(snapshot.install_dir, selected);
}

#[test]
fn invalid_local_files_keep_the_trusted_manifest_available_for_repair_scan() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let workflow = FakeWorkflow {
        state: Arc::new(Mutex::new(FakeState::default())),
        local: LocalBuild::invalid_with_manifest(manifest("0.3.0-preview.1")),
    };
    let service = LauncherService::from_config(
        paths.clone(),
        config(&paths, Some("0.3.0-preview.1")),
        workflow,
    )
    .unwrap();

    assert!(service.scan_repair().is_ok());
}

#[test]
fn install_directory_cannot_change_while_rollback_confirmation_is_pending() {
    let temp = tempfile::tempdir().unwrap();
    let paths = paths(temp.path());
    let workflow = FakeWorkflow::valid("0.3.0-preview.1");
    workflow.state.lock().unwrap().pending_promotion = true;
    let service = LauncherService::from_config(
        paths.clone(),
        config(&paths, Some("0.3.0-preview.1")),
        workflow,
    )
    .unwrap();
    let selected = temp.path().join("other-game");
    std::fs::create_dir(&selected).unwrap();

    let error = service
        .save_settings(LauncherSettingsInput {
            install_dir: selected,
            bandwidth_limit_kib: None,
            start_minimized: false,
        })
        .unwrap_err();

    assert_eq!(error.code, LauncherErrorCode::Busy);
    assert_eq!(
        service.get_snapshot().install_dir,
        paths.default_install_dir
    );
}
