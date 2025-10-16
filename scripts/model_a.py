#!/usr/bin/env python3
import socket
import json
import os
import torch
from transformers import AutoTokenizer, AutoModelForSequenceClassification

SOCK_PATH = "/tmp/infer_a.sock"

# Clean up previous socket
try:
    os.remove(SOCK_PATH)
except FileNotFoundError:
    pass

# ---------- Model Initialization (load once) ----------
print("🧠 Loading model...", flush=True)
MODEL_NAME = "distilbert-base-uncased-finetuned-sst-2-english"
tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
model = AutoModelForSequenceClassification.from_pretrained(MODEL_NAME)
model.eval()
print(f"✅ Model loaded: {MODEL_NAME}", flush=True)

# ---------- Helper: predict ----------
def infer(text: str):
    """Run model inference and return output string + structured fields."""
    inputs = tokenizer(text, return_tensors="pt", truncation=True, max_length=512)
    with torch.no_grad():
        logits = model(**inputs).logits
        probs = torch.nn.functional.softmax(logits, dim=-1).squeeze()
    confidence, cls_idx = torch.max(probs, dim=-1)
    verdict = "YES" if cls_idx.item() == 1 else "NO"

    # Create a human-readable output and structured metadata
    output_text = f"Verdict: {verdict} (confidence={confidence.item():.2f})"

    return {
        "verdict": verdict,
        "confidence": round(confidence.item(), 3),
        "output": output_text,
        "tokens": len(text.split()),
    }

# ---------- Socket server loop ----------
with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
    server.bind(SOCK_PATH)
    server.listen()
    print(f"✅ Model A listening on {SOCK_PATH}", flush=True)

    while True:
        conn, _ = server.accept()
        with conn:
            data = conn.recv(65536)
            if not data:
                continue

            try:
                req = json.loads(data.decode())
                text = req.get("text", "")
                result = infer(text)

                resp = {
                    "id": req.get("id", "unknown"),
                    "output": result["output"],
                    "tokens": result["tokens"],
                    "verdict": result["verdict"],
                    "confidence": result["confidence"],
                }

                conn.sendall(json.dumps(resp).encode() + b"\n")

            except Exception as e:
                conn.sendall(
                    json.dumps({"error": str(e)}).encode() + b"\n"
                )
