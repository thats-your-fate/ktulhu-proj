#!/usr/bin/env python3
import socket, json, os, sys, torch, re, textwrap
from transformers import AutoTokenizer, AutoModelForCausalLM

# ────────────────────────────────────────────────
# ⚙️ Setup
# ────────────────────────────────────────────────
if len(sys.argv) < 3:
    print("Usage: worker_phi3_summary.py <SOCK_PATH> <MODEL_NAME>", flush=True)
    sys.exit(1)

SOCK_PATH, MODEL_NAME = sys.argv[1:3]
try: os.remove(SOCK_PATH)
except FileNotFoundError: pass

device = "cuda" if torch.cuda.is_available() else "cpu"
print(f"⚙️  Loading summarizer model {MODEL_NAME} on {device}...", flush=True)
tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
model = AutoModelForCausalLM.from_pretrained(
    MODEL_NAME,
    torch_dtype=torch.float16 if device == "cuda" else torch.float32,
    device_map="auto"
)
model.eval()
print(f"✅ Summarizer ready and listening on {SOCK_PATH}", flush=True)


# ────────────────────────────────────────────────
# 🧠 Smart summarization helper
# ────────────────────────────────────────────────
def summarize(raw_text: str) -> str:
    """Generate a short, robust headline even for messy or meaningless input."""
    # Normalize and strip control noise
    text = raw_text.strip()
    text = re.sub(r"http\S+", "", text)
    text = re.sub(r"[{}[\]<>`*#_]", "", text)
    text = re.sub(r"\s+", " ", text)
    text = re.sub(r"\\n", " ", text)
    text = text.strip()

    # Keep it short for the model context
    if len(text.split()) > 200:
        text = " ".join(text.split()[:200])
    preview = textwrap.shorten(text, width=120, placeholder="…")
    print(f"📥 Cleaned preview:\n{preview}\n", flush=True)

    # Build an instruct-style prompt that invites abstraction
    prompt = (
        "You are a headline generator. "
        "Write a very short 3–5 word title that describes roughly what this text is about, "
        "even if it is messy or incomplete.\n\n"
        f"Text:\n{text}\n\nHeadline:"
    )

    inputs = tokenizer(prompt, return_tensors="pt").to(device)
    with torch.inference_mode():
        output = model.generate(
            **inputs,
            max_new_tokens=16,
            temperature=0.4,
            top_p=0.9,
            do_sample=False,
            pad_token_id=tokenizer.eos_token_id,
        )

    out = tokenizer.decode(output[0], skip_special_tokens=True)
    print(f"🧩 Raw model output: {repr(out)}", flush=True)

    # Sanitize and clean the model continuation
    out = out.replace(prompt, "")
    out = re.sub(r"(?i)(headline|title|text):", "", out)
    out = re.sub(r"[^A-Za-z0-9\s\-\_]", "", out)
    out = re.sub(r"\s+", " ", out).strip()

    # Fallback safety
    if not out or len(out.split()) < 2:
        out = "User message"
    elif len(out.split()) > 8:
        out = "General message"

    print(f"✅ Final summary: {out}\n{'─'*60}", flush=True)
    return out


# ────────────────────────────────────────────────
# 🔌 UNIX socket listener
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
                req_id = req.get("id", "unknown")
                text = req.get("text", "")
                print(f"\n📨 Request received (id={req_id})", flush=True)

                summary = summarize(text)
                msg = json.dumps({"id": req_id, "summary": summary}).encode() + b"\n"
                conn.sendall(msg)

                print(f"📤 Sent summary response for id={req_id}\n", flush=True)

            except Exception as e:
                err = str(e)
                print(f"⚠️ Error handling request: {err}", flush=True)
                try:
                    conn.sendall(json.dumps({"error": err}).encode() + b"\n")
                except BrokenPipeError:
                    print("⚠️ Client disconnected early", flush=True)
