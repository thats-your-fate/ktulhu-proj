mod app_state;
mod config;
mod messages;
mod worker;
mod ws;
mod util;

use axum::{Router, routing::get};
use tokio::net::TcpListener;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use crate::app_state::{AppState, WorkerState};
use crate::config::AppConfig;
use crate::worker::manager::spawn_workers_from_config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    // 🔧 load configuration
    let cfg = AppConfig::load()?;
    tracing::info!("Loaded {} worker configs", cfg.workers.len());

    // 🧠 spawn workers
    let raw_workers = spawn_workers_from_config(&cfg).await;
    let worker_states = raw_workers.into_iter()
        .map(|w| WorkerState { worker: w, busy: false })
        .collect::<Vec<_>>();

    // 🔌 build shared state
    let (status_tx, _) = broadcast::channel(32);
    let app_state = AppState {
        status_tx,
        workers: Arc::new(Mutex::new(worker_states)),
    };

    // 🌐 define routes
    let app = Router::new()
        .route("/ws/status", get(ws::status::ws_status))
        .route("/ws/infer", get(ws::unified::ws_unified))
        .with_state(app_state);

let addr: std::net::SocketAddr = ([0, 0, 0, 0], 8080).into();
    tracing::info!("🚀 Server running on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
