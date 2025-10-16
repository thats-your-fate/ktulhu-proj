use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct WorkerConfig {
    pub name: String,
    #[serde(default)]    
    pub script: String,
    pub socket: String,
    pub gpu: String,
    #[serde(default)]
    pub model: String, // 🆕 new field for model name
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub workers: Vec<WorkerConfig>,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let json = include_str!("../config.json");
        Ok(serde_json::from_str(json)?)
    }
}
