use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::routes::state::RouteState;
use crate::storage::MessageStore;

pub fn router() -> Router<RouteState> {
    Router::new()
        .route("/chat-thread/:chat_id", get(get_thread))
}

async fn get_thread(
    Path(chat_id): Path<String>,
    State(state): State<RouteState>,
) -> impl IntoResponse 
{
    // 1. Try sliding-window in-memory messages first
    if let Some(recent) = state.recent_messages.read().await.get(&chat_id) {
        if !recent.is_empty() {
            let mut msgs = recent.clone();
            msgs.sort_by_key(|m| m.ts);

            let out: Vec<_> = msgs.iter().map(|m| {
                json!({
                    "id": m.id,
                    "role": m.role,
                    "text": m.text,
                    "summary": m.summary,
                    "ts": m.ts
                })
            }).collect();

            return Json(json!({
                "chat_id": chat_id,
                "messages": out,
                "source": "memory"
            }));
        }
    }

    // 2. Fallback — full history from RocksDB
    let messages = match MessageStore::load_thread(&state.storage, &chat_id) {
        Ok(v) => v,
        Err(e) => {
            return Json(json!({
                "chat_id": chat_id,
                "messages": [],
                "error": e.to_string()
            }));
        }
    };

    let out: Vec<_> = messages.iter().map(|m| {
        json!({
            "id": m.id,
            "role": m.role,
            "text": m.text,
            "summary": m.summary,
            "ts": m.ts
        })
    }).collect();

    Json(json!({
        "chat_id": chat_id,
        "messages": out,
        "source": "db"
    }))
}
