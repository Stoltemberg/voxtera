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
    Busy,
    Cancelled,
    LauncherIncompatible,
    ReleaseUnavailable,
    RepairRequired,
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

    pub fn disk_space() -> Self {
        Self::new(
            LauncherErrorCode::DiskSpace,
            "Não há espaço livre suficiente para concluir a operação.",
        )
    }

    pub fn network() -> Self {
        Self::new(
            LauncherErrorCode::Network,
            "Não foi possível acessar o serviço de atualizações.",
        )
    }

    pub fn integrity() -> Self {
        Self::new(
            LauncherErrorCode::Integrity,
            "A instalação do jogo não passou na verificação de integridade.",
        )
    }

    pub fn unsafe_archive() -> Self {
        Self::new(
            LauncherErrorCode::UnsafeArchive,
            "O pacote de atualização contém dados inseguros.",
        )
    }

    pub fn game_running() -> Self {
        Self::new(
            LauncherErrorCode::GameRunning,
            "O jogo já está em execução.",
        )
    }

    pub fn launch_failed() -> Self {
        Self::new(
            LauncherErrorCode::LaunchFailed,
            "Não foi possível iniciar o jogo.",
        )
    }

    pub fn busy() -> Self {
        Self::new(
            LauncherErrorCode::Busy,
            "Outra operação do launcher já está em andamento.",
        )
    }

    pub fn cancelled() -> Self {
        Self::new(LauncherErrorCode::Cancelled, "A operação foi cancelada.")
    }

    pub fn launcher_incompatible() -> Self {
        Self::new(
            LauncherErrorCode::LauncherIncompatible,
            "Esta atualização exige uma versão mais nova do launcher.",
        )
    }

    pub fn release_unavailable() -> Self {
        Self::new(
            LauncherErrorCode::ReleaseUnavailable,
            "Nenhuma atualização compatível está disponível.",
        )
    }

    pub fn repair_required() -> Self {
        Self::new(
            LauncherErrorCode::RepairRequired,
            "A instalação precisa ser reparada antes de iniciar o jogo.",
        )
    }

    fn new(code: LauncherErrorCode, message: &str) -> Self {
        Self {
            code,
            message: message.to_owned(),
        }
    }
}
