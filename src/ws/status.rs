use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{extract::State, response::IntoResponse};
use futures::SinkExt;
use tokio::time::{interval, Duration};
use crate::{
    app_state::AppState,
    messages::{StatusMsg, WorkerSnapshot},
};

pub async fn ws_status(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_status_ws(socket, state))
}

async fn handle_status_ws(mut socket: WebSocket, state: AppState) {
    let mut rx = state.status_tx.subscribe();

    // 1️⃣ Send initial Ready
    let _ = socket
        .send(Message::Text(serde_json::to_string(&StatusMsg::Ready).unwrap()))
        .await;

    // 2️⃣ Spawn background ticker that broadcasts pool status
    let pool = state.workers.clone();
    let tx = state.status_tx.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(2));
        loop {
            ticker.tick().await;
            let snapshot = {
                let workers = pool.lock().await;
                workers
                    .iter()
                    .map(|w| WorkerSnapshot {
                        name: w.worker.name.clone(),
                        busy: w.busy,
                    })
                    .collect::<Vec<_>>()
            };
            let _ = tx.send(StatusMsg::PoolStatus(snapshot));
        }
    });

    // 3️⃣ Relay all broadcast messages to this WebSocket
    while let Ok(msg) = rx.recv().await {
        let json = serde_json::to_string(&msg).unwrap();
        if socket.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}
