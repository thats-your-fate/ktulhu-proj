#!/usr/bin/env python3
import socket, json, os, sys, torch, re, time
from threading import Thread
from transformers import AutoTokenizer, AutoModelForCausalLM, TextIteratorStreamer

# ────────────────────────────────────────────────
# ⚙️  Args & setup
# ────────────────────────────────────────────────
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

# ────────────────────────────────────────────────
# 🧹 Prompt utilities
# ────────────────────────────────────────────────
def normalize_prompt(raw: str) -> str:
    """Clean whitespace, fix casing, remove duplicates, etc."""
    text = raw.strip()
    text = re.sub(r"\s+", " ", text)
    # If it looks like a sentence fragment, add context
    if len(text.split()) < 5 and not text.endswith("?"):
        text = f"Write a short, clear explanation about {text.lower()}."
    # Capitalize first letter if missing
    if text and text[0].islower():
        text = text[0].upper() + text[1:]
    return text


def build_reasoning_prompt(user_text: str) -> str:
    """
    Add an invisible reasoning layer.
    This gives the model clarity before generation, without showing it to the user.
    """
    normalized = normalize_prompt(user_text)
    meta_prefix = (
        "You are an expert reasoning assistant. "
        "First interpret the user's intent precisely, clarify ambiguous parts internally, "
        "then produce a single clear, helpful answer.\n\n"
        f"User request: {normalized}\n\nResponse:"
    )
    return meta_prefix

# ────────────────────────────────────────────────
# 🧠 Summarizer helper (fixed prompt + fallback)
# ────────────────────────────────────────────────
def make_summary_with_model(user_text: str) -> str:
    """Ask the external Phi-2 summarizer via UNIX socket."""
    try:
        sock_path = "/tmp/infer_c.sock"
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.connect(sock_path)
            payload = json.dumps({"id": "summary_req", "text": user_text}).encode()
            client.sendall(payload)
            data = client.recv(4096)
            if not data:
                return "General request"
            res = json.loads(data.decode())
            return res.get("summary", "General request")
    except Exception as e:
        print(f"⚠️ Summary service failed: {e}", flush=True)
        return "General request"


# ────────────────────────────────────────────────
# 🔁 Streaming inference
# ────────────────────────────────────────────────
def stream_infer(prompt: str, conn, uid: str):
    final_prompt = build_reasoning_prompt(prompt)
    inputs = tokenizer(final_prompt, return_tensors="pt").to(device)
    streamer = TextIteratorStreamer(tokenizer, skip_prompt=True, skip_special_tokens=True)

    # 🧠 Emit short summary BEFORE streaming
    try:
        summary = make_summary_with_model(prompt)
        conn.sendall(json.dumps({"id": uid, "summary": summary}).encode() + b"\n")
        print(f"🧩 Emitted early summary: {summary}", flush=True)
    except Exception as e:
        print(f"⚠️ Failed to make summary: {e}", flush=True)

    # 🔄 Start streaming generation
    thread = Thread(
        target=model.generate,
        kwargs=dict(
            **inputs,
            streamer=streamer,
            max_new_tokens=1024,
            temperature=0.5,
            top_p=0.9,
            do_sample=True,
            pad_token_id=tokenizer.eos_token_id,
        ),
    )
    thread.start()

    try:
        for new_text in streamer:
            msg = json.dumps({"id": uid, "token": new_text})
            conn.sendall(msg.encode() + b"\n")
    except Exception as e:
        conn.sendall(json.dumps({"id": uid, "error": str(e)}).encode() + b"\n")

    # ✅ Wrap up
    conn.sendall(json.dumps({"id": uid, "done": True}).encode() + b"\n")
    thread.join()
    time.sleep(0.05)  # small flush delay


# ────────────────────────────────────────────────
# 🧩 Socket listener
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
