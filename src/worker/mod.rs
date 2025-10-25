pub mod manager;
pub use manager::spawn_workers_from_config;
use tokio::{process::Child, sync::Mutex};
use std::sync::Arc;
#[derive(Debug)]
pub struct Worker {
    pub name: String,
    pub socket_path: String,
    pub process: Arc<Mutex<Child>>, 
}

// manual clone (ignores the Child handle)
impl Clone for Worker {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            socket_path: self.socket_path.clone(),
            process: panic!("Worker::clone() called — process handle cannot be cloned"),
        }
    }
}
