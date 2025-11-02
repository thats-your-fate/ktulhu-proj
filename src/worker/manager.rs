use crate::config::{AppConfig, NodeProcessConfig};
use crate::worker::Worker;
use crate::util::process_registry::ProcessRegistry;

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info, warn};
use tokio::sync::Mutex;

/// 🧩 Spawns all Python workers from config
pub async fn spawn_workers_from_config(cfg: &AppConfig, registry: Arc<ProcessRegistry>) -> Vec<Worker> {
    let mut workers = Vec::new();
    let scripts_dir = PathBuf::from("/srv/mistral/ktulhuUpgarade/scripts");

    for w in &cfg.workers {
        let script_name = w.script.clone().unwrap_or_else(|| {
            if w.model.to_lowercase().contains("mistral") {
                "worker_mistral7b.py".to_string()
            } else if w.model.to_lowercase().contains("phi")
                || w.model.to_lowercase().contains("falcon")
                || w.model.to_lowercase().contains("openchat")
            {
                "worker_mediumlm.py".to_string()
            } else {
                "inference_worker.py".to_string()
            }
        });

        let script_path = scripts_dir.join(&script_name);
        let _ = std::fs::remove_file(&w.socket);

        info!(
            "🚀 Spawning `{}` using `{}` (model: {}, GPU {})",
            w.name, script_path.display(), w.model, w.gpu
        );

        let python_bin = cfg
            .python_bin
            .clone()
            .unwrap_or_else(|| "/srv/mistral/ktulhuUpgarade/.venv/bin/python".to_string());

        let mut cmd = Command::new(python_bin);
        cmd.arg(&script_path)
            .arg(&w.socket)
            .arg(&w.model)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .current_dir(&scripts_dir)
            .env("CUDA_VISIBLE_DEVICES", &w.gpu);

        let child = Arc::new(Mutex::new(
            cmd.spawn().unwrap_or_else(|e| panic!("❌ Failed to spawn {}: {}", w.name, e)),
        ));
        registry.add(child.clone()).await;

        workers.push(Worker {
            name: w.name.clone(),
            socket_path: w.socket.clone(),
            process: child,
        });
    }

    info!("✅ Spawned {} workers", workers.len());
    workers
}

/// 🧠 Optional Node.js orchestration process
pub async fn spawn_node_process_from_config(cfg: &AppConfig, registry: Arc<ProcessRegistry>) {
    if let Some(node_cfg) = &cfg.node_process {
        spawn_node_process(node_cfg, registry).await;
    } else {
        tracing::info!("ℹ️ No Node.js process configured.");
    }
}

/// 🧩 Launch Node.js proxy/orchestrator
pub async fn spawn_node_process(cfg: &NodeProcessConfig, registry: Arc<ProcessRegistry>) {
    let cwd = cfg.nodecwd.clone().unwrap_or_else(|| "/srv/mistral/ktulhuUpgarade/node/dist".into());
    let script_path = PathBuf::from(&cwd).join(&cfg.script);

    info!(
        "🧠 Spawning Node.js process `{}` -> `{}` with sockets: {:?}",
        cfg.name, script_path.display(), cfg.sockets
    );

    let local_node = PathBuf::from("./node-v22-linux-x64/bin/node");
    let mut cmd = if local_node.exists() {
        Command::new(local_node)
    } else {
        Command::new("node")
    };

    cmd.arg(&script_path)
        .args(&cfg.sockets)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(&cwd);

    // 🧩 Pass custom environment variables
    if let Some(envs) = &cfg.env {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }

    // 🌐 Inject tunnel from config.json as PUBLIC_TUNNEL
    if let Some(tunnel) = &cfg.tunnel {
        cmd.env("PUBLIC_TUNNEL", tunnel);
        info!("🌐 Injected PUBLIC_TUNNEL={}", tunnel);
    } else {
        warn!("⚠️ No tunnel configured for Node process — will use quick/ephemeral mode.");
    }

    let child = Arc::new(Mutex::new(
        cmd.spawn().unwrap_or_else(|e| {
            panic!("❌ Failed to spawn Node.js process `{}`: {}", cfg.name, e)
        }),
    ));
    registry.add(child.clone()).await;

    let name = cfg.name.clone();
    let child_for_wait = child.clone();

    tokio::spawn(async move {
        let mut guard = child_for_wait.lock().await;
        match guard.wait().await {
            Ok(status) if status.success() => info!("✅ Node `{}` exited normally", name),
            Ok(status) => tracing::error!("❌ Node `{}` exited with {:?}", name, status),
            Err(e) => tracing::error!("❌ Failed to wait on `{}`: {}", name, e),
        }
    });
}
