use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use serde_json::Value;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub entity: String,
    pub aspect: Option<String>,
    pub value: Option<String>,
    #[serde(default)]
    pub attributes: HashMap<String, serde_json::Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatState {
    pub summary: Option<String>,
    pub intents: Vec<Value>,   // ← changed
    pub facts: Vec<Value>,     // ← changed
}





#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDelta {
    pub chat_id: String,
    pub ts: i64,
    pub last_processed_ts: i64,

    pub summary: Option<String>,

    pub intent: Option<Value>,           // ← changed
    pub facts: Vec<Value>,               // ← changed

    pub state: Option<ChatState>,
}
