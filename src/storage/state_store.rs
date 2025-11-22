use crate::models::state_delta::StateDelta;
use crate::storage::Storage;
use rocksdb::{IteratorMode};
use anyhow::Result;

pub struct StateStore;

impl StateStore {
    pub fn save(storage: &Storage, delta: &StateDelta) -> Result<()> {
        let ts = delta.ts;
        let chat_id = &delta.chat_id;

        // Primary key
        let key = format!("state:{}:{}", chat_id, ts);
        let val = serde_json::to_vec(delta)?;

        storage.db.put_cf(&storage.cf_state, key.as_bytes(), val)?;

        // Secondary index: sorted by timestamp
        let tkey = format!("{:020}:{}", ts, chat_id);
        storage.db.put_cf(&storage.cf_state_by_chat, tkey.as_bytes(), b"")?;

        Ok(())
    }

pub fn load_last_for_chat(
    storage: &Storage,
    chat_id: &str,
) -> Result<Option<StateDelta>> {
    let prefix = format!(":{}", chat_id);

    let iter = storage.db.iterator_cf(
        &storage.cf_state_by_chat,
        IteratorMode::End,
    );

    for item in iter {
        let (key_bytes, _) = item?;
        let key_str = String::from_utf8(key_bytes.to_vec())?;

        if key_str.ends_with(&prefix) {
            // key format: "<20-digit ts>:<chat_id>"
            let parts: Vec<&str> = key_str.split(':').collect();
            let ts_str = parts[0];

            // parse padded ts → integer
            let ts: i64 = ts_str.parse().unwrap_or(0);

            // now build correct RocksDB key
            let full_key = format!("state:{}:{}", chat_id, ts);

            if let Some(raw) =
                storage.db.get_cf(&storage.cf_state, full_key.as_bytes())?
            {
                let delta: StateDelta = serde_json::from_slice(&raw)?;
                return Ok(Some(delta));
            }
        }
    }

    Ok(None)
}

}
