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
    """
    Generate a meaningful one-sentence summary or short paragraph title
    that captures the key intent and topic of the user's message.
    """

    # ── Normalize input ─────────────────────────
    text = raw_text.strip()
    text = re.sub(r"http\S+", "", text)
    text = re.sub(r"[{}[\]<>`*#_]", "", text)
    text = re.sub(r"\s+", " ", text)
    text = re.sub(r"\\n", " ", text)
    text = text.strip()

    if not text:
        return "General request"

    # limit to ~400 tokens worth of text
    words = text.split()
    if len(words) > 400:
        text = " ".join(words[:400])

    preview = textwrap.shorten(text, width=200, placeholder="…")
    print(f"📥 Cleaned preview:\n{preview}\n", flush=True)

    # ── Build a stronger instruction ────────────
    prompt = (
        "You are an expert summarizer. "
        "Read the following user text carefully and write a clear, helpful summary "
        "in one or two sentences that captures the main intent, topic, or problem. "
        "Avoid generic titles like 'General request'. "
        "If technical, describe the domain briefly (e.g., Node.js socket issue, GPU inference, etc.).\n\n"
        f"USER TEXT:\n{text}\n\nSUMMARY:"
    )

    # ── Tokenize & generate ─────────────────────
    inputs = tokenizer(prompt, return_tensors="pt").to(device)
    with torch.inference_mode():
        output = model.generate(
            **inputs,
            max_new_tokens=80,      # ⬆️ allow longer summaries
            temperature=0.6,        # ⬆️ slightly creative but focused
            top_p=0.9,
            do_sample=True,
            repetition_penalty=1.1,
            pad_token_id=tokenizer.eos_token_id,
        )

    decoded = tokenizer.decode(output[0], skip_special_tokens=True)
    print(f"🧩 Raw model output: {repr(decoded)}", flush=True)

    # ── Extract summary portion ─────────────────
    out = decoded[len(prompt):].strip()
    out = re.sub(r"(?i)(summary|headline|title|text)[:\-]\s*", "", out)
    out = re.sub(r"\s+", " ", out).strip()

    # Clip safely
    if len(out.split()) > 60:
        out = " ".join(out.split()[:60]) + "…"

    if not out or len(out.split()) < 3:
        out = "General request"

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
