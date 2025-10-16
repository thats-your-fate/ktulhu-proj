use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum StatusMsg {
    Ready,
    Busy { current: Option<String> },
    WarmingUp,
    Error { message: String },
    Queue { queued: usize, active: usize, capacity: usize },
    PoolStatus(Vec<WorkerSnapshot>), // ✅ required for monitor
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferRequest {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferResponse {
    pub id: String,
    pub output: String,
    #[serde(default)]
    pub tokens: Option<usize>,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkerSnapshot {
    pub name: String,
    pub busy: bool,
}