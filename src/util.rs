use std::time::{SystemTime, UNIX_EPOCH};
pub mod process_registry;
pub fn uuid_like() -> String {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("req-{}", ns)
}
