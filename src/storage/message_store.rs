// src/storage/message_store.rs
use rocksdb::{IteratorMode, Direction};
use crate::models::Message;
use crate::storage::Storage;
use std::collections::HashSet;
pub struct MessageStore;

impl MessageStore {
    pub fn save(storage: &Storage, msg: &Message) -> anyhow::Result<()> {
        let key = format!("msg:{}", msg.id);
        let val = serde_json::to_vec(msg)?;

        // primary store
        storage.db.put_cf(&storage.cf_messages, key.as_bytes(), val)?;

        // time index
        let tkey = format!("{:020}:{}", msg.ts, msg.id);
        storage.db.put_cf(
            &storage.cf_messages_by_time,
            tkey.as_bytes(),
            msg.id.as_bytes(),
        )?;

        Ok(())
    }

    /// Load all messages for a given chat_id, sorted by time
    pub fn load_thread(storage: &Storage, chat_id: &str) -> anyhow::Result<Vec<Message>> {
        let mut out = Vec::new();

        // naive scan first (you can optimize later)
        let iter = storage.db.iterator_cf(
            &storage.cf_messages_by_time,
            IteratorMode::From(b"00000000000000000000:", Direction::Forward),
        );

        for item in iter {
            let (_, msg_id_bytes) = item?;
            let id = String::from_utf8(msg_id_bytes.to_vec())?;
            let mkey = format!("msg:{id}");

            if let Some(raw) = storage.db.get_cf(&storage.cf_messages, mkey.as_bytes())? {
                let msg: Message = serde_json::from_slice(&raw)?;
                if msg.chat_id == chat_id {
                    out.push(msg);
                }
            }
        }

        out.sort_by_key(|m| m.ts);
        Ok(out)
    }

        pub fn list_chat_ids(storage: &Storage) -> anyhow::Result<Vec<String>> {
        let mut set = HashSet::new();

        let iter = storage.db.iterator_cf(
            &storage.cf_messages,
            rocksdb::IteratorMode::Start,
        );

        for item in iter {
            let (_, raw) = item?;
            let msg: Message = serde_json::from_slice(&raw)?;
            set.insert(msg.chat_id);
        }

        Ok(set.into_iter().collect())
    }


        pub fn load_last_summary(
        storage: &Storage,
        chat_id: &str,
    ) -> anyhow::Result<Option<Message>> {
        let mut last: Option<Message> = None;

        let iter = storage.db.iterator_cf(
            &storage.cf_messages_by_time,
            rocksdb::IteratorMode::End, // start from newest
        );

        for entry in iter {
            let (_, id_bytes) = entry?;
            let id = String::from_utf8(id_bytes.to_vec())?;
            let key = format!("msg:{id}");

            if let Some(raw) = storage.db.get_cf(&storage.cf_messages, key.as_bytes())? {
                let msg: Message = serde_json::from_slice(&raw)?;
                if msg.chat_id == chat_id && msg.role == "summary" {
                    last = Some(msg);
                    break;
                }
            }
        }

        Ok(last)
    }
}
