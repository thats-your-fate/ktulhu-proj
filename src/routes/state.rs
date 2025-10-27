use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use crate::kafka::messages::MessageEvent;

/// Unified shared state for all routes
#[derive(Clone)]
pub struct RouteState {
    /// Broadcast channel for live updates
    pub tx: broadcast::Sender<MessageEvent>,
    /// In-memory map: chat_id → Vec<MessageEvent>
    pub messages: Arc<RwLock<HashMap<String, Vec<MessageEvent>>>>,
}
