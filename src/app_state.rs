use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use crate::{messages::StatusMsg, worker::Worker};

#[derive(Clone)]
pub struct WorkerState {
    pub worker: Worker,
    pub busy: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub status_tx: broadcast::Sender<StatusMsg>,
    pub workers: Arc<Mutex<Vec<WorkerState>>>,
}
