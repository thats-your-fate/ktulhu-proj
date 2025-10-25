mod app_state;
mod config;
mod messages;
mod worker;
mod util;
mod kafka;
mod routes;

use axum::{Router};
use std::{sync::Arc, path::Path, net::SocketAddr, collections::HashMap};
use tokio::{
    net::TcpListener,
    sync::{Mutex, broadcast, RwLock},
};
use tracing::{info, warn};

use crate::{
    app_state::{AppState, WorkerState},
    config::AppConfig,
    worker::manager::{spawn_workers_from_config, spawn_node_process_from_config},
    kafka::chat_summary::{spawn_chat_summary_consumer, ChatSummary},
    routes::chat_summary::{router as chat_summary_router, ChatSummaryState},
    util::process_registry::{ProcessRegistry, watch_shutdown},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // 🔧 Load config
    let cfg = AppConfig::load()?;
    info!("Loaded {} worker configs", cfg.workers.len());

    let registry = Arc::new(ProcessRegistry::default());

    // 🧠 Spawn Python inference workers
    let raw_workers = spawn_workers_from_config(&cfg, registry.clone()).await;
    let worker_states = raw_workers
        .into_iter()
        .map(|w| WorkerState { worker: w, busy: false })
        .collect::<Vec<_>>();

    // 🪄 Check Node.js binary
    let node_bin = Path::new("./node-v22-linux-x64/bin/node");
    if !node_bin.exists() {
        warn!("⚠️ Node.js v22 binary not found. Run `./init_node.sh` to install it.");
    } else {
        info!("✅ Found local Node.js binary at {}", node_bin.display());
    }

    // 🚀 Spawn optional Node.js process
    spawn_node_process_from_config(&cfg, registry.clone()).await;

    // 📡 Initialize broadcast channels and shared state
    let (status_tx, _) = broadcast::channel(32);
    let app_state = AppState {
        status_tx,
        workers: Arc::new(Mutex::new(worker_states)),
    };

    // 🧩 Kafka consumer for chat summaries
    let (chat_summary_tx, chat_summary_map) = {
        let (tx, _rx) = broadcast::channel::<ChatSummary>(32);
        let map = Arc::new(RwLock::new(HashMap::new()));
        spawn_chat_summary_consumer(
            "localhost:9092".to_string(),
            "user_messages".to_string(),
            tx.clone(),
            map.clone(),
        ).await;
        (tx, map)
    };

    // 🧠 Chat summary routes
    let chat_state = ChatSummaryState {
        tx: chat_summary_tx.clone(),
        data: chat_summary_map.clone(),
    };

    let app = Router::new()
        .merge(chat_summary_router())
        .with_state(chat_state);

    // 🌍 Bind and serve
    let addr: SocketAddr = ([0, 0, 0, 0], 8080).into();
    info!("🚀 Server running on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    tokio::select! {
        _ = axum::serve(listener, app.into_make_service()) => {},
        _ = watch_shutdown(registry.clone()) => {},
    }

    Ok(())
}
