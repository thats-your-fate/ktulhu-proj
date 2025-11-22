# main.py — clean orchestration entry

import sys, os, json, socket, warnings, time
import threading
from queue import PriorityQueue
from transformers.utils import logging as hf_logging

from worker_mistral7b.inference.engine import load_engine
from worker_mistral7b.inference.dispatcher import handle_request, run_raw_generation
from worker_mistral7b.net.server import read_json_lines

# Queue for STREAMING socket only
request_queue = PriorityQueue()
worker_busy = False


# --------------------------------------------------------
# WORKER THREAD — handles ONLY streaming jobs (main.sock)
# --------------------------------------------------------
def worker_loop(tokenizer, model, device):
    global worker_busy

    while True:
        # (priority, timestamp, job)
        priority, ts, job = request_queue.get()

        worker_busy = True
        req = job["req"]
        conn = job["conn"]

        uid = req.get("id")
        prompt = req.get("text") or ""
        chat_id = req.get("chat_id") or req.get("session_id")

        print(f"▶️  Processing job {uid=} {chat_id=}", flush=True)

        try:
            # STREAMING MODE ONLY
            handle_request(
                prompt,
                conn,
                uid,
                tokenizer,
                model,
                device,
                chat_id,
                stream=True
            )

        except Exception as e:
            try:
                conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
            except:
                pass

        finally:
            try:
                conn.close()
            except:
                pass

            worker_busy = False
            request_queue.task_done()


# --------------------------------------------------------
# API SOCKET — direct inference, no queue
# --------------------------------------------------------
import os
import socket
import json
import time
import traceback
from queue import Queue
import threading
from .pipeline.memory import fetch_memory

# =========================================================
# SIMPLE NON-STREAMING API SOCKET
# =========================================================
def serve_api_socket(api_path, tokenizer, model, device):
    import time
    from worker_mistral7b.summarizer_delta import generate_state_delta

    # Cleanup old socket
    try:
        os.remove(api_path)
    except FileNotFoundError:
        pass

    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(api_path)
    server.listen(16)

    print(f"🔗 JSON Memory API listening on {api_path}", flush=True)

    while True:
        conn, _ = server.accept()

        try:
            data = conn.recv(65535)
            if not data:
                conn.close()
                continue

            # Parse JSON
            try:
                req = json.loads(data.decode("utf-8"))
            except Exception:
                conn.sendall(b'{"error":"invalid json"}\n')
                conn.close()
                continue

            # Ping
            if req.get("ping"):
                conn.sendall(b'{"pong":true}\n')
                conn.close()
                continue

            chat_id = req.get("chat_id")
            message = req.get("text") or ""
            last_ts = req.get("ts", 0)

            print(f"🧵 API inference start chat_id={chat_id}", flush=True)

            # Load old state
            try:
                old_state = fetch_memory(chat_id=chat_id)
                old_state = old_state.state if old_state else {}
            except Exception:
                old_state = {}

            # Run summarizer (returns {summary,intent,facts})
            delta = generate_state_delta(
                chat_id,
                message,
                last_ts,
                old_state,
                tokenizer,
                model,
                device
            )

            # Log what we will send
            print("📤 Sending JSON to Rust:", delta, flush=True)

            # Send exactly what summarizer returned
            conn.sendall((json.dumps(delta) + "\n").encode("utf-8"))

            print("🧵 API inference done", flush=True)

        except Exception as e:
            print("❌ API socket error:", e, flush=True)
            try:
                conn.sendall(b'{"error":"internal error"}\n')
            except:
                pass

        finally:
            try:
                conn.shutdown(socket.SHUT_RDWR)
            except:
                pass
            conn.close()


# --------------------------------------------------------
# MAIN ENTRY (STREAMING SOCKET WITH QUEUE)
# --------------------------------------------------------
def main():
    if len(sys.argv) < 3:
        print("Usage: worker_mistral7b.py <SOCK_PATH> <MODEL_NAME>", flush=True)
        sys.exit(1)

    SOCK_PATH, MODEL_NAME = sys.argv[1:3]

    # Cleanup main socket
    try:
        os.remove(SOCK_PATH)
    except FileNotFoundError:
        pass

    # Quiet transformers
    os.environ.update({
        "TRANSFORMERS_VERBOSITY": "error",
        "TOKENIZERS_PARALLELISM": "false",
        "HF_HUB_DISABLE_TELEMETRY": "1",
    })
    warnings.filterwarnings("ignore")
    hf_logging.set_verbosity_error()

    # Load model
    tokenizer, model, device = load_engine(MODEL_NAME)
    print(f"🧠 Loaded {MODEL_NAME} on {device}", flush=True)

    # Start worker thread for QUEUED STREAMING
    threading.Thread(
        target=worker_loop,
        args=(tokenizer, model, device),
        daemon=True,
    ).start()
    print("🚀 Streaming worker active", flush=True)

    # Start API socket (direct mode)
    API_PATH = SOCK_PATH + "_api"

    api_thread = threading.Thread(
        target=serve_api_socket,
        args=(API_PATH, tokenizer, model, device),
        daemon=True,
    )
    api_thread.start()

    print(f"🚀 API socket activated at {API_PATH}", flush=True)

    # ------------------------
    # MAIN STREAMING SOCKET
    # ------------------------
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
        server.bind(SOCK_PATH)
        server.listen(8)

        print(f"🔗 Listening (streaming) on {SOCK_PATH}", flush=True)

        while True:
            conn, _ = server.accept()

            # Do *not* close conn — worker thread will handle it
            try:
                for req in read_json_lines(conn):

                    uid = req.get("id")
                    prompt = req.get("text") or ""
                    chat_id = req.get("chat_id") or req.get("session_id")

                    print(
                        f"🔍 [QUEUE] {uid=} {chat_id=} text={prompt[:60]!r}",
                        flush=True,
                    )

                    # Create streaming job
                    job = {
                        "req": req,
                        "conn": conn,   # keep open for streaming
                    }

                    # Add to queue
                    position = request_queue.qsize()
                    request_queue.put((10, time.time(), job))

                    # Immediate queue response
                    response = {
                        "status": "queued",
                        "position": position,
                        "id": uid,
                        "chat_id": chat_id,
                    }

                    conn.sendall(json.dumps(response).encode() + b"\n")

            except Exception as e:
                print(f"⚠️ Streaming socket error: {e}", flush=True)
                try:
                    conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
                except:
                    pass


if __name__ == "__main__":
    main()
