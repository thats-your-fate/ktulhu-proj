#!/usr/bin/env python3
import socket, json, time, os, subprocess

SOCK_PATH = "/tmp/infer.sock"

try:
    os.remove(SOCK_PATH)
except FileNotFoundError:
    pass

START_TIME = time.time()
PID = os.getpid()

def get_gpu_stats():
    """Return GPU memory usage using nvidia-smi, or None if unavailable."""
    try:
        result = subprocess.check_output(
            ["nvidia-smi", "--query-gpu=memory.used,memory.total", "--format=csv,noheader,nounits"]
        ).decode().strip().split(", ")
        used, total = map(int, result)
        return {"gpu_mem_used": used, "gpu_mem_total": total, "gpu": "0"}
    except Exception:
        return {"gpu_mem_used": None, "gpu_mem_total": None, "gpu": None}

with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
    server.bind(SOCK_PATH)
    server.listen()
    print(f"🔌 Listening on {SOCK_PATH}", flush=True)

    while True:
        conn, _ = server.accept()
        with conn:
            data = conn.recv(65536)
            if not data:
                continue

            try:
                text = data.decode().strip()

                # ✅ New: respond to control query from Rust
                if text == "__STATUS__":
                    uptime = int(time.time() - START_TIME)
                    stats = {
                        "pid": PID,
                        "uptime": uptime,
                        **get_gpu_stats(),
                        "model": "simple-reverse",
                    }
                    conn.sendall(json.dumps(stats).encode() + b"\n")
                    continue

                # ✅ Regular inference request
                req = json.loads(text)
                out = req.get("text", "")[::-1]
                resp = {
                    "id": req.get("id", "unknown"),
                    "output": out,
                    "tokens": len(out.split()),
                }

                conn.sendall(json.dumps(resp).encode() + b"\n")

            except Exception as e:
                conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
