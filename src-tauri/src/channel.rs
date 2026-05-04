use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State as AxState,
    },
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use bytes::Bytes;
use local_ip_address::local_ip;
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use tauri::State;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};

#[derive(RustEmbed)]
#[folder = "src/local_server"]
struct Asset;

// 오디오 데이터를 WebSocket 클라이언트들에게 브로드캐스트하는 채널
pub struct AudioBroadcast(pub broadcast::Sender<Bytes>);

// 실행 중인 서버 태스크 핸들 (중복 실행 방지)
pub struct ServerHandle(pub Mutex<Option<tokio::task::JoinHandle<()>>>);

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    match Asset::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
            ([(header::CONTENT_TYPE, mime)], file.data.into_owned()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    AxState(tx): AxState<broadcast::Sender<Bytes>>,
) -> Response {
    ws.on_upgrade(move |socket| stream_audio(socket, tx))
}

async fn stream_audio(mut socket: WebSocket, tx: broadcast::Sender<Bytes>) {
    let mut rx = tx.subscribe();
    loop {
        match rx.recv().await {
            Ok(data) => {
                if socket.send(Message::Binary(data)).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => break,
        }
    }
}

#[tauri::command]
pub async fn open_room(
    broadcast: State<'_, AudioBroadcast>,
    server: State<'_, ServerHandle>,
) -> Result<String, String> {
    // 기존 서버 종료
    if let Some(handle) = server.0.lock().await.take() {
        handle.abort();
    }

    let ip = local_ip().map_err(|e| e.to_string())?;
    let tx = broadcast.0.clone();

    let router = Router::new()
        .route("/", get(|| async { Redirect::to("/receiver.html") }))
        .route("/audio", get(ws_handler))
        .fallback(static_handler)
        .with_state(tx);

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 6767)))
        .await
        .map_err(|e| format!("포트 6767 바인딩 실패: {e}"))?;

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.ok();
    });

    *server.0.lock().await = Some(handle);

    Ok(format!("http://{}:6767", ip))
}
