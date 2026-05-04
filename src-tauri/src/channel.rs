use axum::{routing::get, Router, response::Redirect};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use std::net::SocketAddr;
use std::path::PathBuf;
use local_ip_address::local_ip;

// CARGO_MANIFEST_DIR = src-tauri/, so parent = project root
fn dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("dist")
}

#[tauri::command]
pub async fn open_room() -> Result<String, String> {
    let ip = local_ip().map_err(|e| e.to_string())?;
    let port = 6767;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let dist_path = dist_dir();
    if !dist_path.exists() {
        return Err(format!(
            "'dist' 폴더를 찾을 수 없습니다 ({}). 먼저 'npm run build'를 실행하세요.",
            dist_path.display()
        ));
    }

    tokio::spawn(async move {
        let app = Router::new()
            .route("/hello", get(|| async { "Axum server is running!" }))
            .route("/receiver", get(|| async { Redirect::to("/receiver.html") }))
            .fallback_service(ServeDir::new(dist_dir()));

        let listener = TcpListener::bind(addr).await.unwrap();
        println!("Axum serving '{}' at http://{}:{}", dist_dir().display(), ip, port);
        axum::serve(listener, app).await.unwrap();
    });

    Ok(format!("http://{}:{}/receiver.html", ip, port))
}
