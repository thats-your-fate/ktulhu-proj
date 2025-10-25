use axum::{
    extract::{State, ws::{WebSocketUpgrade, WebSocket}},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use futures_util::{SinkExt, StreamExt};
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

/// Return all chat summaries
async fn list_last(State(state): State<ChatSummaryState>) -> impl IntoResponse {
    let data = state.data.read().await;
    let chats: Vec<_> = data.values().cloned().collect();
    Json(json!({ "chats": chats }))
}

/// WebSocket stream of new chat summaries
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ChatSummaryState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket: WebSocket| async move {
        let mut rx = state.tx.subscribe();

        info!("🌐 New /chat-summary/ws connection");

        while let Ok(summary) = rx.recv().await {
            let msg = serde_json::to_string(&summary).unwrap();
            if socket.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                break;
            }
        }

        info!("❌ /chat-summary/ws disconnected");
    })
}
