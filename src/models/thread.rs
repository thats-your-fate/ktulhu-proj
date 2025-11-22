use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,  
    pub created_at: i64,
    pub last_ts: i64,         
}
