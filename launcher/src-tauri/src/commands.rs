use std::{path::PathBuf, process::Command, sync::Arc};

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::{
    LAUNCHER_PROGRESS_EVENT, LauncherConfig, LauncherError, LauncherOperation, LauncherPaths,
    LauncherProgressEvent, LauncherService, LauncherSettingsInput, LauncherSnapshot,
    ProductionWorkflow, ProgressSink, RepairPlan, SystemProcessRunner, validate_install_dir,
};

pub type AppLauncherService = LauncherService<ProductionWorkflow<SystemProcessRunner>>;

pub struct AppState {
    pub service: Arc<AppLauncherService>,
    pub paths: LauncherPaths,
}

impl AppState {
    pub fn initialize(app: &AppHandle) -> Result<Self, LauncherError> {
        let paths = LauncherPaths::discover()?;
        let config = LauncherConfig::load_or_migrate(&paths)?;
        let progress: Arc<dyn ProgressSink> = Arc::new(TauriProgressSink::new(app.clone()));
        let workflow = ProductionWorkflow::new(paths.clone(), SystemProcessRunner, progress)?;
        let service = LauncherService::from_config(paths.clone(), config, workflow)?;
        Ok(Self {
            service: Arc::new(service),
            paths,
        })
    }
}

#[derive(Clone)]
struct TauriProgressSink {
    app: AppHandle,
}

impl TauriProgressSink {
    fn new(app: AppHandle) -> Self { Self { app } }
}

impl ProgressSink for TauriProgressSink {
    fn emit(&self, event: LauncherProgressEvent) {
        let _ = self.app.emit(LAUNCHER_PROGRESS_EVENT, event);
    }
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> LauncherSnapshot { state.service.get_snapshot() }

#[tauri::command]
pub async fn check_release(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LauncherSnapshot, LauncherError> {
    emit_stage(&app, LauncherOperation::CheckingRelease, "checking_release");
    let result = state.service.check_release().await;
    emit_stage(&app, LauncherOperation::CheckingRelease, "finished");
    result
}

#[tauri::command]
pub async fn install_or_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LauncherSnapshot, LauncherError> {
    emit_stage(&app, LauncherOperation::Installing, "starting");
    let result = state.service.install_or_update().await;
    emit_stage(&app, LauncherOperation::Installing, "finished");
    result
}

#[tauri::command]
pub fn cancel_operation(state: State<'_, AppState>) -> Result<(), LauncherError> {
    state.service.cancel_operation()
}

#[tauri::command]
pub fn scan_repair(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<RepairPlan, LauncherError> {
    emit_stage(&app, LauncherOperation::ScanningRepair, "scanning");
    let result = state.service.scan_repair();
    emit_stage(&app, LauncherOperation::ScanningRepair, "finished");
    result
}

#[tauri::command]
pub async fn repair(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LauncherSnapshot, LauncherError> {
    emit_stage(&app, LauncherOperation::Repairing, "starting");
    let result = state.service.repair().await;
    emit_stage(&app, LauncherOperation::Repairing, "finished");
    result
}

#[tauri::command]
pub fn choose_install_dir(app: AppHandle) -> Result<Option<PathBuf>, LauncherError> {
    let Some(selected) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    match selected {
        FilePath::Path(path) => validate_install_dir(&path).map(Some),
        FilePath::Url(_) => Err(LauncherError::config_invalid()),
    }
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    settings: LauncherSettingsInput,
) -> Result<LauncherSnapshot, LauncherError> {
    state.service.save_settings(settings)
}

#[tauri::command]
pub fn launch_game(state: State<'_, AppState>) -> Result<u32, LauncherError> {
    state.service.launch_game()
}

#[tauri::command]
pub fn open_logs(state: State<'_, AppState>) -> Result<(), LauncherError> {
    std::fs::create_dir_all(&state.paths.logs_dir)
        .map_err(|_| LauncherError::permission_denied())?;
    {
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_owned());
        let explorer = PathBuf::from(system_root).join("system32").join("explorer.exe");
        Command::new(explorer)
            .arg(&state.paths.logs_dir)
            .spawn()
    }
        .map(|_| ())
        .map_err(|_| LauncherError::launch_failed())
}

fn emit_stage(app: &AppHandle, operation: LauncherOperation, stage: &str) {
    let _ = app.emit(
        LAUNCHER_PROGRESS_EVENT,
        LauncherProgressEvent::stage(operation, stage),
    );
}

pub fn register(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::initialize(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            check_release,
            install_or_update,
            cancel_operation,
            scan_repair,
            repair,
            choose_install_dir,
            save_settings,
            launch_game,
            open_logs,
        ])
}
