#!/usr/bin/env python3
import socket, json, os, sys, torch
from threading import Thread
from transformers import AutoTokenizer, AutoModelForCausalLM, TextIteratorStreamer

if len(sys.argv) < 3:
    print("Usage: worker_mistral7b.py <SOCK_PATH> <MODEL_NAME>", flush=True)
    sys.exit(1)

SOCK_PATH, MODEL_NAME = sys.argv[1:3]
try:
    os.remove(SOCK_PATH)
except FileNotFoundError:
    pass

device = "cuda" if torch.cuda.is_available() else "cpu"
print(f"🧠 Loading {MODEL_NAME} on {device}...", flush=True)

tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
model = AutoModelForCausalLM.from_pretrained(
    MODEL_NAME,
    torch_dtype=torch.float16 if device == "cuda" else torch.float32,
    device_map="auto"
)
model.eval()
print(f"✅ Ready and listening on {SOCK_PATH}", flush=True)


def stream_infer(prompt: str, conn, uid: str):
    """Generate tokens incrementally and send JSON lines for each."""
    inputs = tokenizer(prompt, return_tensors="pt").to(device)
    streamer = TextIteratorStreamer(tokenizer, skip_prompt=True, skip_special_tokens=True)

    # Run model.generate in background thread so we can iterate stream live
    thread = Thread(
        target=model.generate,
        kwargs=dict(
            **inputs,
            streamer=streamer,
            max_new_tokens=1024,
            temperature=0.7,
            top_p=0.9,
            do_sample=True,
        ),
    )
    thread.start()

    try:
        for new_text in streamer:
            msg = json.dumps({"id": uid, "token": new_text})
            print(f"🧩 Emitting: {repr(new_text)}", flush=True)   # debug
            conn.sendall(msg.encode() + b"\n")
    except Exception as e:
        conn.sendall(json.dumps({"id": uid, "error": str(e)}).encode() + b"\n")

    # tell Node that stream is complete
    conn.sendall(json.dumps({"id": uid, "done": True}).encode() + b"\n")
    thread.join()


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
                uid = req.get("id", "")
                text = req.get("text", "")
                stream_infer(text, conn, uid)
            except Exception as e:
                conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
