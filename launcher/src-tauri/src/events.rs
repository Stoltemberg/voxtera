use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{DownloadProgress, LauncherOperation};

pub const LAUNCHER_PROGRESS_EVENT: &str = "launcher://progress";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LauncherProgressEvent {
    pub operation: LauncherOperation,
    pub stage: String,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub bytes_per_second: Option<f64>,
    pub eta_seconds: Option<f64>,
}

impl LauncherProgressEvent {
    pub fn stage(operation: LauncherOperation, stage: impl Into<String>) -> Self {
        Self {
            operation,
            stage: stage.into(),
            downloaded_bytes: None,
            total_bytes: None,
            bytes_per_second: None,
            eta_seconds: None,
        }
    }

    pub fn download(operation: LauncherOperation, progress: DownloadProgress) -> Self {
        Self {
            operation,
            stage: if progress.complete {
                "download_complete".to_owned()
            } else {
                "downloading".to_owned()
            },
            downloaded_bytes: Some(progress.downloaded_bytes),
            total_bytes: Some(progress.total_bytes),
            bytes_per_second: progress.bytes_per_second,
            eta_seconds: progress.eta_seconds,
        }
    }
}

pub trait ProgressSink: Send + Sync + 'static {
    fn emit(&self, event: LauncherProgressEvent);
}

#[derive(Debug, Default)]
pub struct NoopProgressSink;

impl ProgressSink for NoopProgressSink {
    fn emit(&self, _event: LauncherProgressEvent) {}
}

pub fn noop_progress_sink() -> Arc<dyn ProgressSink> { Arc::new(NoopProgressSink) }
