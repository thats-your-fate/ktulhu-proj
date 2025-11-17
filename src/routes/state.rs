use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};

use crate::kafka::messages::MessageEvent;
use crate::storage::Storage;

#[derive(Clone)]
pub struct RouteState {
    pub storage: Arc<Storage>, // RocksDB
    pub tx: broadcast::Sender<MessageEvent>, // WS
    pub recent_messages: Arc<RwLock<HashMap<String, Vec<MessageEvent>>>>, // sliding window
}
