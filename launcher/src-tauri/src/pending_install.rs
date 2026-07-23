#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingInstallCommit {
    pub transaction_id: String,
    pub version: String,
    pub manifest_sha256: String,
    pub install_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum PendingInstallError {
    #[error("IO failed: {0}")]
    Io(#[source] std::io::Error),
    #[error("corrupted")]
    Corrupted,
    #[error("mismatch")]
    Mismatch,
}

impl From<std::io::Error> for PendingInstallError {
    fn from(value: std::io::Error) -> Self {
        PendingInstallError::Io(value)
    }
}

impl From<PendingInstallError> for LauncherError {
    fn from(_: PendingInstallError) -> Self {
        LauncherError::integrity()
    }
}

pub fn pending_path(paths: &LauncherPaths) -> PathBuf {
    paths.root.join("pending-install.json")
}

pub fn write_pending_install(
    paths: &LauncherPaths,
    manifest: &Manifest,
    install_dir: &Path,
) -> Result<PendingInstallCommit, LauncherError> {
    let marker = PendingInstallCommit {
        transaction_id: Uuid::new_v4().to_string(),
        version: manifest.version.to_string(),
        manifest_sha256: manifest_sha256(manifest),
        install_dir: install_dir.to_path_buf(),
    };
    let bytes = serde_json::to_vec_pretty(&marker).map_err(|_| LauncherError::integrity())?;
    crate::service::write_atomic_bytes(&pending_path(paths), &bytes)?;
    Ok(marker)
}

pub fn clear_pending_install(paths: &LauncherPaths) -> Result<(), PendingInstallError> {
    let path = pending_path(paths);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn recover_pending_install(
    paths: &LauncherPaths,
    workflow: &dyn LauncherWorkflow,
) -> Result<(), LauncherError> {
    let path = pending_path(paths);
    if !path.exists() {
        return Ok(());
    }
    let bytes = fs::read(&path).map_err(PendingInstallError::from)?;
    let marker: PendingInstallCommit =
        serde_json::from_slice(&bytes).map_err(|_| PendingInstallError::Corrupted)?;
    let local = workflow.inspect_local(&LauncherConfig {
        schema_version: 1,
        install_dir: marker.install_dir.clone(),
        installed_version: Some(marker.version.clone()),
        bandwidth_limit_kib: None,
        start_minimized: false,
    });
    if local.valid
        && local.manifest.as_ref().map(manifest_sha256) == Some(marker.manifest_sha256.clone())
    {
        let _ = fs::remove_file(&path);
        workflow.confirm_first_launch()?;
    } else {
        let _ = workflow.rollback_pending();
        let _ = fs::remove_file(&path);
    }
    Ok(())
}
