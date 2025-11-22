use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    time::{timeout, Duration},
};
use tracing::{debug, error, info, warn};
use tokio::sync::{RwLock, broadcast};
use serde_json::json;

use crate::{
    MessageEvent,
    models::Message,
    models::state_delta::{ChatState},
    storage::{Storage, state_store::StateStore},
    StateDelta,
};

const SOCK: &str = "/tmp/infer_b.sock_api";

pub struct HistoryWorker {
    storage: Arc<Storage>,
    recent_state: Arc<RwLock<HashMap<String, Vec<StateDelta>>>>,
}

impl HistoryWorker {
    pub fn new(storage: Arc<Storage>, recent_state: Arc<RwLock<HashMap<String, Vec<StateDelta>>>>) -> Self {
        Self { storage, recent_state }
    }

    // =====================================================
    // 🧠 Call python inference server — NEW STRUCTURE
    // =====================================================
async fn infer(&self, chat_id: &str, msg: &Message) -> StateDelta {
    let connect_timeout = Duration::from_secs(3);
    let read_timeout = Duration::from_secs(60);

    info!("🔌 [infer] Connecting to Python for msg {}", msg.id);

    // Connect
    let stream = match timeout(connect_timeout, UnixStream::connect(SOCK)).await {
        Ok(Ok(s)) => s,
        _ => {
            warn!("⚠ [infer] Python unreachable → fallback");
            return Self::fallback(chat_id, msg, "connect_error");
        }
    };

    let mut reader = BufReader::new(stream);

    // Build request
    let req = serde_json::json!({
        "chat_id": chat_id,
        "id": msg.id,
        "ts": msg.ts,
        "text": msg.text,
    });

    let mut buf = serde_json::to_vec(&req).unwrap();
    buf.push(b'\n');

    // Send
    if let Err(e) = reader.get_mut().write_all(&buf).await {
        warn!("⚠ [infer] Python write error: {}", e);
        return Self::fallback(chat_id, msg, "write_error");
    }
    debug!("➡️ [infer] sent to python: {}", msg.id);

    // Receive one line
    let mut line = String::new();
    let n = match timeout(read_timeout, reader.read_line(&mut line)).await {
        Ok(Ok(n)) => n,
        _ => {
            warn!("⚠ [infer] timeout/EOF");
            return Self::fallback(chat_id, msg, "timeout_or_eof");
        }
    };

    if n == 0 {
        warn!("⚠ [infer] EOF from python");
        return Self::fallback(chat_id, msg, "eof");
    }

    // LOG RAW PYTHON OUTPUT
    error!("🐍 RAW PYTHON OUTPUT BEGIN ==========================");
    error!("{}", line);
    error!("🐍 RAW PYTHON OUTPUT END   ==========================");

    // Try parse
    let parsed: serde_json::Value = match serde_json::from_str(&line) {
        Ok(v) => {
            debug!("🧩 [infer] parsed JSON = {}", v);
            v
        }
        Err(e) => {
            error!("❌ [infer] JSON parse error: {}", e);
            return Self::fallback(chat_id, msg, "parse_error");
        }
    };

    // Extract fields
    let summary = parsed.get("summary")
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let intent  = parsed.get("intent").cloned();
    let facts   = parsed.get("facts")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    // LOG FINAL EXTRACTED FIELDS
    error!("📝 [infer] extracted summary = {:?}", summary);
    error!("🧭 [infer] extracted intent  = {:?}", intent);
    error!("📦 [infer] extracted facts   = {:?}", facts);

    StateDelta {
        chat_id: chat_id.to_string(),
        ts: chrono::Utc::now().timestamp_millis(),
        last_processed_ts: msg.ts,

        summary,
        intent,
        facts,

        state: None,
    }
}


    // =====================================================
    // Fallback
    // =====================================================
    fn fallback(chat_id: &str, msg: &Message, summary: impl Into<String>) -> StateDelta {
        StateDelta {
            chat_id: chat_id.to_string(),
            ts: chrono::Utc::now().timestamp_millis(),
            last_processed_ts: msg.ts,
            summary: Some(summary.into()),
            intent: None,
            facts: vec![],
            state: None,
        }
    }

    // =====================================================
    // Merge python semantic memory into cumulative ChatState
    // =====================================================
    fn merge(&self, base: Option<ChatState>, delta: &StateDelta) -> ChatState {
        let mut s = base.unwrap_or(ChatState {
            intents: vec![],
            facts: vec![],
            summary: None,
        });

        // merge intent
        if let Some(intent) = &delta.intent {
            if !s.intents.contains(intent) {
                s.intents.push(intent.clone());
            }
        }

        // merge facts
        for f in &delta.facts {
            if !s.facts.contains(f) {
                s.facts.push(f.clone());
            }
        }

        // latest summary overwrites
        if let Some(summary) = &delta.summary {
            s.summary = Some(summary.clone());
        }

        s
    }

    // =====================================================
    // 🚀 Kafka-driven worker loop
    // =====================================================
    pub async fn run(self: Arc<Self>, mut rx: broadcast::Receiver<MessageEvent>) {
        info!("🚀 HistoryWorker started");

        while let Ok(event) = rx.recv().await {
            if event.role != "user" {
                continue;
            }

            let chat = event.chat_id.clone();

            // Load last state
            let last_state = StateStore::load_last_for_chat(&self.storage, &chat)
                .unwrap_or(None);

            let last_ts = last_state.as_ref().map(|s| s.last_processed_ts).unwrap_or(0);

            if event.ts <= last_ts {
                continue;
            }

            let msg = Message {
                id: event.id.clone(),
                chat_id: chat.clone(),
                session_id: event.session_id.clone(),
                device_hash: event.device_hash.clone(),
                user_id: event.user_id.clone(),
                role: event.role.clone(),
                text: event.text.clone(),
                summary: event.summary.clone(),
                ts: event.ts,
            };

            // Extract memory from python
            let delta = self.infer(&chat, &msg).await;

            // merge with old memory
            let merged = self.merge(last_state.and_then(|s| s.state), &delta);

            // wrap FINAL delta to store
let final_delta = StateDelta {
    chat_id: chat.clone(),
    ts: chrono::Utc::now().timestamp_millis(),
    last_processed_ts: msg.ts,

    summary: delta.summary.clone(),
    intent: delta.intent.clone(),
    facts: delta.facts.clone(),

    state: Some(merged.clone()),
};



            // Save to RocksDB
            if let Err(e) = StateStore::save(&self.storage, &final_delta) {
                error!("❌ save error {}: {}", chat, e);
            }

            // Save to memory
            {
                let mut map = self.recent_state.write().await;
                map.entry(chat).or_default().push(final_delta);
            }
        }
    }
}
