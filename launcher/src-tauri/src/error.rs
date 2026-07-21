use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LauncherErrorCode {
    ConfigInvalid,
    PermissionDenied,
    DiskSpace,
    Network,
    Integrity,
    UnsafeArchive,
    GameRunning,
    LaunchFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct LauncherError {
    pub code: LauncherErrorCode,
    pub message: String,
}

impl LauncherError {
    pub fn config_invalid() -> Self {
        Self::new(
            LauncherErrorCode::ConfigInvalid,
            "A configuração do launcher é inválida.",
        )
    }

    pub fn permission_denied() -> Self {
        Self::new(
            LauncherErrorCode::PermissionDenied,
            "O launcher não tem permissão para acessar o local necessário.",
        )
    }

    fn new(code: LauncherErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.to_owned(),
        }
    }
}
