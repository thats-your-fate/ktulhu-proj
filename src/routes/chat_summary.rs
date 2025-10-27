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
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::info;
use futures_util::SinkExt;

use crate::kafka::messages::MessageEvent;
use crate::routes::state::RouteState;



/// Build the router for chat summary endpoints
pub fn router() -> Router<RouteState> {
    Router::new()
        .route("/chat-summary/last", get(list_last))
        .route("/chat-summary/ws", get(ws_handler))
}

async fn list_last(State(state): State<RouteState>) -> impl IntoResponse {
    let data = state.messages.read().await;

    // Collect last "summary" message for every chat_id
    let chats: Vec<_> = data
        .iter()
        .filter_map(|(chat_id, msgs)| {
            msgs.iter()
                .rev()
                .find(|m| m.role == "summary" && m.summary.is_some())
                .map(|m| {
                    json!({
                        "chat_id": chat_id,
                        "summary": m.summary,
                        "ts": m.ts
                    })
                })
        })
        .collect();

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
            // Only forward summary role messages
            if event.role != "summary" {
                continue;
            }

            let msg = json!({
                "chat_id": event.chat_id,
                "summary": event.summary,
                "ts": event.ts
            });

            // Send JSON message to client
            if socket
                .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                .await
                .is_err()
            {
                break;
            }
        }

        info!("❌ /chat-summary/ws disconnected");
    })
}
