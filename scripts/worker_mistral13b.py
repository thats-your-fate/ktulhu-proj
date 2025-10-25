#!/usr/bin/env python3
import socket, json, os, sys, torch, re
from threading import Thread
from transformers import (
    AutoTokenizer,
    AutoModelForCausalLM,
    TextIteratorStreamer,
    BitsAndBytesConfig,
)

if len(sys.argv) < 2:
    print("Usage: worker_mistral13b.py <SOCK_PATH>", flush=True)
    sys.exit(1)

SOCK_PATH = sys.argv[1]
MODEL_NAME = "mistralai/Mistral-13B-Instruct-v0.1"

try:
    os.remove(SOCK_PATH)
except FileNotFoundError:
    pass

device = "cuda" if torch.cuda.is_available() else "cpu"
print(f"🧠 Loading {MODEL_NAME} (4-bit) on {device}...", flush=True)

# ────────────────────────────────────────────────
# 🧩 Quantized model load
# ────────────────────────────────────────────────
bnb_config = BitsAndBytesConfig(
    load_in_4bit=True,
    bnb_4bit_use_double_quant=True,
    bnb_4bit_quant_type="nf4",
    bnb_4bit_compute_dtype=torch.bfloat16 if torch.cuda.is_available() else torch.float32,
)

tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
model = AutoModelForCausalLM.from_pretrained(
    MODEL_NAME,
    quantization_config=bnb_config,
    device_map="auto",
)
model.eval()
print(f"✅ Ready and listening on {SOCK_PATH}", flush=True)

# ────────────────────────────────────────────────
# Prompt normalization & reasoning
# ────────────────────────────────────────────────
def normalize_prompt(raw: str) -> str:
    text = raw.strip()
    text = re.sub(r"\s+", " ", text)
    if len(text.split()) < 5 and not text.endswith("?"):
        text = f"Write a short, clear explanation about {text.lower()}."
    if text and text[0].islower():
        text = text[0].upper() + text[1:]
    return text

def build_reasoning_prompt(user_text: str) -> str:
    normalized = normalize_prompt(user_text)
    return (
        "You are an expert reasoning assistant. "
        "First interpret the user's intent precisely, then produce a single clear answer.\n\n"
        f"User request: {normalized}\n\nResponse:"
    )

# ────────────────────────────────────────────────
# Streaming inference logic
# ────────────────────────────────────────────────
def stream_infer(prompt: str, conn, uid: str):
    final_prompt = build_reasoning_prompt(prompt)
    inputs = tokenizer(final_prompt, return_tensors="pt").to(device)
    streamer = TextIteratorStreamer(tokenizer, skip_prompt=True, skip_special_tokens=True)

    thread = Thread(
        target=model.generate,
        kwargs=dict(
            **inputs,
            streamer=streamer,
            max_new_tokens=1024,
            temperature=0.5,
            top_p=0.9,
            do_sample=True,
        ),
    )
    thread.start()

    try:
        for new_text in streamer:
            msg = json.dumps({"id": uid, "token": new_text})
            conn.sendall(msg.encode() + b"\n")
    except Exception as e:
        conn.sendall(json.dumps({"id": uid, "error": str(e)}).encode() + b"\n")

    conn.sendall(json.dumps({"id": uid, "done": True}).encode() + b"\n")
    thread.join()

# ────────────────────────────────────────────────
# Socket main loop
# ────────────────────────────────────────────────
with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
    server.bind(SOCK_PATH)
    server.listen(8)
    print(f"🔗 Listening on {SOCK_PATH}", flush=True)
    while True:
        conn, _ = server.accept()
        with conn:
            try:
                data = conn.recv(65536)
                if not data:
                    continue
                req = json.loads(data.decode())
                uid = req.get("id", "")
                text = req.get("text", "")
                stream_infer(text, conn, uid)
            except Exception as e:
                conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
