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
        .route("/chat-thread/:chat_id", axum::routing::delete(delete_thread))
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


async fn delete_thread(
    Path(chat_id): Path<String>,
    State(state): State<RouteState>,
) -> impl IntoResponse 
{
    // 1. Delete from in-memory sliding window
    {
        let mut lock = state.recent_messages.write().await;
        lock.remove(&chat_id);
    }

    // 2. Delete from RocksDB (pass full storage)
    match MessageStore::delete_thread(&state.storage, &chat_id) {
        Ok(()) => {
            println!("🗑️ Deleted thread {chat_id} from memory + DB");
            Json(json!({
                "chat_id": chat_id,
                "deleted": true,
                "source": ["memory", "db"]
            }))
        }

        Err(e) => {
            eprintln!("❌ Failed to delete thread {chat_id}: {}", e);
            Json(json!({
                "chat_id": chat_id,
                "deleted": false,
                "error": e.to_string()
            }))
        }
    }
}


