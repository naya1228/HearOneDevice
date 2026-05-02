use std::sync::Arc;
use axum::extract::State as AxState;
use axum::routing::{get, post};
use axum::Router;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

pub struct RtcSession {
    pub peer: Arc<RTCPeerConnection>,
    pub audio_track: Option<Arc<TrackLocalStaticSample>>,
}

pub struct RtcState {
    pub session: Mutex<Option<RtcSession>>,
    pub server_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

fn create_api() -> Result<webrtc::api::API, String> {
    let mut m = MediaEngine::default();
    m.register_default_codecs().map_err(|e| e.to_string())?;
    let mut registry = Registry::new();
    registry =
        register_default_interceptors(registry, &mut m).map_err(|e| e.to_string())?;
    Ok(APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build())
}

fn ice_config() -> RTCConfiguration {
    RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    }
}

// --------------- signaling server ---------------

#[derive(Clone)]
struct SignalCtx {
    offer_sdp: String,
    answer_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<String>>>>,
}

async fn handle_get_offer(AxState(ctx): AxState<SignalCtx>) -> String {
    ctx.offer_sdp.clone()
}

async fn handle_post_answer(AxState(ctx): AxState<SignalCtx>, body: String) -> &'static str {
    if let Some(tx) = ctx.answer_tx.lock().await.take() {
        let _ = tx.send(body);
    }
    "ok"
}

// --------------- helpers ---------------

fn setup_connection_events(pc: &RTCPeerConnection, app: AppHandle) {
    pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        println!("Peer connection: {s}");
        let status = match s {
            RTCPeerConnectionState::Connected => "connected",
            RTCPeerConnectionState::Disconnected => "disconnected",
            RTCPeerConnectionState::Failed => "failed",
            RTCPeerConnectionState::Closed => "closed",
            _ => return Box::pin(async {}),
        };
        let _ = app.emit("rtc-status", status);
        Box::pin(async {})
    }));
}

async fn gather_local_sdp(pc: &RTCPeerConnection) -> Result<String, String> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    pc.on_ice_candidate(Box::new(move |c| {
        if c.is_none() {
            let _ = tx.try_send(());
        }
        Box::pin(async {})
    }));
    let _ = rx.recv().await;
    let desc = pc
        .local_description()
        .await
        .ok_or_else(|| "local description 없음".to_string())?;
    serde_json::to_string(&desc).map_err(|e| e.to_string())
}

async fn cleanup(state: &RtcState) {
    if let Some(handle) = state.server_task.lock().await.take() {
        handle.abort();
    }
    if let Some(session) = state.session.lock().await.take() {
        let _ = session.peer.close().await;
    }
}

// --------------- tauri commands ---------------

#[tauri::command]
pub async fn open_room(app: AppHandle, state: State<'_, RtcState>) -> Result<(), String> {
    cleanup(&state).await;

    let api = create_api()?;
    let pc = Arc::new(
        api.new_peer_connection(ice_config())
            .await
            .map_err(|e| e.to_string())?,
    );

    let audio_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: "audio/opus".to_owned(),
            clock_rate: 48000,
            channels: 2,
            sdp_fmtp_line: String::new(),
            rtcp_feedback: vec![],
        },
        "audio".to_owned(),
        "share-your-sounds".to_owned(),
    ));

    let rtp_sender = pc
        .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await
        .map_err(|e| e.to_string())?;

    tokio::spawn(async move {
        let mut buf = vec![0u8; 1500];
        while let Ok((_, _)) = rtp_sender.read(&mut buf).await {}
    });

    setup_connection_events(&pc, app.clone());

    let offer = pc.create_offer(None).await.map_err(|e| e.to_string())?;
    pc.set_local_description(offer)
        .await
        .map_err(|e| e.to_string())?;

    let offer_sdp = gather_local_sdp(&pc).await?;

    *state.session.lock().await = Some(RtcSession {
        peer: Arc::clone(&pc),
        audio_track: Some(audio_track),
    });

    let (answer_tx, answer_rx) = tokio::sync::oneshot::channel::<String>();

    let router = Router::new()
        .route("/offer", get(handle_get_offer))
        .route("/answer", post(handle_post_answer))
        .with_state(SignalCtx {
            offer_sdp,
            answer_tx: Arc::new(Mutex::new(Some(answer_tx))),
        });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:6767")
        .await
        .map_err(|e| e.to_string())?;

    let app_bg = app.clone();
    let handle = tokio::spawn(async move {
        tokio::select! {
            _ = axum::serve(listener, router) => {}
            result = answer_rx => {
                if let Ok(answer_sdp) = result {
                    match serde_json::from_str::<RTCSessionDescription>(&answer_sdp) {
                        Ok(answer) => {
                            if let Err(e) = pc.set_remote_description(answer).await {
                                let _ = app_bg.emit("rtc-status", format!("error: {e}"));
                            }
                        }
                        Err(e) => {
                            let _ = app_bg.emit("rtc-status", format!("error: {e}"));
                        }
                    }
                }
            }
        }
    });

    *state.server_task.lock().await = Some(handle);

    Ok(())
}

#[tauri::command]
pub async fn connect_to_host(
    app: AppHandle,
    state: State<'_, RtcState>,
    host_ip: String,
) -> Result<(), String> {
    cleanup(&state).await;

    let client = reqwest::Client::new();
    let offer_sdp = client
        .get(format!("http://{}:6767/offer", host_ip))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let api = create_api()?;
    let pc = Arc::new(
        api.new_peer_connection(ice_config())
            .await
            .map_err(|e| e.to_string())?,
    );

    setup_connection_events(&pc, app.clone());

    pc.on_track(Box::new(move |track, _receiver, _transceiver| {
        println!("Track received: {:?}", track.codec().capability.mime_type);
        Box::pin(async move {})
    }));

    let offer: RTCSessionDescription =
        serde_json::from_str(&offer_sdp).map_err(|e| e.to_string())?;
    pc.set_remote_description(offer)
        .await
        .map_err(|e| e.to_string())?;

    let answer = pc
        .create_answer(None)
        .await
        .map_err(|e| e.to_string())?;
    pc.set_local_description(answer)
        .await
        .map_err(|e| e.to_string())?;

    let answer_sdp = gather_local_sdp(&pc).await?;

    client
        .post(format!("http://{}:6767/answer", host_ip))
        .body(answer_sdp)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    *state.session.lock().await = Some(RtcSession {
        peer: pc,
        audio_track: None,
    });

    Ok(())
}

#[tauri::command]
pub async fn close_rtc(state: State<'_, RtcState>) -> Result<(), String> {
    cleanup(&state).await;
    Ok(())
}
