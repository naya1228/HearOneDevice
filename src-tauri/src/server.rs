use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use std::sync::Mutex;
use tauri::State as TauriState;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

#[derive(RustEmbed)]
#[folder = "../docs/receiver/"]
struct ReceiverAssets;

#[derive(Clone)]
struct AppState {
    audio_tx: broadcast::Sender<Vec<u8>>,
}

pub struct ServerHandle {
    pub task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub port: Mutex<Option<u16>>,
}

impl ServerHandle {
    pub fn new() -> Self {
        Self {
            task: Mutex::new(None),
            port: Mutex::new(None),
        }
    }
}

pub const SERVER_PORT: u16 = 6767;

#[tauri::command]
pub async fn start_server(
    handle: TauriState<'_, ServerHandle>,
    audio_tx: TauriState<'_, broadcast::Sender<Vec<u8>>>,
) -> Result<u16, String> {
    // 이미 실행 중이면 그 포트 반환
    if let Some(port) = *handle.port.lock().unwrap() {
        return Ok(port);
    }

    let state = AppState {
        audio_tx: audio_tx.inner().clone(),
    };

    let app = Router::new()
        .route("/audio", get(audio_ws))
        .route("/", get(index_handler))
        .fallback(static_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], SERVER_PORT));
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| format!("포트 {} 바인딩 실패: {e}", SERVER_PORT))?;

    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    *handle.task.lock().unwrap() = Some(task);
    *handle.port.lock().unwrap() = Some(SERVER_PORT);

    Ok(SERVER_PORT)
}

#[tauri::command]
pub async fn stop_server(handle: TauriState<'_, ServerHandle>) -> Result<(), String> {
    if let Some(task) = handle.task.lock().unwrap().take() {
        task.abort();
    }
    *handle.port.lock().unwrap() = None;
    Ok(())
}

async fn audio_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.audio_tx.subscribe();
    ws.on_upgrade(move |socket| handle_audio_socket(socket, rx))
}

async fn handle_audio_socket(socket: WebSocket, mut rx: broadcast::Receiver<Vec<u8>>) {
    let (mut sender, mut receiver) = socket.split();

    // 클라이언트 → 서버 메시지는 무시하지만 close 감지용으로 읽기 태스크 분리
    let close_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if matches!(msg, Ok(Message::Close(_)) | Err(_)) {
                break;
            }
        }
    });

    loop {
        match rx.recv().await {
            Ok(chunk) => {
                if sender.send(Message::Binary(chunk)).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => {
                // 청취자가 늦으면 일부 청크 손실. 계속 진행.
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }

    close_task.abort();
}

async fn index_handler() -> impl IntoResponse {
    serve_asset("index.html")
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        serve_asset("index.html")
    } else {
        serve_asset(path)
    }
}

fn serve_asset(path: &str) -> Response {
    match ReceiverAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap(),
    }
}
