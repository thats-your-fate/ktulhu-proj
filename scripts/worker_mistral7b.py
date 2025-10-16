#!/usr/bin/env python3
import socket, json, os, sys, torch
from transformers import AutoTokenizer, AutoModelForCausalLM

if len(sys.argv) < 3:
    print("Usage: worker_mistral7b.py <SOCK_PATH> <MODEL_NAME>", flush=True)
    sys.exit(1)

SOCK_PATH, MODEL_NAME = sys.argv[1:3]

try: os.remove(SOCK_PATH)
except FileNotFoundError: pass

device = "cuda" if torch.cuda.is_available() else "cpu"
print(f"🧠 Loading {MODEL_NAME} on {device}...", flush=True)
tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
model = AutoModelForCausalLM.from_pretrained(
    MODEL_NAME,
    torch_dtype=torch.float16 if device == "cuda" else torch.float32,
    device_map="auto"
)
model.eval()
print(f"✅ Ready on {SOCK_PATH}", flush=True)

def infer(prompt: str):
    inputs = tokenizer(prompt, return_tensors="pt").to(device)
    with torch.no_grad():
        output = model.generate(
            **inputs,
            max_new_tokens=256,
            temperature=0.7,
            top_p=0.9
        )
    text = tokenizer.decode(output[0], skip_special_tokens=True)
    return {"output": text, "tokens": len(text.split())}

with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
    server.bind(SOCK_PATH)
    server.listen(8)
    print(f"🔌 Listening on {SOCK_PATH}", flush=True)
    while True:
        conn, _ = server.accept()
        with conn:
            try:
                data = conn.recv(65536)
                if not data:
                    continue
                req = json.loads(data.decode())
                out = infer(req.get("text", ""))
                msg = json.dumps({"id": req.get("id", ""), **out}).encode() + b"\n"
                try:
                    conn.sendall(msg)
                except BrokenPipeError:
                    print("[WARN] Client disconnected before send", flush=True)
            except Exception as e:
                try:
                    conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
                except BrokenPipeError:
                    print("[WARN] Client disconnected during error send", flush=True)

