use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Preview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LauncherConfig {
    pub schema_version: u32,
    pub install_dir: PathBuf,
    pub installed_version: Option<String>,
    pub bandwidth_limit_kib: Option<u64>,
    pub start_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledBuild {
    pub version: String,
    pub manifest_sha256: String,
    pub first_launch_confirmed: bool,
}
