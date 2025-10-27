use serde::{Serialize, Deserialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use chrono::Utc;

use super::messages::MessageEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSummary {
    pub chat_id: String,
    pub summary: String,       // 🧠 short summary (AI or first user msg)
    pub text: Option<String>,  // 💬 latest message snippet
    pub ts: i64,
}

pub type ChatMap = Arc<RwLock<HashMap<String, ChatSummary>>>;

/// 🧠 Generate or update summaries from incoming Kafka `MessageEvent`
pub async fn update_summary(map: &ChatMap, event: &MessageEvent) {
    match event.role.as_str() {
        // 🧩 Model-generated summary message
        "summary" => {
            if let Some(ref text) = event.summary {
                if text.trim().len() > 3 {
                    let mut map = map.write().await;
                    let entry = map.entry(event.chat_id.clone()).or_insert(ChatSummary {
                        chat_id: event.chat_id.clone(),
                        summary: String::new(),
                        text: None,
                        ts: Utc::now().timestamp_millis(),
                    });

                    entry.summary = text.trim().chars().take(160).collect();
                    entry.ts = event.ts.unwrap_or_else(|| Utc::now().timestamp_millis());
                    tracing::info!("🧩 Updated AI summary for chat {}: {}", event.chat_id, entry.summary);
                }
            }
        }

        // 💬 Normal user messages
        "user" => {
            if let Some(ref text) = event.text {
                if text.trim().len() > 3 {
                    let mut map = map.write().await;
                    let entry = map.entry(event.chat_id.clone()).or_insert(ChatSummary {
                        chat_id: event.chat_id.clone(),
                        summary: String::new(),
                        text: None,
                        ts: Utc::now().timestamp_millis(),
                    });

                    // update latest snippet
                    entry.text = Some(format!("👤 {}", text.trim()));

                    // if summary empty → use first user message as fallback
                    if entry.summary.is_empty() {
                        entry.summary = text.trim().chars().take(120).collect();
                    }

                    entry.ts = event.ts.unwrap_or_else(|| Utc::now().timestamp_millis());
                }
            }
        }

        // 🤖 Assistant messages → only update latest text
        "assistant" => {
            if let Some(ref text) = event.text {
                if text.trim().len() > 3 {
                    let mut map = map.write().await;
                    let entry = map.entry(event.chat_id.clone()).or_insert(ChatSummary {
                        chat_id: event.chat_id.clone(),
                        summary: String::new(),
                        text: None,
                        ts: Utc::now().timestamp_millis(),
                    });

                    entry.text = Some(format!("🤖 {}", text.trim()));
                    entry.ts = event.ts.unwrap_or_else(|| Utc::now().timestamp_millis());
                }
            }
        }

        // ⚙️ “system” messages (New chat, status etc.)
        "system" => {
            if let Some(ref text) = event.summary {
                let mut map = map.write().await;
                let entry = map.entry(event.chat_id.clone()).or_insert(ChatSummary {
                    chat_id: event.chat_id.clone(),
                    summary: text.trim().chars().take(120).collect(),
                    text: None,
                    ts: event.ts.unwrap_or_else(|| Utc::now().timestamp_millis()),
                });
                tracing::info!("🛠️ System summary set for chat {}: {}", event.chat_id, entry.summary);
            }
        }

        // 🚫 Unknown / ignored roles
        other => {
            tracing::warn!("⚠️ Ignoring unrecognized role: {}", other);
        }
    }
}
