use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig,
    Message,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEvent {
    pub role: String,
    pub chat_id: String,
    pub session_id: Option<String>,
    pub device_hash: Option<String>,
    pub text: Option<String>,
    pub summary: Option<String>,
    pub ts: Option<i64>,
}

pub async fn spawn_chat_summary_consumer(
    brokers: String,
    topic: String,
    tx: broadcast::Sender<MessageEvent>,
    messages: Arc<RwLock<HashMap<String, Vec<MessageEvent>>>>,
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
                        let mut map = messages.write().await;
                        map.entry(event.chat_id.clone())
                            .or_default()
                            .push(event.clone());
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

fn clean_user_text(input: &str) -> String {
    input
        .replace('\u{0000}', "")
        .trim()
        .to_string()
}
