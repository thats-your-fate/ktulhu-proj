mod config;
mod messages;
mod worker;
mod util;
mod kafka;
mod routes;
mod scraper;
mod storage;
mod models;

use axum::Router;
use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{broadcast, RwLock},
};
use tracing::{info, warn};
use tower_http::cors::{Any, CorsLayer, AllowOrigin};
use http::Method;
use std::time::Duration;

use crate::{
    config::AppConfig,
    kafka::messages::{spawn_chat_summary_consumer, MessageEvent},
    util::process_registry::{watch_shutdown, ProcessRegistry},
    worker::manager::{spawn_node_process_from_config, spawn_workers_from_config},
    scraper::manager::spawn_scrapers_from_config,
    storage::{Storage},
    routes::{
        chat_summary::router as chat_summary_router,
        chat_thread::router as chat_thread_router,
        state::RouteState,
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // 🔧 Load config
    let cfg = AppConfig::load()?;
    info!("Loaded {} worker configs", cfg.workers.len());

    let registry = Arc::new(ProcessRegistry::default());

    //  Spawn Python inference workers
    let _raw_workers = spawn_workers_from_config(&cfg, registry.clone()).await;

    // 🪄 Check Node.js binary
    let node_bin = Path::new("./node-v22-linux-x64/bin/node");
    if !node_bin.exists() {
        warn!("⚠️ Node.js v22 binary not found. Run ./init_node.sh");
    } else {
        info!("Found Node binary at {}", node_bin.display());
    }

    //  Optional Node.js proxy
    spawn_node_process_from_config(&cfg, registry.clone()).await;

    // Scrapers
    spawn_scrapers_from_config(&cfg, registry.clone()).await;

    // 🧩 Shared structures
    let (chat_tx, _rx) = broadcast::channel::<MessageEvent>(1024);


let recent_messages: Arc<RwLock<HashMap<String, Vec<MessageEvent>>>> =
    Arc::new(RwLock::new(HashMap::new()));



    // 🔥 Initialize RocksDB
    let storage = Arc::new(Storage::open("db/messages_db")?);
    info!("🗄️ RocksDB initialized at db/messages_db");

    // 🔌 Start Kafka consumer
    if let Some(kafka_cfg) = &cfg.kafka {
        info!(
            "📡 Starting Kafka consumer at {} (topic: {})",
            kafka_cfg.brokers, kafka_cfg.topic
        );
        spawn_chat_summary_consumer(
            kafka_cfg.brokers.clone(),
            kafka_cfg.topic.clone(),
            chat_tx.clone(),
    recent_messages.clone(),   
    storage.clone(),      
        )
        .await;
    } else {
        warn!("⚠️ No Kafka configuration found — skipping Kafka consumer startup.");
    }

    // 🌐 Build route state
let route_state = RouteState {
    tx: chat_tx.clone(),
recent_messages: recent_messages.clone(),
    storage: storage.clone(),
};

    // 🪩 CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any)
        .max_age(Duration::from_secs(3600))
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            if let Ok(o) = origin.to_str() {
                o == "https://ktulhu.com"
                    || o.ends_with(".ktulhu.com")
                    || o.starts_with("http://localhost")
                    || o.starts_with("http://127.0.0.1")
            } else {
                false
            }
        }));

    // 🧠 Build Axum app
    let app = Router::new()
        .merge(chat_summary_router())
        .merge(chat_thread_router())
        .with_state(route_state)
        .layer(cors);

    // 🚀 Serve
    let addr: SocketAddr = ([0, 0, 0, 0], 8080).into();
    info!("🚀 Server running on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    tokio::select! {
        _ = axum::serve(listener, app.into_make_service()) => {},
        _ = watch_shutdown(registry.clone()) => {},
    }

    Ok(())
}
