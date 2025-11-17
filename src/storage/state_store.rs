use crate::models::state_delta::StateDelta;
use crate::storage::Storage;
use rocksdb::{IteratorMode};
use anyhow::Result;

pub struct StateStore;

impl StateStore {
    pub fn save(storage: &Storage, delta: &StateDelta) -> Result<()> {
        let key = format!("state:{}", delta.ts);
        let val = serde_json::to_vec(delta)?;

        storage.db.put_cf(&storage.cf_state, key.as_bytes(), val)?;

        let tkey = format!("{:020}:{}", delta.ts, delta.chat_id);
        storage.db.put_cf(
            &storage.cf_state_by_chat,
            tkey.as_bytes(),
            delta.chat_id.as_bytes(),
        )?;

        Ok(())
    }

    /// Return the most recent delta for a chat
    pub fn load_last_for_chat(
        storage: &Storage,
        chat_id: &str,
    ) -> Result<Option<StateDelta>> {
        let iter = storage.db.iterator_cf(
            &storage.cf_state_by_chat,
            IteratorMode::End,
        );

        for item in iter {
            let (_, chat_bytes) = item?;
            let cid = String::from_utf8(chat_bytes.to_vec())?;

            if cid == chat_id {
                let key = format!("state:{}", cid);
                if let Some(raw) =
                    storage.db.get_cf(&storage.cf_state, key.as_bytes())?
                {
                    let delta: StateDelta = serde_json::from_slice(&raw)?;
                    return Ok(Some(delta));
                }
            }
        }

        Ok(None)
    }
}
