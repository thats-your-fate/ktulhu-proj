mod app_state;
mod config;
mod messages;
mod worker;
mod util;
mod kafka;
mod routes;

use axum::Router;
use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{broadcast, Mutex, RwLock},
};
use tracing::{info, warn};
use tower_http::cors::{Any, CorsLayer,AllowOrigin};

use crate::{
    app_state::{AppState, WorkerState},
    config::AppConfig,
    kafka::messages::{spawn_chat_summary_consumer, MessageEvent},
    util::process_registry::{watch_shutdown, ProcessRegistry},
    worker::manager::{spawn_node_process_from_config, spawn_workers_from_config},
    
};


use crate::routes::{
    chat_summary::router as chat_summary_router,
    chat_thread::router as chat_thread_router,
    state::RouteState,
};

use http::Method;
use std::time::Duration;



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // 🔧 Load config
    let cfg = AppConfig::load()?;
    info!("Loaded {} worker configs", cfg.workers.len());

    let registry = Arc::new(ProcessRegistry::default());

    //  Spawn Python inference workers
    let raw_workers = spawn_workers_from_config(&cfg, registry.clone()).await;
    let worker_states = raw_workers
        .into_iter()
        .map(|w| WorkerState { worker: w, busy: false })
        .collect::<Vec<_>>();

    // 🪄 Check Node.js binary
    let node_bin = Path::new("./node-v22-linux-x64/bin/node");
    if !node_bin.exists() {
        warn!(" Node.js v22 binary not found. Run `./init_node.sh` to install it.");
    } else {
        info!(" Found local Node.js binary at {}", node_bin.display());
    }

    // 🚀 Optional Node.js proxy
    spawn_node_process_from_config(&cfg, registry.clone()).await;

    //  App state
    let (status_tx, _) = broadcast::channel(32);
    let _app_state = AppState {
        status_tx,
        workers: Arc::new(Mutex::new(worker_states)),
    };

    //  Shared in-memory maps
    // Keep all messages per chat in memory
    let (chat_tx, _rx) = broadcast::channel::<MessageEvent>(64);
    let messages_map: Arc<RwLock<HashMap<String, Vec<MessageEvent>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    //  Start Kafka consumer (store & broadcast events)
//  Start Kafka consumer (store & broadcast events)
if let Some(kafka_cfg) = &cfg.kafka {
    info!(
        "📡 Starting Kafka consumer at {} (topic: {})",
        kafka_cfg.brokers, kafka_cfg.topic
    );
    spawn_chat_summary_consumer(
        kafka_cfg.brokers.clone(),
        kafka_cfg.topic.clone(),
        chat_tx.clone(),
        messages_map.clone(),
    )
    .await;
} else {
    warn!("⚠️ No Kafka configuration found in config.json — skipping Kafka consumer startup.");
}


let route_state = RouteState {
    tx: chat_tx.clone(),
    messages: messages_map.clone(),
};



let cors = CorsLayer::new()
    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
    .allow_headers(Any)
    .max_age(Duration::from_secs(3600))
    .allow_origin(AllowOrigin::predicate(|origin, _req_head| {
        if let Ok(origin_str) = origin.to_str() {
            origin_str.ends_with(".ktulhu.com")
                || origin_str.starts_with("http://localhost")
                || origin_str.starts_with("http://127.0.0.1")
        } else {
            false
        }
    }));


let app = Router::new()
    .merge(chat_summary_router())
    .merge(chat_thread_router())
    .with_state(route_state)
    .layer(cors);


    // 🌍 Serve
    let addr: SocketAddr = ([0, 0, 0, 0], 8080).into();
    info!("🚀 Server running on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    tokio::select! {
        _ = axum::serve(listener, app.into_make_service()) => {},
        _ = watch_shutdown(registry.clone()) => {},
    }

    Ok(())
}
