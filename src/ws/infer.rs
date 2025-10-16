use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{extract::State, response::IntoResponse};
use futures::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec};
use crate::{app_state::AppState, messages::{InferRequest, InferResponse, StatusMsg}, util::uuid_like};

pub async fn ws_infer_a(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_infer_ws(socket, state, "/tmp/infer_a.sock"))
}

pub async fn ws_infer_b(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_infer_ws(socket, state, "/tmp/infer_b.sock"))
}

async fn handle_infer_ws(mut socket: WebSocket, state: AppState, socket_path: &str) {
    tracing::info!("📨 WS connection for socket {}", socket_path);

    while let Some(Ok(Message::Text(txt))) = socket.next().await {
        let req_id = uuid_like();
        let req = InferRequest { id: req_id.clone(), text: txt.clone(), mode: None };

        tracing::info!("🧠 Infer `{}` via `{}` ({} chars)", req_id, socket_path, txt.len());
        let _ = state.status_tx.send(StatusMsg::Busy { current: Some(req_id.clone()) });

        match do_infer(req, socket_path).await {
            Ok(resp) => {
                let _ = socket.send(Message::Text(serde_json::to_string(&resp).unwrap())).await;
                let _ = state.status_tx.send(StatusMsg::Ready);
            }
            Err(e) => {
                tracing::error!("❌ Inference `{}` failed: {}", req_id, e);
                let _ = state.status_tx.send(StatusMsg::Error { message: e.to_string() });
                let _ = socket.send(Message::Text(
                    serde_json::json!({ "error": e.to_string() }).to_string()
                )).await;
            }
        }
    }
}

pub async fn do_infer(req: InferRequest, socket_path: &str) -> anyhow::Result<InferResponse> {
    let stream = UnixStream::connect(socket_path).await?;
    let mut framed = Framed::new(stream, LinesCodec::new());

    let line = serde_json::to_string(&req)?;
    framed.send(line).await?;

    if let Some(Ok(line)) = framed.next().await {
        let resp: InferResponse = serde_json::from_str(line.trim())?;
        Ok(resp)
    } else {
        anyhow::bail!("no response from Python")
    }
}
