pub mod manager;
use tokio::{process::Child, sync::Mutex};
use std::sync::Arc;
#[derive(Debug, Clone)]
pub struct Worker {
    pub name: String,
    pub socket_path: String,
    pub process: Arc<Mutex<Child>>, 
}
