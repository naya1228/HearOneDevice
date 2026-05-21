use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use tauri::State;
use tokio::sync::Mutex;

pub struct TunnelHandle(pub Mutex<Option<Child>>);

/// 번들된 cloudflared 바이너리 경로 반환
/// Tauri의 externalBin은 빌드/dev 모두 실행파일 옆에 배치됨
fn cloudflared_path() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("실행파일 디렉토리를 찾을 수 없음")?;

    #[cfg(target_os = "windows")]
    let path = dir.join("cloudflared.exe");
    #[cfg(not(target_os = "windows"))]
    let path = dir.join("cloudflared");

    if !path.exists() {
        return Err(format!("cloudflared 바이너리가 없습니다: {:?}", path));
    }

    // Linux: 실행 권한 보장 (번들 시 유실될 수 있음)
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        let mode = meta.permissions().mode();
        if mode & 0o111 == 0 {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
        }
    }

    Ok(path)
}

#[tauri::command]
pub async fn open_tunnel(state: State<'_, TunnelHandle>) -> Result<String, String> {
    // 기존 터널 종료
    kill_child(&mut state.0.lock().await);

    let bin = cloudflared_path()?;

    let mut child = Command::new(&bin)
        .args(["tunnel", "--url", "http://localhost:6767", "--no-autoupdate"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cloudflared 실행 실패: {e}"))?;

    let stderr = child.stderr.take().unwrap();

    // 별도 스레드에서 stderr 읽어 터널 URL과 연결 등록 신호를 추적
    // 1) URL 라인이 먼저 등장
    // 2) "Registered tunnel connection" 라인이 등장해야 실제 접속 가능
    enum TunnelEvent {
        Url(String),
        Ready,
    }
    let (tx, rx) = std::sync::mpsc::channel::<TunnelEvent>();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            if let Some(url) = extract_tunnel_url(&line) {
                let _ = tx.send(TunnelEvent::Url(url));
            } else if line.contains("Registered tunnel connection") {
                let _ = tx.send(TunnelEvent::Ready);
                return;
            }
        }
    });

    // URL과 Ready 둘 다 받을 때까지 최대 45초 대기
    let url = tokio::task::spawn_blocking(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
        let mut url: Option<String> = None;
        let mut ready = false;
        while !(url.is_some() && ready) {
            let remaining = deadline
                .checked_duration_since(std::time::Instant::now())
                .ok_or_else(|| "터널 준비 타임아웃 (45초)".to_string())?;
            match rx.recv_timeout(remaining) {
                Ok(TunnelEvent::Url(u)) => url = Some(u),
                Ok(TunnelEvent::Ready) => ready = true,
                Err(_) => return Err("터널 이벤트를 받지 못했습니다 (45초 타임아웃)".to_string()),
            }
        }
        Ok(url.unwrap())
    })
    .await
    .map_err(|e| e.to_string())??;

    *state.0.lock().await = Some(child);

    Ok(url)
}

#[tauri::command]
pub async fn close_tunnel(state: State<'_, TunnelHandle>) -> Result<(), String> {
    kill_child(&mut state.0.lock().await);
    Ok(())
}

fn kill_child(guard: &mut tokio::sync::MutexGuard<'_, Option<Child>>) {
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait(); // 좀비 프로세스 방지
    }
}

/// stderr 한 줄에서 trycloudflare.com URL 추출
fn extract_tunnel_url(line: &str) -> Option<String> {
    let start = line.find("https://")?;
    let rest = &line[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '|')
        .unwrap_or(rest.len());
    let url = rest[..end].trim_end_matches(|c: char| !c.is_alphanumeric() && c != '/');
    if url.contains("trycloudflare.com") {
        Some(url.to_string())
    } else {
        None
    }
}
