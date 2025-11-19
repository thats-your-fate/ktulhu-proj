use axum::{
    extract::{
        State,
        ws::{WebSocketUpgrade, WebSocket, Message},
    },
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use futures_util::{StreamExt, SinkExt};
use serde_json::{json, Value};
use tracing::info;

use crate::storage::MessageStore;

use crate::routes::state::RouteState;

/// 🧩 Utility: safely unwrap a JSON-encoded string if needed
fn normalize_text(input: &Option<String>) -> String {
    if let Some(s) = input {
        // Try parsing if it looks like JSON
        if s.trim_start().starts_with('{') {
            if let Ok(v) = serde_json::from_str::<Value>(s) {
                // If there's a field "text" → return that
                if let Some(inner) = v.get("text").and_then(|v| v.as_str()) {
                    return inner.to_string();
                }
                // Otherwise return pretty version of JSON
                return v.to_string();
            }
        }
        // Normal case: plain string
        return s.clone();
    }
    String::new()
}

/// Build the router for chat summary endpoints
pub fn router() -> Router<RouteState> {
    Router::new()
        .route("/chat-summary/last", get(list_last))
        .route("/chat-summary/ws", get(ws_handler))
}

///  Returns the most recent `summary` per chat_id
async fn list_last(State(state): State<RouteState>) -> impl IntoResponse {
    let mut out = Vec::new();

    let threads = MessageStore::list_chat_ids(&state.storage)
        .unwrap_or_default();

    for chat_id in threads {
        if let Ok(Some(msg)) = MessageStore::load_last_summary(&state.storage, &chat_id) {
            out.push(json!({
                "chat_id": chat_id,
                "summary": msg.summary,
                "ts": msg.ts
            }));
        }
    }

    Json(json!({ "chats": out }))
}

///  WebSocket: stream new summary messages in real time
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<RouteState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket: WebSocket| async move {
        // Split so we can read + write independently
        let (mut ws_tx, mut ws_rx) = socket.split();
        let mut rx = state.tx.subscribe();

        info!(" Connected to /chat-summary/ws");

        loop {
            tokio::select! {
                // 🔹 Detect client disconnects / incoming messages
                incoming = ws_rx.next() => {
                    match incoming {
                        Some(Ok(Message::Close(_))) | None => {
                            info!("❌ WS client disconnected");
                            break;
                        }
                        Some(Ok(_)) => {
                            // ignore any other client messages
                        }
                        Some(Err(e)) => {
                            info!("⚠️ WS receive error: {e}");
                            break;
                        }
                    }
                }

                // 🔹 Read from broadcast channel
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            if event.role != "summary" {
                                continue;
                            }

                            let clean_summary = normalize_text(&event.summary);

                            let msg = json!({
                                "chat_id": event.chat_id,
                                "summary": clean_summary,
                                "ts": event.ts
                            });

                            if ws_tx.send(Message::Text(msg.to_string())).await.is_err() {
                                info!("⚠️ WS send failed, closing connection");
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            info!("🔚 broadcast channel closed, stopping WS");
                            break;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            info!("⚠️ WS lagged, skipped {skipped} messages");
                            // you can `continue` here – we just miss some old summaries
                        }
                    }
                }
            }
        }

        // Try to close gracefully
        let _ = ws_tx.send(Message::Close(None)).await;
        info!("🔚 /chat-summary/ws closed gracefully");
    })
}

