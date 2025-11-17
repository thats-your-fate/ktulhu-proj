use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDelta {
    pub chat_id: String,
    pub session_id: Option<String>,
    pub device_hash: Option<String>,
    pub state_delta: serde_json::Value,
    pub ts: i64,
}
