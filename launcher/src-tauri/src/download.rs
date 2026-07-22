use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use reqwest::{Client, StatusCode, header};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File, OpenOptions},
    io::AsyncWriteExt,
    sync::watch,
};

use crate::{DownloadProgress, ProgressThrottle, ReleaseAsset};

const MAX_WRITE_CHUNK: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub asset: ReleaseAsset,
    pub expected_sha256: String,
    pub cache_path: PathBuf,
    pub bandwidth_limit_kib: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadOutcome {
    pub bytes_written: u64,
    pub part_path: PathBuf,
    pub metadata_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("download cancelled")]
    Cancelled,
    #[error("download network error")]
    Network(#[source] reqwest::Error),
    #[error("download protocol error: {0}")]
    Protocol(String),
    #[error("download cache error")]
    Cache(#[source] std::io::Error),
}

impl DownloadError {
    pub fn is_cancelled(&self) -> bool { matches!(self, Self::Cancelled) }

    pub fn is_network(&self) -> bool { matches!(self, Self::Network(_)) }
}

#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<watch::Sender<bool>>,
}

impl CancellationToken {
    pub fn new() -> Self {
        let (sender, _) = watch::channel(false);
        Self {
            cancelled: Arc::new(sender),
        }
    }

    pub fn cancel(&self) { self.cancelled.send_replace(true); }

    pub fn is_cancelled(&self) -> bool { *self.cancelled.borrow() }

    async fn cancelled(&self) {
        let mut receiver = self.cancelled.subscribe();
        if *receiver.borrow() {
            return;
        }
        let _ = receiver.changed().await;
    }
}

impl Default for CancellationToken {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone)]
pub struct DownloadManager {
    client: Client,
    cancellation: CancellationToken,
}

impl DownloadManager {
    pub fn new(cancellation: CancellationToken) -> Result<Self, DownloadError> {
        let client = Client::builder()
            .user_agent(concat!("VoxteraLauncher/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(DownloadError::Network)?;
        Ok(Self {
            client,
            cancellation,
        })
    }

    pub async fn download<S>(
        &self,
        request: DownloadRequest,
        mut sink: S,
    ) -> Result<DownloadOutcome, DownloadError>
    where
        S: FnMut(DownloadProgress) + Send,
    {
        if self.cancellation.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }
        if let Some(parent) = request.cache_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(DownloadError::Cache)?;
        }

        let part_path = sibling_with_suffix(&request.cache_path, ".part");
        let metadata_path = sibling_with_suffix(&request.cache_path, ".part.json");
        let resume = load_resume(&request, &part_path, &metadata_path).await;
        if resume.is_none() {
            remove_if_present(&metadata_path).await?;
        }

        let offset = resume.as_ref().map_or(0, |resume| resume.bytes_written);
        if resume.is_some() && offset == request.asset.size {
            let file = OpenOptions::new()
                .write(true)
                .open(&part_path)
                .await
                .map_err(DownloadError::Cache)?;
            file.sync_all().await.map_err(DownloadError::Cache)?;
            let mut progress = ProgressThrottle::new(request.asset.size, Instant::now());
            if let Some(event) = progress.observe(offset, Instant::now(), true) {
                sink(event);
            }
            return Ok(DownloadOutcome {
                bytes_written: offset,
                part_path,
                metadata_path,
            });
        }

        let (response, append, etag) = self.validated_response(&request, resume.as_ref()).await?;
        let initial_bytes = if append { offset } else { 0 };
        let metadata = ResumeMetadata {
            url: request.asset.url.clone(),
            etag,
            expected_sha256: request.expected_sha256.clone(),
            expected_size: request.asset.size,
        };
        let mut file = open_part(&part_path, append).await?;
        persist_metadata(&metadata_path, &metadata).await?;
        let started_at = Instant::now();
        let mut progress = ProgressThrottle::new(request.asset.size, started_at);
        if let Some(event) = progress.observe(initial_bytes, started_at, false) {
            sink(event);
        }
        let mut limiter = BandwidthLimiter::new(request.bandwidth_limit_kib, started_at);
        let mut bytes_written = initial_bytes;
        let mut stream = response.bytes_stream();

        loop {
            let next = tokio::select! {
                _ = self.cancellation.cancelled() => {
                    file.flush().await.map_err(DownloadError::Cache)?;
                    return Err(DownloadError::Cancelled);
                }
                next = stream.next() => next,
            };
            let Some(chunk) = next else {
                break;
            };
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    file.flush().await.map_err(DownloadError::Cache)?;
                    return Err(DownloadError::Network(error));
                },
            };

            for slice in chunk.chunks(MAX_WRITE_CHUNK) {
                limiter.wait(slice.len() as u64, &self.cancellation).await?;
                file.write_all(slice).await.map_err(DownloadError::Cache)?;
                bytes_written = bytes_written.saturating_add(slice.len() as u64);
                if bytes_written > request.asset.size {
                    file.flush().await.map_err(DownloadError::Cache)?;
                    return Err(DownloadError::Protocol(
                        "response exceeded the declared asset size".to_owned(),
                    ));
                }
                if let Some(event) = progress.observe(bytes_written, Instant::now(), false) {
                    sink(event);
                }
            }
        }

        if bytes_written != request.asset.size {
            file.flush().await.map_err(DownloadError::Cache)?;
            return Err(DownloadError::Protocol(format!(
                "response ended at {bytes_written} of {} bytes",
                request.asset.size
            )));
        }
        file.flush().await.map_err(DownloadError::Cache)?;
        file.sync_all().await.map_err(DownloadError::Cache)?;
        persist_metadata(&metadata_path, &metadata).await?;
        if let Some(event) = progress.observe(bytes_written, Instant::now(), true) {
            sink(event);
        }

        Ok(DownloadOutcome {
            bytes_written,
            part_path,
            metadata_path,
        })
    }

    async fn validated_response(
        &self,
        request: &DownloadRequest,
        resume: Option<&ResumeState>,
    ) -> Result<(reqwest::Response, bool, Option<String>), DownloadError> {
        if let Some(resume) = resume {
            let response = self
                .send(
                    &request.asset.url,
                    Some((resume.bytes_written, &resume.etag)),
                )
                .await?;
            if valid_resume_response(&response, resume, request.asset.size) {
                let etag = response_etag(&response);
                return Ok((response, true, etag));
            }
        }

        let response = self.send(&request.asset.url, None).await?;
        validate_fresh_response(&response, request.asset.size)?;
        let etag = response_etag(&response);
        Ok((response, false, etag))
    }

    async fn send(
        &self,
        url: &str,
        resume: Option<(u64, &str)>,
    ) -> Result<reqwest::Response, DownloadError> {
        let mut request = self.client.get(url);
        if let Some((offset, etag)) = resume {
            request = request
                .header(header::RANGE, format!("bytes={offset}-"))
                .header(header::IF_RANGE, etag);
        }
        tokio::select! {
            _ = self.cancellation.cancelled() => Err(DownloadError::Cancelled),
            response = request.send() => response.map_err(DownloadError::Network),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResumeMetadata {
    url: String,
    etag: Option<String>,
    expected_sha256: String,
    expected_size: u64,
}

#[derive(Debug)]
struct ResumeState {
    bytes_written: u64,
    etag: String,
}

async fn load_resume(
    request: &DownloadRequest,
    part_path: &Path,
    metadata_path: &Path,
) -> Option<ResumeState> {
    let metadata = fs::read(metadata_path).await.ok()?;
    let metadata: ResumeMetadata = serde_json::from_slice(&metadata).ok()?;
    let etag = metadata.etag?;
    let bytes_written = fs::metadata(part_path).await.ok()?.len();
    (metadata.url == request.asset.url
        && metadata.expected_sha256 == request.expected_sha256
        && metadata.expected_size == request.asset.size
        && bytes_written > 0
        && bytes_written <= request.asset.size)
        .then_some(ResumeState {
            bytes_written,
            etag,
        })
}

fn valid_resume_response(
    response: &reqwest::Response,
    resume: &ResumeState,
    expected_size: u64,
) -> bool {
    if response.status() != StatusCode::PARTIAL_CONTENT
        || response_etag(response).as_deref() != Some(&resume.etag)
        || response.content_length() != Some(expected_size - resume.bytes_written)
    {
        return false;
    }
    response
        .headers()
        .get(header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                == format!(
                    "bytes {}-{}/{}",
                    resume.bytes_written,
                    expected_size - 1,
                    expected_size
                )
        })
}

fn validate_fresh_response(
    response: &reqwest::Response,
    expected_size: u64,
) -> Result<(), DownloadError> {
    if response.status() != StatusCode::OK {
        return Err(DownloadError::Protocol(format!(
            "expected HTTP 200, got {}",
            response.status()
        )));
    }
    if response
        .content_length()
        .is_some_and(|length| length != expected_size)
    {
        return Err(DownloadError::Protocol(
            "content length does not match the declared asset size".to_owned(),
        ));
    }
    Ok(())
}

fn response_etag(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

async fn open_part(path: &Path, append: bool) -> Result<File, DownloadError> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .await
        .map_err(DownloadError::Cache)
}

async fn persist_metadata(path: &Path, metadata: &ResumeMetadata) -> Result<(), DownloadError> {
    let bytes =
        serde_json::to_vec(metadata).map_err(|error| DownloadError::Protocol(error.to_string()))?;
    let mut file = File::create(path).await.map_err(DownloadError::Cache)?;
    file.write_all(&bytes).await.map_err(DownloadError::Cache)?;
    file.flush().await.map_err(DownloadError::Cache)?;
    file.sync_all().await.map_err(DownloadError::Cache)
}

async fn remove_if_present(path: &Path) -> Result<(), DownloadError> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DownloadError::Cache(error)),
    }
}

fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

#[derive(Debug)]
struct BandwidthLimiter {
    bytes_per_second: Option<u64>,
    started_at: Instant,
    bytes_scheduled: u64,
}

impl BandwidthLimiter {
    fn new(limit_kib: Option<u64>, started_at: Instant) -> Self {
        Self {
            bytes_per_second: limit_kib
                .and_then(|limit| limit.checked_mul(1024))
                .filter(|v| *v > 0),
            started_at,
            bytes_scheduled: 0,
        }
    }

    async fn wait(
        &mut self,
        bytes: u64,
        cancellation: &CancellationToken,
    ) -> Result<(), DownloadError> {
        let Some(bytes_per_second) = self.bytes_per_second else {
            return Ok(());
        };
        self.bytes_scheduled = self.bytes_scheduled.saturating_add(bytes);
        let target = Duration::from_secs_f64(self.bytes_scheduled as f64 / bytes_per_second as f64);
        let elapsed = self.started_at.elapsed();
        if let Some(delay) = target.checked_sub(elapsed) {
            tokio::select! {
                _ = cancellation.cancelled() => return Err(DownloadError::Cancelled),
                _ = tokio::time::sleep(delay) => {}
            }
        }
        Ok(())
    }
}
