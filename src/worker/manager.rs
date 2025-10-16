use crate::config::AppConfig;
use crate::worker::Worker;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

pub async fn spawn_workers_from_config(cfg: &AppConfig) -> Vec<Worker> {
    let mut workers = Vec::new();
    let scripts_dir = PathBuf::from("/srv/mistral/ktulhu-proj/scripts");

    for w in &cfg.workers {
        // 🧠 Auto-select script based on model name
        let script_name = if w.model.to_lowercase().contains("mistral") {
            "worker_mistral7b.py"
        } else if w.model.to_lowercase().contains("phi")
            || w.model.to_lowercase().contains("falcon")
            || w.model.to_lowercase().contains("openchat")
        {
            "worker_mediumlm.py"
        } else if w.model.to_lowercase().contains("bert")
            || w.model.to_lowercase().contains("roberta")
            || w.model.to_lowercase().contains("tox")
        {
            "inference_worker.py"
        } else {
            tracing::warn!(
                "⚠️ Could not auto-detect model family for `{}`; using default worker.",
                w.model
            );
            "inference_worker.py"
        };

        let script_path = scripts_dir.join(script_name);

        // cleanup old socket
        let _ = std::fs::remove_file(&w.socket);

        tracing::info!(
            "🚀 Spawning `{}` -> `{}` (model: {}, GPU {})",
            w.name,
            script_path.display(),
            w.model,
            w.gpu
        );

        let mut cmd = Command::new("/srv/mistral/ktulhu-proj/.venv/bin/python");
        cmd.arg(&script_path)
            .arg(&w.socket)
            .arg(&w.model)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .current_dir(&scripts_dir)
            .env("CUDA_VISIBLE_DEVICES", &w.gpu);

        let process = cmd.spawn()
            .unwrap_or_else(|e| panic!("❌ Failed to spawn {}: {}", w.name, e));

        workers.push(Worker {
            name: w.name.clone(),
            socket_path: w.socket.clone(),
            process,
        });
    }

    tracing::info!("✅ Spawned {} workers", workers.len());
    workers
}
