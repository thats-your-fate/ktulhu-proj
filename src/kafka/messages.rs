use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig,
    Message, // for payload_view
};
use serde::{Deserialize, Serialize};
use tracing::{info, error};

/// Unified Kafka message event from Node proxy
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

/// Spawns async Kafka consumer that stores all messages in memory
/// and broadcasts `MessageEvent`s (only summaries) via WebSocket.
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
                    // decode payload
                    let payload_opt = msg.payload_view::<str>();
                    let payload = match payload_opt {
                        Some(Ok(p)) => p,
                        _ => {
                            error!("⚠️ Skipping invalid or empty payload");
                            continue;
                        }
                    };

                    // parse as JSON
                    let event: MessageEvent = match serde_json::from_str(payload) {
                        Ok(e) => e,
                        Err(e) => {
                            error!("❌ JSON parse failed: {e}\n↳ {}", truncate(payload, 400));
                            continue;
                        }
                    };

                    // clean up text
                    let clean = clean_text(event.text.clone());
                    let mut event = event;
                    if !clean.is_empty() {
                        event.text = Some(clean);
                    }

                    // store in memory
                    {
                        let mut map = messages.write().await;
                        map.entry(event.chat_id.clone())
                            .or_default()
                            .push(event.clone());
                    }

                    // broadcast only summaries
                    if event.role == "summary" {
                        let _ = tx.send(event.clone());
                    }
                }

                Err(e) => error!("Kafka stream error: {e}"),
            }
        }
    });
}

/// ✂️ Utility: safely truncate very long payloads for logs
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...[+{} chars]", &s[..max_len], s.len() - max_len)
    } else {
        s.to_string()
    }
}

/// 🧼 Utility: clean malformed JSON/Markdown text safely
fn clean_text(input: Option<String>) -> String {
    if let Some(t) = input {
        let trimmed = t.trim();

        // 🧩 Try to parse JSON first — if valid, keep it as-is
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                return trimmed.to_string();
            }
        }

        // 🧼 Otherwise, sanitize plain text (no JSON semantics)
        trimmed
            .replace('\r', " ")
            .replace('\n', " ")
            .replace('\t', " ")
            .replace('\u{0000}', "")
            .chars()
            .filter(|c| !c.is_control() || *c == '\n')
            .collect::<String>()
            .trim()
            .to_string()
    } else {
        String::new()
    }
}

