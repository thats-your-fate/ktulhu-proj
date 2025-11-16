use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::routes::state::RouteState;

/// Build router for chat thread messages
pub fn router() -> Router<RouteState> {
    Router::new()
        // Example: GET /chat-thread/952df5b9-1903-4f2a-9cb0-bfd0be7a2f3d
        .route("/chat-thread/:chat_id", get(get_thread))
}

/// 🧩 Return all messages for a given chat_id (chronological order)
async fn get_thread(
    Path(chat_id): Path<String>,
    State(state): State<RouteState>,
) -> impl IntoResponse {
    let data = state.messages.read().await;

    if let Some(messages) = data.get(&chat_id) {
        // Sort messages chronologically
        let mut sorted = messages.clone();
        sorted.sort_by_key(|m| m.ts.unwrap_or_default());

        let msgs: Vec<_> = sorted
            .iter()
            .map(|m| {
                json!({
                    "role": m.role,
                    "text": m.text,
                    "summary": m.summary,
                    "ts": m.ts
                })
            })
            .collect();

        Json(json!({
            "chat_id": chat_id,
            "messages": msgs
        }))
    } else {
        Json(json!({
            "chat_id": chat_id,
            "messages": []
        }))
    }
}
