use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec};
use serde::Deserialize;

#[derive(Deserialize)]
struct ClientInferInput {
    text: String,
    mode: Option<String>,
}


use crate::{
    app_state::AppState,
    messages::{InferRequest, InferResponse, StatusMsg},
    util::uuid_like,
};

/// Route: GET /ws/infer/:worker
pub async fn ws_infer(
    ws: WebSocketUpgrade,
    Path(worker): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let socket_path = format!("/tmp/infer_{}.sock", worker);
    ws.on_upgrade(move |socket| handle_infer_ws(socket, state, socket_path))
}

async fn handle_infer_ws(mut socket: WebSocket, state: AppState, socket_path: String) {
    tracing::info!("🔌 New WS inference connection -> {}", socket_path);

    while let Some(msg) = socket.next().await {
        match msg {
            Ok(Message::Text(txt)) => {
                let req_id = uuid_like();

                // ✅ Try to parse the incoming JSON
                let parsed: ClientInferInput = match serde_json::from_str(&txt) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!("⚠️ Could not parse JSON: {}", e);
                        let _ = socket.send(Message::Text(
                            json!({ "error": "Invalid JSON format" }).to_string(),
                        )).await;
                        continue;
                    }
                };

                let req = InferRequest {
                    id: req_id.clone(),
                    text: parsed.text,
                    mode: parsed.mode, // ✅ pass through client mode if provided
                };

                tracing::info!(
                    "🧠 [{}] Dispatching inference to `{}` ({} chars, mode={:?})",
                    req_id,
                    socket_path,
                    req.text.len(),
                    req.mode
                );

                let _ = state
                    .status_tx
                    .send(StatusMsg::Busy { current: Some(req_id.clone()) });

                match do_infer(req, &socket_path).await {
                    Ok(resp) => {
                        let payload = serde_json::to_string(&resp).unwrap();
                        let _ = socket.send(Message::Text(payload)).await;
                        let _ = state.status_tx.send(StatusMsg::Ready);
                    }
                    Err(e) => {
                        tracing::error!("❌ [{}] Inference failed: {}", req_id, e);
                        let _ = state.status_tx.send(StatusMsg::Error {
                            message: e.to_string(),
                        });
                        let _ = socket
                            .send(Message::Text(
                                json!({ "error": e.to_string(), "id": req_id }).to_string(),
                            ))
                            .await;
                    }
                }
            }
            Ok(Message::Close(_)) => {
                tracing::info!("🔌 WebSocket closed by client");
                break;
            }
            Ok(_) => continue,
            Err(e) => {
                tracing::error!("⚠️ WS error: {}", e);
                break;
            }
        }
    }

    tracing::info!("🔚 WS session ended for {}", socket_path);
}


/// Perform the actual inference call by sending request JSON to the Python worker over a Unix socket.
pub async fn do_infer(req: InferRequest, socket_path: &str) -> anyhow::Result<InferResponse> {
    let stream = UnixStream::connect(socket_path).await?;
    let mut framed = Framed::new(stream, LinesCodec::new());

    // Send request JSON
    let line = serde_json::to_string(&req)?;
    framed.send(line).await?;

    // Await response
    if let Some(Ok(line)) = framed.next().await {
        let resp: InferResponse = serde_json::from_str(line.trim())?;
        Ok(resp)
    } else {
        anyhow::bail!("no response from Python worker")
    }
}
