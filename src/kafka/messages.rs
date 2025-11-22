use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig,
    Message,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error};
use crate::Storage;
use crate::storage::MessageStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEvent {
    pub id: String, 
    pub role: String,
    pub chat_id: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,    
    pub device_hash: Option<String>,
    pub text: Option<String>,
    pub summary: Option<String>,
    pub ts: i64,      
}


pub async fn spawn_chat_summary_consumer(
    brokers: String,
    topic: String,
    tx: broadcast::Sender<MessageEvent>,
    recent_messages: Arc<RwLock<HashMap<String, Vec<MessageEvent>>>>,
        storage: Arc<Storage>,   
) {
    tokio::spawn(async move {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "chat-summary-consumer")
            .set("bootstrap.servers", &brokers)
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("❌ Failed to create Kafka consumer");

        consumer.subscribe(&[&topic]).expect("❌ Failed to subscribe");
        info!("📡 Kafka consumer started for topic: {}", topic);

        use futures::StreamExt;
        let mut stream = consumer.stream();

        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => {
                    let payload = match msg.payload_view::<str>() {
                        Some(Ok(p)) => p,
                        _ => {
                            error!("⚠️ Skipping empty or invalid payload");
                            continue;
                        }
                    };

                    // Parse JSON → strongly typed MessageEvent
                    let mut event: MessageEvent = match serde_json::from_str(payload) {
                        Ok(e) => e,
                        Err(e) => {
                            error!("❌ JSON parse failed: {e}\n↳ {payload}");
                            continue;
                        }
                    };

                    // 🧠 Preserve formatting for assistant
                    if event.role == "assistant" {
                        // do NOT touch formatting!
                    } else {
                        // optional: cleanup plain text for user messages
                        if let Some(t) = &event.text {
                            event.text = Some(clean_user_text(t));
                        }
                    }

                    // Store in memory map
{
    let mut map = recent_messages.write().await;
    let entry = map.entry(event.chat_id.clone()).or_default();

    entry.push(event.clone());

    // keep only last 50
    if entry.len() > 50 {
        entry.remove(0);
    }
}


                    {
    // Convert event to model::Message
    let msg = crate::models::Message {
        id: event.id.clone(),
        chat_id: event.chat_id.clone(),
        session_id: event.session_id.clone(),
        device_hash: event.device_hash.clone(),
        user_id: None,       
        role: event.role.clone(),
        text: event.text.clone(),
        summary: event.summary.clone(),
        ts: event.ts,
    };

    // Persist to RocksDB
    if let Err(err) = MessageStore::save(&storage, &msg) {
        error!("❌ Failed to persist message {}: {}", msg.id, err);
    }
}


                    // Only summaries broadcast to SSE/WS listeners
                    if event.role == "summary" {
                        let _ = tx.send(event.clone());
                    }
                }

                Err(e) => error!("Kafka stream error: {e}"),
            }
        }
    });
}


pub async fn spawn_user_event_consumer(
    brokers: String,
    topic: String,
    tx_user: broadcast::Sender<MessageEvent>,
    storage: Arc<Storage>,
) {
    tokio::spawn(async move {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "chat-user-consumer")
            .set("bootstrap.servers", &brokers)
            .set("auto.offset.reset", "latest")   // ONLY new user messages
            .create()
            .expect("❌ Failed to create Kafka consumer");

        consumer.subscribe(&[&topic]).expect("❌ Failed to subscribe");
        info!("👂 Kafka USER consumer started for topic: {}", topic);

        use futures::StreamExt;
        let mut stream = consumer.stream();

        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => {
                    let payload = match msg.payload_view::<str>() {
                        Some(Ok(p)) => p,
                        _ => continue,
                    };

                    let event: MessageEvent = match serde_json::from_str(payload) {
                        Ok(e) => e,
                        Err(e) => {
                            error!("❌ USER JSON parse failed: {e}");
                            continue;
                        }
                    };

                    // Only process user messages (important!)
                    if event.role != "user" {
                        continue;
                    }

                    info!("📡 USER message: {} ({})", event.id, event.chat_id);

                    // Re-save into RocksDB for consistency
                    let db_msg = crate::models::Message {
                        id: event.id.clone(),
                        chat_id: event.chat_id.clone(),
                        session_id: event.session_id.clone(),
                        device_hash: event.device_hash.clone(),
                        user_id: event.user_id.clone(),
                        role: "user".into(),
                        text: event.text.clone(),
                        summary: None,
                        ts: event.ts,
                    };

                    if let Err(e) = MessageStore::save(&storage, &db_msg) {
                        error!("❌ Failed to save user msg {}: {}", db_msg.id, e);
                    }

                    // Send to HistoryWorker
                    let _ = tx_user.send(event);
                }

                Err(e) => error!("Kafka user-consumer stream error: {e}"),
            }
        }
    });
}


fn clean_user_text(input: &str) -> String {
    input
        .replace('\u{0000}', "")
        .trim()
        .to_string()
}
