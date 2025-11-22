    use crate::config::{AppConfig, NodeProcessConfig};
    use crate::worker::Worker;
    use crate::util::process_registry::ProcessRegistry;

    use std::path::PathBuf;
    use std::process::Stdio;
    use std::sync::Arc;
    use tokio::process::Command;
    use tokio::sync::Mutex;




    ///  Spawns all Python workers from config
pub async fn spawn_workers_from_config(
    cfg: &AppConfig,
    registry: Arc<ProcessRegistry>
) -> Vec<Worker> {

    let mut workers = Vec::new();

    let root = PathBuf::from(&cfg.root);

    let scripts_dir = root.join("scripts");



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

        let python_bin = cfg.python_bin.clone()
            .unwrap_or_else(|| root.join(".venv/bin/python").display().to_string());

        // remove old socket
        let _ = std::fs::remove_file(&w.socket);

        tracing::info!(
            "🚀 Spawning worker {} using Python={} Script={} Model={}",
            w.name,
            python_bin,
            script_path.display(),
            w.model
        );

        let mut cmd = Command::new(&python_bin);
        cmd.arg(&script_path)
            .arg(&w.socket)
            .arg(&w.model)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .current_dir(&scripts_dir)
            .env("CUDA_VISIBLE_DEVICES", &w.gpu);

        let child = Arc::new(Mutex::new(
            cmd.spawn().unwrap_or_else(|e| {
                panic!("❌ Failed to spawn worker {}: {}", w.name, e)
            })
        ));

        registry.add(child.clone()).await;

        workers.push(Worker {
            name: w.name.clone(),
            socket_path: w.socket.clone(),
            process: child,
        });
    }

    workers
}


pub async fn spawn_node_process_from_config(
    app: &AppConfig,
    registry: Arc<ProcessRegistry>
) {
    if let Some(node_cfg) = &app.node_process {
        spawn_node_process(node_cfg, app, registry).await;
    } else {
        tracing::info!("ℹ️ No Node.js process configured.");
    }
}


pub async fn spawn_node_process(
    cfg: &NodeProcessConfig,
    app: &AppConfig,
    registry: Arc<ProcessRegistry>,
){

let root = PathBuf::from(&app.root);

let cwd = cfg.nodecwd
    .as_ref()
    .map(PathBuf::from)
    .unwrap_or_else(|| root.join("node/dist"));

let script_path = cwd.join(&cfg.script);


    let local_node = root.join("node-v22-linux-x64/bin/node");

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

    // custom env vars from config
    if let Some(envs) = &cfg.env {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }

    // tunnel injection
    if let Some(tunnel) = &cfg.tunnel {
        cmd.env("PUBLIC_TUNNEL", tunnel);
        tracing::info!("🌐 PUBLIC_TUNNEL={}", tunnel);
    }

    let child = Arc::new(Mutex::new(
        cmd.spawn().unwrap_or_else(|e| {
            panic!("❌ Failed to spawn Node.js process {}: {}", cfg.name, e)
        })
    ));

    registry.add(child.clone()).await;

    let name = cfg.name.clone();

    tokio::spawn(async move {
        let mut guard = child.lock().await;
        match guard.wait().await {
            Ok(status) if status.success() =>
                tracing::info!("🟢 Node `{}` exited normally", name),
            Ok(status) =>
                tracing::error!("❌ Node `{}` exited with status: {:?}", name, status),
            Err(e) =>
                tracing::error!("❌ Failed to wait on Node `{}`: {}", name, e),
        }
    });
}

