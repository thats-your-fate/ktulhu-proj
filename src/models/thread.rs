use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,            // exact same as chat_id
    pub persona_id: Option<String>,
    pub created_at: i64,
    pub last_ts: i64,          // last message timestamp (index-friendly)
}
