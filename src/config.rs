use serde::Deserialize;



#[derive(Debug, Deserialize, Clone)]
pub struct NodeProcessConfig {
    /// Friendly name (for logs)
    pub name: String,

    /// Path to the Node.js script to run (relative to /scripts)
    pub script: String,

    /// List of sockets this Node process should connect to
    pub sockets: Vec<String>,

    /// Optional environment variables for the Node process
    #[serde(default)]
    pub env: Option<Vec<(String, String)>>,

    /// Path for Node orchestrator working dir (Python uses `cwd` for scripts)
    pub cwd: Option<String>,

    /// Optional Node-specific CWD override
    #[serde(default)]
    pub nodecwd: Option<String>,
}


#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub workers: Vec<WorkerConfig>,
    pub node_process: Option<NodeProcessConfig>,
    pub python_bin: Option<String>, // 👈 NEW
}

#[derive(Debug, Deserialize)]
pub struct WorkerConfig {
    pub name: String,
    pub socket: String,
    pub gpu: String,
    pub model: String,
    pub script: Option<String>,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let json = include_str!("../config.json");
        Ok(serde_json::from_str(json)?)
    }
}


