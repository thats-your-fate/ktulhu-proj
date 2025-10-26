use axum::{
    extract::{
        State,
        ws::{WebSocketUpgrade, WebSocket, Message},
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use futures_util::SinkExt;
use tracing::info;

use crate::kafka::chat_summary::{ChatSummary, ChatMap};

#[derive(Clone)]
pub struct ChatSummaryState {
    pub tx: broadcast::Sender<ChatSummary>,
    pub data: ChatMap,
}

pub fn router() -> Router<ChatSummaryState> {
    Router::new()
        .route("/chat-summary/last", get(list_last))
        .route("/chat-summary/ws", get(ws_handler))
}

/// 🧩 Return all chat summaries (HTTP GET)
async fn list_last(State(state): State<ChatSummaryState>) -> impl IntoResponse {
    let data = state.data.read().await;

    // Map preview → summary for frontend
    let chats: Vec<_> = data
        .values()
        .map(|c| {
            json!({
                "chat_id": c.chat_id,
                "summary": c.summary,  // ✅ map preview → summary
                "ts": c.ts
            })
        })
        .collect();

    Json(json!({ "chats": chats }))
}

/// 🌐 WebSocket live feed of new chat summaries
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ChatSummaryState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket: WebSocket| async move {
        let mut rx = state.tx.subscribe();
        info!("🌐 New /chat-summary/ws connection");

        while let Ok(summary) = rx.recv().await {
            // Normalize structure before sending to frontend
            let msg = json!({
                "chat_id": summary.chat_id,
                "summary": summary.summary,  // ✅ rename key
                "ts": summary.ts
            });

            let text = serde_json::to_string(&msg).unwrap();

            if socket.send(Message::Text(text)).await.is_err() {
                break;
            }
        }

        info!("❌ /chat-summary/ws disconnected");
    })
}
