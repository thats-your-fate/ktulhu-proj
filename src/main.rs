mod app_state;
mod config;
mod messages;
mod worker;
mod util;

use axum::{Router, routing::get};
use std::{sync::Arc, path::Path};
use tokio::{net::TcpListener, sync::{Mutex, broadcast}};
use crate::{
    app_state::{AppState, WorkerState},
    config::AppConfig,
    worker::manager::{spawn_workers_from_config, spawn_node_process_from_config},
};
use tracing::{info, warn};
use crate::util::process_registry::{ProcessRegistry, watch_shutdown};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // 🔧 Load configuration
    let cfg = AppConfig::load()?;
    info!("Loaded {} worker configs", cfg.workers.len());

    // 🧱 Shared process registry
    let registry = Arc::new(ProcessRegistry::default());

    // 🧠 Spawn Python inference workers
    let raw_workers = spawn_workers_from_config(&cfg, registry.clone()).await;
    let worker_states = raw_workers
        .into_iter()
        .map(|w| WorkerState { worker: w, busy: false })
        .collect::<Vec<_>>();

    // 🪄 Check Node.js binary presence
    let node_bin = Path::new("./node-v22-linux-x64/bin/node");
    if !node_bin.exists() {
        warn!("⚠️ Node.js v22 binary not found. Run `./init_node.sh` to install it.");
    } else {
        info!("✅ Found local Node.js binary at {}", node_bin.display());
    }

    // 🧩 Optional Node.js orchestrator (scraper/controller)
    spawn_node_process_from_config(&cfg, registry.clone()).await;

    // 🔌 Shared app state (for WebSocket or HTTP routes)
    let (status_tx, _) = broadcast::channel(32);
    let app_state = AppState {
        status_tx,
        workers: Arc::new(Mutex::new(worker_states)),
    };

    let addr: std::net::SocketAddr = ([0, 0, 0, 0], 8080).into();
    info!("🚀 Server running on {}", addr);


    tokio::select! {
        _ = watch_shutdown(registry.clone()) => {},
        // 👇 Dummy main loop, replace with your server in future
        _ = async {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        } => {},
    }

    Ok(())
}
