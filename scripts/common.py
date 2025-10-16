#!/usr/bin/env python3
import os, socket, json, traceback
import torch
from transformers import AutoTokenizer, AutoModelForSequenceClassification

def load_model(model_name: str):
    print(f"🧠 Loading model {model_name}...", flush=True)
    tokenizer = AutoTokenizer.from_pretrained(model_name)
    model = AutoModelForSequenceClassification.from_pretrained(model_name)
    model.eval()
    print(f"✅ Model loaded: {model_name}", flush=True)
    return tokenizer, model

def run_server(sock_path: str, infer_fn):
    try:
        os.remove(sock_path)
    except FileNotFoundError:
        pass

    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
        server.bind(sock_path)
        server.listen()
        print(f"✅ Listening on {sock_path}", flush=True)

        while True:
            conn, _ = server.accept()
            with conn:
                try:
                    data = conn.recv(65536)
                    if not data:
                        continue
                    req = json.loads(data.decode())
                    text = req.get("text", "")
                    result = infer_fn(text)

                    resp = {
                        "id": req.get("id", "unknown"),
                        "output": result["output"],
                        "tokens": result["tokens"],
                        "verdict": result["verdict"],
                        "confidence": result["confidence"],
                    }
                    conn.sendall(json.dumps(resp).encode() + b"\n")

                except Exception as e:
                    traceback.print_exc()
                    conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
