use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct NodeProcessConfig {
    /// Friendly name (for logs)
    pub name: String,
    pub script: String,
    pub sockets: Vec<String>,

    #[serde(default)]
    pub env: Option<Vec<(String, String)>>,
    pub cwd: Option<String>,


    #[serde(default)]
    pub nodecwd: Option<String>,
        #[serde(default)]
    pub tunnel: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkerConfig {
    pub name: String,
    pub socket: String,
    pub gpu: String,
    pub model: String,
    #[serde(default)]
    pub script: Option<String>,
}

/// Kafka connection settings
#[derive(Debug, Deserialize, Clone)]
pub struct KafkaConfig {
    pub brokers: String,
    pub topic: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub workers: Vec<WorkerConfig>,
    pub node_process: Option<NodeProcessConfig>,

    /// Optional Python binary path (e.g. "/usr/bin/python3")
    #[serde(default)]
    pub python_bin: Option<String>,

    /// Optional Hugging Face home/cache directory
    #[serde(default)]
    pub hf_home: Option<PathBuf>,

    /// Optional Kafka broker/topic configuration
    #[serde(default)]
    pub kafka: Option<KafkaConfig>,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let json = include_str!("../config.json");
        let cfg: Self = serde_json::from_str(json)?;

        // If Hugging Face home is set, export it
        if let Some(ref path) = cfg.hf_home {
            std::env::set_var("HF_HOME", path);
            std::env::set_var("TRANSFORMERS_CACHE", path.join("transformers"));
            println!("📦 Using Hugging Face home: {}", path.display());
        }

        Ok(cfg)
    }
}
