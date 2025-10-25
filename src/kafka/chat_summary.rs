use std::{collections::HashMap, sync::Arc, time::Duration};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig, Message,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast, RwLock};
use chrono::Utc;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSummary {
    pub chat_id: String,
    pub session_id: Option<String>,
    pub device_hash: Option<String>,
    pub preview: Option<String>,
    pub ts: i64,
}

pub type ChatMap = Arc<RwLock<HashMap<String, ChatSummary>>>;

/// Spawn Kafka consumer that reads user_messages and broadcasts chat summaries.
pub async fn spawn_chat_summary_consumer(
    brokers: impl Into<String>,
    topic: impl Into<String>,
    tx: broadcast::Sender<ChatSummary>,
    state: ChatMap,
) {
    let brokers = brokers.into();
    let topic = topic.into();

    tokio::spawn(async move {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "chat-summary-consumer")
            .set("bootstrap.servers", &brokers)
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Failed to create Kafka consumer");

        consumer.subscribe(&[&topic])
            .expect("Failed to subscribe to topic");

        info!("📡 Chat summary consumer listening on topic `{}`", topic);

        loop {
            match consumer.recv().await {
                Ok(msg) => {
                    if let Some(Ok(payload)) = msg.payload_view::<str>() {
                        if let Ok(v) = serde_json::from_str::<Value>(payload) {
                            if let Some(inner) = v.get("message") {
                                if let Some(chat_id) = inner.get("chat_id").and_then(|x| x.as_str()) {
                                    let summary = ChatSummary {
                                        chat_id: chat_id.to_string(),
                                        session_id: inner.get("session_id").and_then(|x| x.as_str()).map(|s| s.to_string()),
                                        device_hash: inner.get("device_hash").and_then(|x| x.as_str()).map(|s| s.to_string()),
                                        preview: inner.get("text").and_then(|x| x.as_str()).map(|s| s.to_string()),
                                        ts: Utc::now().timestamp_millis(),
                                    };

                                    // Update memory map
                                    {
                                        let mut guard = state.write().await;
                                        guard.insert(chat_id.to_string(), summary.clone());
                                    }

                                    let _ = tx.send(summary);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Kafka error: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });
}
