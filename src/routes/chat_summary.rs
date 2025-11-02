use axum::{
    extract::{
        State,
        ws::{WebSocketUpgrade, WebSocket, Message},
    },
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::{sync::Arc};
use tokio::sync::RwLock;
use tracing::info;
use futures_util::SinkExt;

use crate::kafka::messages::MessageEvent;
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

/// 🧠 Returns the most recent `summary` per chat_id
async fn list_last(State(state): State<RouteState>) -> impl IntoResponse {
    let data = state.messages.read().await;

    let mut chats = Vec::new();

    for (chat_id, msgs) in data.iter() {
        if let Some(last_summary) = msgs.iter().rev().find(|m| m.role == "summary" && m.summary.is_some()) {
            let clean_summary = normalize_text(&last_summary.summary);
            let clean_text = normalize_text(&last_summary.text);

            chats.push(json!({
                "chat_id": chat_id,
                "summary": clean_summary,
                "text": clean_text,
                "ts": last_summary.ts
            }));
        }
    }

    Json(json!({ "chats": chats }))
}

/// 🌐 WebSocket: stream new summary messages in real time
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<RouteState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket: WebSocket| async move {
        let mut rx = state.tx.subscribe();
        info!("🌐 Connected to /chat-summary/ws");

        while let Ok(event) = rx.recv().await {
            if event.role != "summary" {
                continue;
            }

            // Clean values before sending
            let clean_summary = normalize_text(&event.summary);
            let clean_text = normalize_text(&event.text);

            let msg = json!({
                "chat_id": event.chat_id,
                "summary": clean_summary,
                "text": clean_text,
                "ts": event.ts
            });

            if let Err(e) = socket.send(Message::Text(msg.to_string())).await {
                info!("⚠️ WS send error: {e}");
                break;
            }
        }

        info!("❌ /chat-summary/ws disconnected");
    })
}
