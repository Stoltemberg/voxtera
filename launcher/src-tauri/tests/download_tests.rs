mod support;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use launcher_core::{
    CancellationToken, DownloadManager, DownloadProgress, DownloadRequest, ProgressThrottle,
    ReleaseAsset,
};
use support::{RangeServer, ServerMode};
use tempfile::TempDir;

const TEST_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn test_bytes() -> Vec<u8> { (0..32 * 1024).map(|index| (index % 251) as u8).collect() }

struct DownloadFixture {
    _temp: TempDir,
    cache_path: PathBuf,
    manager: DownloadManager,
    cancellation: CancellationToken,
    events: Arc<Mutex<Vec<DownloadProgress>>>,
}

impl DownloadFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let cache_path = temp.path().join("Voxtera-windows-x64.zip");
        let cancellation = CancellationToken::new();
        Self {
            _temp: temp,
            cache_path,
            manager: DownloadManager::new(cancellation.clone()).unwrap(),
            cancellation,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_partial(bytes: &[u8], etag: &str, url: &str, full_size: u64) -> Self {
        let fixture = Self::new();
        fs::write(fixture.part_path(), bytes).unwrap();
        fixture.write_metadata(url, etag, full_size, TEST_SHA256);
        fixture
    }

    fn request(&self, url: String, size: u64) -> DownloadRequest {
        DownloadRequest {
            asset: ReleaseAsset {
                name: "Voxtera-windows-x64.zip".to_owned(),
                size,
                url,
            },
            expected_sha256: TEST_SHA256.to_owned(),
            cache_path: self.cache_path.clone(),
            bandwidth_limit_kib: None,
        }
    }

    fn sink(&self) -> impl FnMut(DownloadProgress) + Send + 'static {
        let events = self.events.clone();
        move |event| events.lock().unwrap().push(event)
    }

    fn part_path(&self) -> PathBuf { PathBuf::from(format!("{}.part", self.cache_path.display())) }

    fn metadata_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.part.json", self.cache_path.display()))
    }

    fn write_metadata(&self, url: &str, etag: &str, size: u64, sha256: &str) {
        let metadata = serde_json::json!({
            "url": url,
            "etag": etag,
            "expected_sha256": sha256,
            "expected_size": size,
        });
        fs::write(self.metadata_path(), serde_json::to_vec(&metadata).unwrap()).unwrap();
    }
}

#[tokio::test]
async fn downloads_fresh_asset_to_a_part_file() {
    let bytes = test_bytes();
    let server = RangeServer::new(&bytes, "etag-v1").await;
    let fixture = DownloadFixture::new();

    let result = fixture
        .manager
        .download(
            fixture.request(server.url(), bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap();

    assert_eq!(result.bytes_written, bytes.len() as u64);
    assert_eq!(result.part_path, fixture.part_path());
    assert_eq!(fs::read(fixture.part_path()).unwrap(), bytes);
    assert!(fixture.metadata_path().is_file());
    assert_eq!(server.last_range(), None);
    assert!(fixture.events.lock().unwrap().last().unwrap().complete);
}

#[tokio::test]
async fn resumes_matching_partial_download() {
    let bytes = test_bytes();
    let server = RangeServer::new(&bytes, "etag-v1").await;
    let fixture =
        DownloadFixture::with_partial(&bytes[..4096], "etag-v1", &server.url(), bytes.len() as u64);

    let result = fixture
        .manager
        .download(
            fixture.request(server.url(), bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap();

    assert_eq!(result.bytes_written, bytes.len() as u64);
    assert_eq!(server.last_range(), Some("bytes=4096-".to_owned()));
    assert_eq!(server.first_if_range(), Some("etag-v1".to_owned()));
    assert_eq!(fs::read(fixture.part_path()).unwrap(), bytes);
}

#[tokio::test]
async fn restarts_when_server_ignores_the_range() {
    let bytes = test_bytes();
    let server = RangeServer::with_mode(&bytes, "etag-v1", ServerMode::IgnoreRange).await;
    let fixture =
        DownloadFixture::with_partial(&bytes[..4096], "etag-v1", &server.url(), bytes.len() as u64);

    fixture
        .manager
        .download(
            fixture.request(server.url(), bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap();

    assert_eq!(server.first_range(), Some("bytes=4096-".to_owned()));
    assert_eq!(fs::read(fixture.part_path()).unwrap(), bytes);
}

#[tokio::test]
async fn restarts_with_a_fresh_request_when_etag_changes() {
    let bytes = test_bytes();
    let server = RangeServer::with_mode(&bytes, "etag-v1", ServerMode::ChangedEtag).await;
    let fixture =
        DownloadFixture::with_partial(&bytes[..4096], "etag-v1", &server.url(), bytes.len() as u64);

    fixture
        .manager
        .download(
            fixture.request(server.url(), bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap();

    assert_eq!(server.first_range(), Some("bytes=4096-".to_owned()));
    assert_eq!(server.last_range(), None);
    assert_eq!(server.request_count(), 2);
    assert_eq!(fs::read(fixture.part_path()).unwrap(), bytes);
}

#[tokio::test]
async fn cancellation_interrupts_a_slow_download_and_preserves_partial_state() {
    let bytes = test_bytes();
    let server = RangeServer::with_mode(&bytes, "etag-v1", ServerMode::Slow).await;
    let fixture = DownloadFixture::new();
    let cancellation = fixture.cancellation.clone();
    let cancel_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(180)).await;
        cancellation.cancel();
    });

    let error = fixture
        .manager
        .download(
            fixture.request(server.url(), bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap_err();
    cancel_task.await.unwrap();

    assert!(error.is_cancelled());
    assert!(fixture.part_path().metadata().unwrap().len() < bytes.len() as u64);
    assert!(fixture.metadata_path().is_file());
}

#[tokio::test]
async fn server_disconnect_returns_an_error_without_discarding_partial_state() {
    let bytes = test_bytes();
    let server = RangeServer::with_mode(&bytes, "etag-v1", ServerMode::Disconnect).await;
    let fixture = DownloadFixture::new();

    let error = fixture
        .manager
        .download(
            fixture.request(server.url(), bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap_err();

    assert!(error.is_network());
    assert!(fixture.part_path().metadata().unwrap().len() < bytes.len() as u64);
    assert!(fixture.metadata_path().is_file());
}

#[tokio::test]
async fn accepts_empty_download_with_zero_content_length() {
    let server = RangeServer::new(&[], "etag-empty").await;
    let fixture = DownloadFixture::new();

    let result = fixture
        .manager
        .download(fixture.request(server.url(), 0), fixture.sink())
        .await
        .unwrap();

    assert_eq!(result.bytes_written, 0);
    assert_eq!(fs::read(fixture.part_path()).unwrap(), Vec::<u8>::new());
}

#[tokio::test]
async fn accepts_stream_without_content_length() {
    let bytes = test_bytes();
    let server = RangeServer::with_mode(&bytes, "etag-v1", ServerMode::UnknownLength).await;
    let fixture = DownloadFixture::new();

    let result = fixture
        .manager
        .download(
            fixture.request(server.url(), bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap();

    assert_eq!(result.bytes_written, bytes.len() as u64);
    assert_eq!(fs::read(fixture.part_path()).unwrap(), bytes);
}

#[tokio::test]
async fn stale_resume_metadata_is_discarded_without_sending_range() {
    let bytes = test_bytes();
    let server = RangeServer::new(&bytes, "etag-v1").await;
    let fixture = DownloadFixture::with_partial(
        &bytes[..4096],
        "etag-old",
        "http://stale.invalid/asset",
        bytes.len() as u64,
    );

    fixture
        .manager
        .download(
            fixture.request(server.url(), bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap();

    assert_eq!(server.last_range(), None);
    assert_eq!(fs::read(fixture.part_path()).unwrap(), bytes);
}

#[tokio::test]
async fn metadata_rewrite_is_valid_and_removes_the_atomic_temporary_file() {
    let first_bytes = test_bytes();
    let first_server = RangeServer::new(&first_bytes, "etag-v1").await;
    let fixture = DownloadFixture::new();
    fixture
        .manager
        .download(
            fixture.request(first_server.url(), first_bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap();

    let temporary_path = PathBuf::from(format!("{}.tmp", fixture.metadata_path().display()));
    fs::write(&temporary_path, b"interrupted metadata write").unwrap();
    let second_bytes = vec![42_u8; first_bytes.len()];
    let second_server = RangeServer::new(&second_bytes, "etag-v2").await;
    let second_url = second_server.url();

    fixture
        .manager
        .download(
            fixture.request(second_url.clone(), second_bytes.len() as u64),
            fixture.sink(),
        )
        .await
        .unwrap();

    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.metadata_path()).unwrap()).unwrap();
    assert_eq!(metadata["url"], second_url);
    assert_eq!(metadata["etag"], "etag-v2");
    assert_eq!(metadata["expected_size"], second_bytes.len() as u64);
    assert!(!temporary_path.exists());
    assert_eq!(fs::read(fixture.part_path()).unwrap(), second_bytes);
}

#[test]
fn progress_is_immediate_throttled_to_four_hz_and_forced_at_completion() {
    let started = Instant::now();
    let mut throttle = ProgressThrottle::new(1_000, started);

    assert!(throttle.observe(0, started, false).is_some());
    assert!(
        throttle
            .observe(100, started + Duration::from_millis(249), false)
            .is_none()
    );
    let periodic = throttle
        .observe(200, started + Duration::from_millis(250), false)
        .unwrap();
    assert!(periodic.bytes_per_second.is_some());
    assert!(periodic.eta_seconds.is_some());
    assert!(
        throttle
            .observe(300, started + Duration::from_millis(300), false)
            .is_none()
    );
    let complete = throttle
        .observe(1_000, started + Duration::from_millis(301), true)
        .unwrap();
    assert!(complete.complete);
    assert!(
        throttle
            .observe(1_000, started + Duration::from_millis(600), true)
            .is_none()
    );
}

#[tokio::test]
async fn bandwidth_limit_delays_streaming_without_buffering_the_archive() {
    let bytes = vec![7_u8; 8 * 1024];
    let server = RangeServer::new(&bytes, "etag-v1").await;
    let fixture = DownloadFixture::new();
    let mut request = fixture.request(server.url(), bytes.len() as u64);
    request.bandwidth_limit_kib = Some(64);
    let started = Instant::now();

    fixture
        .manager
        .download(request, fixture.sink())
        .await
        .unwrap();

    assert!(started.elapsed() >= Duration::from_millis(90));
    assert_eq!(fs::read(fixture.part_path()).unwrap(), bytes);
}

#[allow(dead_code)]
fn assert_send<T: Send>() {}

#[test]
fn public_download_types_are_sendable_for_the_service_layer() {
    assert_send::<DownloadRequest>();
    assert_send::<DownloadProgress>();
    assert_send::<DownloadManager>();
    assert!(Path::new("archive.zip.part").extension().is_some());
}
