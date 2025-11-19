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
        .route("/messages/by-device/:device_hash", get(get_messages_by_device))
}

pub async fn get_messages_by_device(
    Path(device_hash): Path<String>,
    State(state): State<RouteState>,
) -> impl IntoResponse 
{
    let all = match MessageStore::load_all_messages(&state.storage) {
        Ok(v) => v,
        Err(e) => {
            return Json(json!({
                "device_hash": device_hash,
                "messages": [],
                "error": e.to_string(),
            }))
        }
    };

    let mut filtered: Vec<_> = all
        .into_iter()
        .filter(|m| m.device_hash.as_deref() == Some(&device_hash))
        .collect();

    filtered.sort_by_key(|m| m.ts);

    Json(json!({
        "device_hash": device_hash,
        "messages": filtered,
        "source": "db"
    }))
}
