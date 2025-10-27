use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use chrono::Utc;
use rdkafka::{
    consumer::{BaseConsumer, Consumer, StreamConsumer},
    ClientConfig, Message,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSummary {
    pub chat_id: String,
    
    pub session_id: Option<String>,
    pub device_hash: Option<String>,
    pub summary: Option<String>,
    pub ts: i64,
}

pub type ChatMap = Arc<RwLock<HashMap<String, ChatSummary>>>;

/// 🧩 Spawn Kafka consumer that reads `user_messages` and broadcasts chat summaries.
pub async fn spawn_chat_summary_consumer(
    brokers: impl Into<String>,
    topic: impl Into<String>,
    tx: broadcast::Sender<ChatSummary>,
    state: ChatMap,
) {
    let brokers = brokers.into();
    let topic = topic.into();

    tokio::spawn(async move {
        // ─────────────────────────────────────────────
        // 🧠 PRELOAD RECENT SUMMARIES
        // ─────────────────────────────────────────────
        let preload_consumer: BaseConsumer = ClientConfig::new()
            .set("bootstrap.servers", &brokers)
            .set("group.id", "chat-summary-preload")
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("❌ Failed to create preload Kafka consumer");

        preload_consumer
            .subscribe(&[&topic])
            .expect("❌ Failed to subscribe for preload");

        info!("📜 Preloading up to 10 distinct summarized chats from `{}`", topic);

        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut summaries: Vec<ChatSummary> = Vec::new();

        let start_time = std::time::Instant::now();

        while start_time.elapsed() < Duration::from_secs(5) && summaries.len() < 10 {
            if let Some(result) = preload_consumer.poll(Duration::from_millis(200)) {
                if let Ok(msg) = result {
                    if let Some(Ok(payload)) = msg.payload_view::<str>() {
                        if let Some(summary) =
                            parse_summary(payload, msg.timestamp().to_millis())
                        {
                            if seen_ids.insert(summary.chat_id.clone()) {
                                summaries.push(summary);
                            }
                        }
                    }
                }
            }
        }

        // ✅ Sort by timestamp descending
        summaries.sort_by(|a, b| b.ts.cmp(&a.ts));

        {
            let mut guard = state.write().await;
            for summary in &summaries {
                guard.insert(summary.chat_id.clone(), summary.clone());
            }
        }

        info!("✅ Preloaded {} summarized chats", summaries.len());

        // ─────────────────────────────────────────────
        // 🔄 MAIN LIVE STREAMING LOOP
        // ─────────────────────────────────────────────
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "chat-summary-consumer")
            .set("bootstrap.servers", &brokers)
            .set("auto.offset.reset", "latest")
            .create()
            .expect("❌ Failed to create Kafka consumer");

        consumer
            .subscribe(&[&topic])
            .expect("❌ Failed to subscribe to live topic");

        info!("📡 Live chat summary consumer started on `{}`", topic);

        loop {
            match consumer.recv().await {
                Ok(msg) => {
                    if let Some(Ok(payload)) = msg.payload_view::<str>() {
                        if let Some(summary) =
                            parse_summary(payload, msg.timestamp().to_millis())
                        {
                            {
                                let mut guard = state.write().await;
                                guard.insert(summary.chat_id.clone(), summary.clone());
                            }

                            let _ = tx.send(summary);
                        }
                    }
                }
                Err(e) => {
                    warn!("⚠️ Kafka consumer error: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    });
}

/// 🧩 Parse only model-generated summaries
fn parse_summary(payload: &str, kafka_ts: Option<i64>) -> Option<ChatSummary> {
    if let Ok(v) = serde_json::from_str::<Value>(payload) {
        // Try flat form first
        let chat_id = v
            .get("chat_id")
            .and_then(|x| x.as_str())
            // ✅ Fallback: nested inside "message"
            .or_else(|| v.get("message")?.get("chat_id")?.as_str());

        let summary_text = v.get("summary").and_then(|x| x.as_str());

        if let (Some(chat_id), Some(summary_text)) = (chat_id, summary_text) {
            return Some(ChatSummary {
                chat_id: chat_id.to_string(),
                session_id: v
                    .get("session_id")
                    .or_else(|| v.get("message")?.get("session_id"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string()),
                device_hash: v
                    .get("device_hash")
                    .or_else(|| v.get("message")?.get("device_hash"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string()),
                summary: Some(summary_text.to_string()),
                ts: v
                    .get("ts")
                    .and_then(|x| x.as_i64())
                    .unwrap_or_else(|| kafka_ts.unwrap_or_else(|| Utc::now().timestamp_millis())),
            });
        }
    }
    None
}

