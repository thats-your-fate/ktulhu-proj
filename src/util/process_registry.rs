use std::sync::Arc;
use std::time::Duration;
use tokio::{process::Child, signal, sync::Mutex, time};
use tracing::{info, warn};

#[derive(Default)]
pub struct ProcessRegistry {
    pub children: Mutex<Vec<Arc<Mutex<Child>>>>,
}

impl ProcessRegistry {
    pub async fn add(&self, child: Arc<Mutex<Child>>) {
        self.children.lock().await.push(child);
    }

    /// Gracefully terminates all registered child processes.
    pub async fn kill_all(&self) {
        let mut guard = self.children.lock().await;
        if guard.is_empty() {
            info!("ℹ️ No registered child processes to kill.");
            return;
        }

        warn!("🛑 Terminating {} child processes...", guard.len());

        // Request kill for all
        for child_arc in &*guard {
            let mut child = child_arc.lock().await;
            if let Some(pid) = child.id() {
                info!("💀 Sending kill signal to PID={}", pid);
            }
            let _ = child.start_kill(); // Sends SIGKILL on Unix, TerminateProcess on Windows
        }

        // Give a short grace period for cleanup
        time::sleep(Duration::from_millis(300)).await;

        guard.clear();
        info!("✅ All child processes terminated.");
    }
}

/// Watches for Ctrl+C (SIGINT) and triggers full cleanup.
pub async fn watch_shutdown(registry: Arc<ProcessRegistry>) {
    signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    warn!("🛑 Ctrl+C received — terminating child processes...");
    registry.kill_all().await;
    info!("✅ Clean shutdown complete.");
}
