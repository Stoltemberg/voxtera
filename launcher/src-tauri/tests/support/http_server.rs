use std::{
    convert::Infallible,
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
    routing::get,
};
use futures_util::stream;
use tokio::{net::TcpListener, task::JoinHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMode {
    Normal,
    IgnoreRange,
    ChangedEtag,
    Disconnect,
    UnknownLength,
    Slow,
}

#[derive(Clone)]
struct ServerState {
    bytes: Arc<Vec<u8>>,
    etag: Arc<str>,
    mode: ServerMode,
    requests: Arc<Mutex<Vec<(Option<String>, Option<String>)>>>,
}

pub struct RangeServer {
    address: SocketAddr,
    state: ServerState,
    task: JoinHandle<()>,
}

impl RangeServer {
    pub async fn new(bytes: &[u8], etag: &str) -> Self {
        Self::with_mode(bytes, etag, ServerMode::Normal).await
    }

    pub async fn with_mode(bytes: &[u8], etag: &str, mode: ServerMode) -> Self {
        let state = ServerState {
            bytes: Arc::new(bytes.to_vec()),
            etag: Arc::from(etag),
            mode,
            requests: Arc::new(Mutex::new(Vec::new())),
        };
        let app = Router::new()
            .route("/asset", get(serve_asset))
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            address,
            state,
            task,
        }
    }

    pub fn url(&self) -> String { format!("http://{}/asset", self.address) }

    pub fn last_range(&self) -> Option<String> {
        self.state
            .requests
            .lock()
            .unwrap()
            .last()
            .and_then(|request| request.0.clone())
    }

    pub fn first_range(&self) -> Option<String> {
        self.state
            .requests
            .lock()
            .unwrap()
            .first()
            .and_then(|request| request.0.clone())
    }

    pub fn first_if_range(&self) -> Option<String> {
        self.state
            .requests
            .lock()
            .unwrap()
            .first()
            .and_then(|request| request.1.clone())
    }

    pub fn request_count(&self) -> usize { self.state.requests.lock().unwrap().len() }
}

impl Drop for RangeServer {
    fn drop(&mut self) { self.task.abort(); }
}

async fn serve_asset(State(state): State<ServerState>, headers: HeaderMap) -> Response<Body> {
    let requested_range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let if_range = headers
        .get(header::IF_RANGE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    state
        .requests
        .lock()
        .unwrap()
        .push((requested_range.clone(), if_range.clone()));

    if state.mode == ServerMode::Disconnect {
        let split = state.bytes.len().min(4096);
        let bytes = state.bytes.clone();
        let body = stream::unfold(0_u8, move |step| {
            let bytes = bytes.clone();
            async move {
                match step {
                    0 => {
                        tokio::time::sleep(Duration::from_millis(25)).await;
                        Some((
                            Ok::<_, io::Error>(Bytes::copy_from_slice(&bytes[..split])),
                            1,
                        ))
                    },
                    1 => {
                        tokio::time::sleep(Duration::from_millis(25)).await;
                        Some((Err(io::Error::other("fixture disconnect")), 2))
                    },
                    _ => None,
                }
            }
        });
        return response(
            StatusCode::OK,
            &state.etag,
            Some(state.bytes.len() as u64),
            None,
            Body::from_stream(body),
        );
    }

    if state.mode == ServerMode::UnknownLength {
        let chunks = state
            .bytes
            .chunks(1024)
            .map(|chunk| Ok::<_, Infallible>(Bytes::copy_from_slice(chunk)))
            .collect::<Vec<_>>();
        return response(
            StatusCode::OK,
            &state.etag,
            None,
            None,
            Body::from_stream(stream::iter(chunks)),
        );
    }

    if state.mode == ServerMode::Slow {
        let bytes = state.bytes.clone();
        let body = stream::unfold((bytes, 0usize), |(bytes, offset)| async move {
            if offset >= bytes.len() {
                None
            } else {
                tokio::time::sleep(Duration::from_millis(75)).await;
                let end = (offset + 1024).min(bytes.len());
                let chunk = Bytes::copy_from_slice(&bytes[offset..end]);
                Some((Ok::<_, Infallible>(chunk), (bytes, end)))
            }
        });
        return response(
            StatusCode::OK,
            &state.etag,
            Some(state.bytes.len() as u64),
            None,
            Body::from_stream(body),
        );
    }

    let can_resume = state.mode != ServerMode::IgnoreRange
        && requested_range.is_some()
        && if_range.as_deref() == Some(&state.etag);
    if can_resume {
        let offset = requested_range
            .as_deref()
            .and_then(|value| value.strip_prefix("bytes="))
            .and_then(|value| value.strip_suffix('-'))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap();
        let response_etag = if state.mode == ServerMode::ChangedEtag {
            "etag-v2"
        } else {
            &state.etag
        };
        return response(
            StatusCode::PARTIAL_CONTENT,
            response_etag,
            Some((state.bytes.len() - offset) as u64),
            Some(format!(
                "bytes {offset}-{}/{}",
                state.bytes.len() - 1,
                state.bytes.len()
            )),
            Body::from(state.bytes[offset..].to_vec()),
        );
    }

    response(
        StatusCode::OK,
        &state.etag,
        Some(state.bytes.len() as u64),
        None,
        Body::from(state.bytes.as_ref().clone()),
    )
}

fn response(
    status: StatusCode,
    etag: &str,
    content_length: Option<u64>,
    content_range: Option<String>,
    body: Body,
) -> Response<Body> {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(header::ETAG, HeaderValue::from_str(etag).unwrap());
    if let Some(length) = content_length {
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&length.to_string()).unwrap(),
        );
    }
    if let Some(range) = content_range {
        response.headers_mut().insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&range).unwrap(),
        );
    }
    response
}
