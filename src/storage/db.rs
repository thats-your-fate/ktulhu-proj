use rocksdb::{DB, Options, ColumnFamily, ColumnFamilyDescriptor};

pub struct Storage {
    pub db: DB,
    pub cf_messages: &'static ColumnFamily,
    pub cf_messages_by_time: &'static ColumnFamily,
}

impl Storage {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new("messages_by_id", Options::default()),
            ColumnFamilyDescriptor::new("messages_by_time", Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        // SAFETY: CF handles return references with static lifetime
        let cf_messages: &'static ColumnFamily = unsafe {
            std::mem::transmute(db.cf_handle("messages_by_id").unwrap())
        };

        let cf_messages_by_time: &'static ColumnFamily = unsafe {
            std::mem::transmute(db.cf_handle("messages_by_time").unwrap())
        };

        Ok(Self {
            db,
            cf_messages,
            cf_messages_by_time,
        })
    }
}
