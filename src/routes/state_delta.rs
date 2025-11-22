use axum::{
    extract::{State, Path},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde_json::json;

use crate::routes::state::RouteState;
use crate::storage::StateStore;

pub fn router() -> Router<RouteState> {
    Router::new()
        .route("/state-delta/last", get(list_last))
        .route("/state-delta/history/:chat_id", get(history))
}

async fn list_last(State(state): State<RouteState>) -> impl IntoResponse {
    let mut out = Vec::new();

    let chats = crate::storage::MessageStore::list_chat_ids(&state.storage)
        .unwrap_or_default();

    for chat_id in chats {
        if let Ok(Some(delta)) = StateStore::load_last_for_chat(&state.storage, &chat_id) {

            // ✔ Build JSON according to NEW StateDelta struct
            out.push(json!({
                "chat_id": chat_id,
                "ts": delta.ts,
                "last_processed_ts": delta.last_processed_ts,

                "intent": delta.intent,
                "facts": delta.facts,
                "summary": delta.summary,

                // This contains the merged ChatState from HistoryWorker
                "state": delta.state
            }));
        }
    }

    Json(json!({ "chats": out }))
}

pub async fn history(
    State(state): State<RouteState>,
    Path(chat_id): Path<String>,
) -> impl IntoResponse {
    let map = state.recent_state.read().await;

    let list = map
        .get(&chat_id)
        .cloned()
        .unwrap_or_else(|| vec![]);

    Json(json!({
        "chat_id": chat_id,
        "history": list,          // <-- VALID ARRAY
        "count": list.len()
    }))
}
