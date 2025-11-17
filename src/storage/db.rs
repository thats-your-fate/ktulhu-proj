use rocksdb::{DB, Options, ColumnFamily, ColumnFamilyDescriptor};

pub struct Storage {
    pub db: DB,

    pub cf_messages: &'static ColumnFamily,
    pub cf_messages_by_time: &'static ColumnFamily,

    // NEW FOR STATE DELTAS
    pub cf_state: &'static ColumnFamily,
    pub cf_state_by_chat: &'static ColumnFamily,
}

impl Storage {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Add ALL column families here
        let cfs = vec![
            ColumnFamilyDescriptor::new("messages_by_id", Options::default()),
            ColumnFamilyDescriptor::new("messages_by_time", Options::default()),

            // NEW CFs
            ColumnFamilyDescriptor::new("state", Options::default()),
            ColumnFamilyDescriptor::new("state_by_chat", Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        // SAFETY: RocksDB handles use a static lifetime internally
        let cf_messages: &'static ColumnFamily = unsafe {
            std::mem::transmute(db.cf_handle("messages_by_id").unwrap())
        };

        let cf_messages_by_time: &'static ColumnFamily = unsafe {
            std::mem::transmute(db.cf_handle("messages_by_time").unwrap())
        };

        let cf_state: &'static ColumnFamily = unsafe {
            std::mem::transmute(db.cf_handle("state").unwrap())
        };

        let cf_state_by_chat: &'static ColumnFamily = unsafe {
            std::mem::transmute(db.cf_handle("state_by_chat").unwrap())
        };

        Ok(Self {
            db,
            cf_messages,
            cf_messages_by_time,

            // NEW
            cf_state,
            cf_state_by_chat,
        })
    }
}
