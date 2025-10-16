use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{extract::State, response::IntoResponse};
use futures::{StreamExt};
use serde::Deserialize;
use crate::{app_state::AppState, messages::{InferRequest, InferResponse}, util::uuid_like};
use crate::ws::infer::do_infer;

#[derive(Debug, Deserialize)]
struct InferInput {
    text: String,
}

pub async fn ws_unified(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_unified_ws(socket, state))
}

async fn handle_unified_ws(mut socket: WebSocket, state: AppState) {
    tracing::info!("🌐 Unified WebSocket connected");

    while let Some(Ok(Message::Text(txt))) = socket.next().await {
        // Parse client payload
        let payload: InferInput = match serde_json::from_str(&txt) {
            Ok(p) => p,
            Err(_) => {
                let _ = socket.send(Message::Text("{\"error\":\"invalid JSON\"}".into())).await;
                continue;
            }
        };

        // choose free worker
let mut workers = state.workers.lock().await;
if let Some(ws) = workers.iter_mut().find(|w| !w.busy) {
    ws.busy = true;
    let sock_path = ws.worker.socket_path.clone();
    let model_name = ws.worker.name.clone();
    drop(workers);

    let req = InferRequest {
        id: uuid_like(),
        text: payload.text,
        mode: Some(model_name),
    };

            match do_infer(req, &sock_path).await {
                Ok(resp) => {
                    let json = serde_json::to_string(&resp).unwrap();
                    let _ = socket.send(Message::Text(json)).await;
                }
                Err(e) => {
                    let msg = serde_json::json!({ "error": e.to_string() }).to_string();
                    let _ = socket.send(Message::Text(msg)).await;
                }
            }

            // mark worker free again
 let mut workers = state.workers.lock().await;
    if let Some(w) = workers.iter_mut().find(|x| x.worker.socket_path == sock_path) {
        w.busy = false;
    }
} else {
    let _ = socket
        .send(Message::Text("{\"error\":\"no free workers\"}".into()))
        .await;
}

    }

    tracing::info!("🔌 Unified WebSocket disconnected");
}
