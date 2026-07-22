use std::time::Duration;

use reqwest::{Client, redirect};
use semver::Version;
use serde::Deserialize;

const RELEASES_URL: &str = "https://api.github.com/repos/Stoltemberg/voxtera/releases";
const LAUNCHER_ASSET: &str = "VoxteraLauncher-setup.exe";
const GAME_ASSET: &str = "Voxtera-windows-x64.zip";
const MANIFEST_ASSET: &str = "voxtera-manifest.json";

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewRelease {
    pub version: Version,
    pub prerelease: bool,
    pub launcher_installer: ReleaseAsset,
    pub game_archive: ReleaseAsset,
    pub manifest: ReleaseAsset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReleaseErrorKind {
    Network,
    Contract,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct ReleaseError {
    kind: ReleaseErrorKind,
    message: String,
}

impl ReleaseError {
    pub fn code(&self) -> &'static str {
        match self.kind {
            ReleaseErrorKind::Network => "network",
            ReleaseErrorKind::Contract => "release_contract",
        }
    }

    fn network() -> Self {
        Self {
            kind: ReleaseErrorKind::Network,
            message: "Não foi possível consultar a release Preview.".to_owned(),
        }
    }

    fn contract(message: impl Into<String>) -> Self {
        Self {
            kind: ReleaseErrorKind::Contract,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseClient {
    client: Client,
}

impl ReleaseClient {
    pub fn new() -> Result<Self, ReleaseError> {
        let redirect_policy = redirect::Policy::custom(|attempt| {
            if attempt.url().scheme() != "https" {
                attempt.error("release redirect must use HTTPS")
            } else if attempt.previous().len() >= 10 {
                attempt.stop()
            } else {
                attempt.follow()
            }
        });
        let client = Client::builder()
            .https_only(true)
            .redirect(redirect_policy)
            .user_agent(concat!("VoxteraLauncher/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| ReleaseError::network())?;
        Ok(Self { client })
    }

    pub async fn latest_preview(&self) -> Result<PreviewRelease, ReleaseError> {
        let releases = self
            .client
            .get(RELEASES_URL)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|_| ReleaseError::network())?
            .json::<Vec<GitHubRelease>>()
            .await
            .map_err(|_| ReleaseError::network())?;
        select_release(&releases)
    }
}

impl Default for ReleaseClient {
    fn default() -> Self {
        Self::new().expect("the static Voxtera release HTTP client configuration must be valid")
    }
}

pub fn select_release(releases: &[GitHubRelease]) -> Result<PreviewRelease, ReleaseError> {
    let mut candidates = releases
        .iter()
        .filter(|release| !release.draft)
        .map(|release| parse_release_version(&release.tag_name).map(|version| (version, release)))
        .collect::<Result<Vec<_>, _>>()?;
    candidates.sort_unstable_by(|left, right| right.0.cmp(&left.0));
    let (version, release) = candidates
        .into_iter()
        .next()
        .ok_or_else(|| ReleaseError::contract("Nenhuma release Preview válida foi encontrada."))?;

    Ok(PreviewRelease {
        version,
        prerelease: release.prerelease,
        launcher_installer: exact_asset(release, LAUNCHER_ASSET)?,
        game_archive: exact_asset(release, GAME_ASSET)?,
        manifest: exact_asset(release, MANIFEST_ASSET)?,
    })
}

fn parse_release_version(tag: &str) -> Result<Version, ReleaseError> {
    let version = tag.strip_prefix('v').unwrap_or(tag);
    Version::parse(version)
        .map_err(|_| ReleaseError::contract("A release Preview possui uma versão inválida."))
}

fn exact_asset(release: &GitHubRelease, name: &str) -> Result<ReleaseAsset, ReleaseError> {
    let matches = release
        .assets
        .iter()
        .filter(|asset| asset.name == name)
        .collect::<Vec<_>>();
    let [asset] = matches.as_slice() else {
        return Err(ReleaseError::contract(format!(
            "A release Preview deve conter exatamente um asset {name}."
        )));
    };
    let url = reqwest::Url::parse(&asset.browser_download_url)
        .map_err(|_| ReleaseError::contract("A release Preview possui uma URL inválida."))?;
    if url.scheme() != "https" {
        return Err(ReleaseError::contract(
            "Os assets da release Preview devem usar HTTPS.",
        ));
    }

    Ok(ReleaseAsset {
        name: asset.name.clone(),
        size: asset.size,
        url: asset.browser_download_url.clone(),
    })
}
