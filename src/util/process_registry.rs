use std::sync::Arc;
use tokio::{process::Child, signal, sync::Mutex};
use tracing::info;

#[derive(Default)]
pub struct ProcessRegistry {
    pub children: Mutex<Vec<Arc<Mutex<Child>>>>,
}

impl ProcessRegistry {
    pub async fn add(&self, child: Arc<Mutex<Child>>) {
        self.children.lock().await.push(child);
    }

    pub async fn kill_all(&self) {
        let mut guard = self.children.lock().await;
        for child_arc in guard.drain(..) {
            let mut child = child_arc.lock().await;
            if let Some(id) = child.id() {
                info!("🧨 Killing process PID={}", id);
                let _ = child.start_kill(); // sends SIGKILL
            }
        }
    }
}

pub async fn watch_shutdown(registry: Arc<ProcessRegistry>) {
    signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    tracing::warn!("🛑 Ctrl+C received — terminating child processes...");
    registry.kill_all().await;
    tracing::info!("✅ All child processes killed, exiting cleanly.");
}
