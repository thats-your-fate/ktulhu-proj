# Ktulhu Inference Platform

**Ktulhu** is a lightweight, bare-metal multi-model inference system built in **Rust + Python**.  
It combines the **safety and performance of Rust** for orchestration with the **flexibility of Hugging Face models** in Python — all running locally via fast **Unix-socket IPC**.

---

## Features

### Modular Multi-Model Architecture

Run multiple independent Python workers, each hosting a different model:

-  `distilbert-base-uncased-finetuned-sst-2-english` → fast classification  
-  `microsoft/phi-2` → mid-tier generative model  
-  `mistralai/Mistral-7B-Instruct-v0.2` → full-scale reasoning

Rust manages all workers, tracks their state, and provides unified access through a **WebSocket API**.

---

###  Fast Local Communication

- **Internal communication:** Unix sockets (`/tmp/infer_*.sock`)  
- **External access:** WebSocket API on port `8080`  
  - `/ws/infer` – send inference requests  
  - `/ws/status` – check system and worker status

Each worker exchanges simple **JSON messages**, keeping the system easy to extend and integrate.

---

##  Tech Stack

-  **Rust** – Orchestration layer, worker management, WebSocket server  
-  **Python** – Model runtime with Hugging Face Transformers  
-  **Unix Sockets** – High-speed local IPC  
-  **WebSocket** – Unified external API

---

##  Roadmap

- [ ] Add streaming inference support  
- [ ] Docker-based deployment option  
- [ ] REST API gateway (optional)  
- [ ] Model hot-reloading without downtime

---

##  License

MIT © 2025 — Built with ❤️ by the thats-your-fate
