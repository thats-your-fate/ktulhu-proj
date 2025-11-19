use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    ClientConfig, Message,
};
use tracing::{info, error};

use crate::models::state_delta::StateDelta;

use crate::Storage;
use crate::storage::StateStore;



pub async fn spawn_state_delta_consumer(
    brokers: String,
    topic: String,
    recent: Arc<RwLock<HashMap<String, Vec<StateDelta>>>>,
    storage: Arc<Storage>,
) {
    tokio::spawn(async move {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "state-delta-consumer")
            .set("bootstrap.servers", &brokers)
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Failed to create Kafka consumer");

        consumer.subscribe(&[&topic]).expect("Subscribe failed");
        info!("📡 Kafka consumer running for: {topic}");

        use futures::StreamExt;
        let mut stream = consumer.stream();

        while let Some(msg_result) = stream.next().await {
            match msg_result {
                Ok(msg) => {
                    let payload = match msg.payload_view::<str>() {
                        Some(Ok(p)) => p,
                        _ => continue,
                    };

                    let delta: StateDelta = match serde_json::from_str(payload) {
                        Ok(d) => d,
                        Err(e) => {
                            error!("❌ Delta JSON parse error: {e}");
                            continue;
                        }
                    };

                    // Store in memory (optional)
                    {
                        let mut map = recent.write().await;
                        let entry = map.entry(delta.chat_id.clone()).or_default();
                        entry.push(delta.clone());
                        if entry.len() > 50 {
                            entry.remove(0);
                        }
                    }

                    // Persist
                    if let Err(e) = StateStore::save(&storage, &delta) {
                        error!("❌ Failed to save delta: {e}");
                    }

                }

                Err(e) => error!("Kafka error: {e}"),
            }
        }
    });
}
