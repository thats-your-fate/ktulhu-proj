use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct ScraperConfig {
    pub name: String,
    pub r#type: String,
    pub socket_path: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BraveConfig {
    pub apikey: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NodeProcessConfig {
    pub name: String,
    pub script: String,
    pub sockets: Vec<String>,

    #[serde(default)]
    pub env: Option<Vec<(String, String)>>,

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

#[derive(Debug, Deserialize, Clone)]
pub struct KafkaConfig {
    pub brokers: String,
    pub topic: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub workers: Vec<WorkerConfig>,
    pub node_process: Option<NodeProcessConfig>,

    #[serde(default)]
    pub python_bin: Option<String>,
    
    #[serde(default)]
        pub root: String,    

    #[serde(default)]
    pub kafka: Option<KafkaConfig>,

    #[serde(default)]
    pub web_scrapper: Vec<ScraperConfig>,

    #[serde(default)]
    pub brave: Option<BraveConfig>,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let json = include_str!("../config.json");
        let cfg: Self = serde_json::from_str(json)?;
        Ok(cfg)
    }
}
