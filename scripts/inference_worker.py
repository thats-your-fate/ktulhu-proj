#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import socket
import json
import os
import sys
import torch
from transformers import AutoTokenizer, AutoModelForSequenceClassification

if len(sys.argv) < 3:
    print("Usage: inference_worker.py <SOCK_PATH> <MODEL_NAME>", flush=True)
    sys.exit(1)

SOCK_PATH = sys.argv[1]
MODEL_NAME = sys.argv[2]

# cleanup old socket
try:
    os.remove(SOCK_PATH)
except FileNotFoundError:
    pass

print(f"🧠 Loading model {MODEL_NAME}...", flush=True)
tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
model = AutoModelForSequenceClassification.from_pretrained(MODEL_NAME)
model.eval()
device = "cuda" if torch.cuda.is_available() else "cpu"
model.to(device)
print(f"✅ Model ready on socket {SOCK_PATH} (device={device})", flush=True)


def infer(text: str):
    """Run inference on one text snippet."""
    if not text.strip():
        return {
            "verdict": "NO",
            "confidence": 0.0,
            "output": "Empty input.",
            "tokens": 0,
        }

    inputs = tokenizer(text, return_tensors="pt", truncation=True, max_length=512).to(device)
    with torch.no_grad():
        logits = model(**inputs).logits
        probs = torch.nn.functional.softmax(logits, dim=-1).squeeze()

    confidence, cls_idx = torch.max(probs, dim=-1)
    verdict = "YES" if cls_idx.item() == 1 else "NO"

    return {
        "verdict": verdict,
        "confidence": round(confidence.item(), 3),
        "output": f"Verdict: {verdict} (confidence={confidence.item():.2f})",
        "tokens": len(text.split()),
    }


def handle_client(conn):
    """Handle one client connection safely."""
    try:
        data = conn.recv(65536)
        if not data:
            return

        req = json.loads(data.decode())
        text = req.get("text", "")
        req_id = req.get("id", "unknown")

        print(f"🟢 Request {req_id}: {len(text)} chars", flush=True)
        result = infer(text)
        resp = {"id": req_id, **result}

        try:
            conn.sendall(json.dumps(resp).encode() + b"\n")
            print(f"🧠 Responded → {result['verdict']} ({result['confidence']})", flush=True)
        except BrokenPipeError:
            print("⚠️  Client disconnected before response was sent.", flush=True)

    except json.JSONDecodeError:
        print("⚠️  Received invalid JSON request.", flush=True)
        try:
            conn.sendall(json.dumps({"error": "invalid_json"}).encode() + b"\n")
        except BrokenPipeError:
            pass
    except Exception as e:
        print(f"❌ Exception: {e}", flush=True)
        try:
            conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
        except BrokenPipeError:
            pass


with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
    server.bind(SOCK_PATH)
    server.listen(16)  # allow short bursts of incoming connections
    print(f"✅ Listening on {SOCK_PATH}", flush=True)

    while True:
        try:
            conn, _ = server.accept()
            with conn:
                handle_client(conn)
        except KeyboardInterrupt:
            print("🛑 Stopping worker (Ctrl+C pressed).", flush=True)
            break
        except Exception as e:
            print(f"[WARN] Socket accept error: {e}", flush=True)
