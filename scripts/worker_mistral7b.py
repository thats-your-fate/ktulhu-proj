#!/usr/bin/env python3
import socket, json, os, sys, torch, re, time, warnings
from threading import Thread
from transformers import AutoTokenizer, AutoModelForCausalLM, TextIteratorStreamer
from transformers.utils import logging as hf_logging

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

# 🧹 Quiet mode: suppress irrelevant transformer / torch logs
os.environ.update({
    "TRANSFORMERS_VERBOSITY": "error",
    "TOKENIZERS_PARALLELISM": "false",
    "HF_HUB_DISABLE_TELEMETRY": "1",
    "TORCH_CPP_LOG_LEVEL": "ERROR"
})
warnings.filterwarnings("ignore")
hf_logging.set_verbosity_error()

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
    """Create the instruct-style reasoning prompt for generation."""
    normalized = normalize_prompt(user_text)
    meta_prefix = (
        "You are a friendly conversational reasoning assistant. "
        "Always respond directly to the user in first person. "
        "Do not describe the user’s actions or thoughts. "
        "Just answer clearly and conversationally.\n\n"
        f"User request: {normalized}\n\nAssistant:"
    )
    return meta_prefix


# ────────────────────────────────────────────────
# 🧠 Meta-response rewriter (to fix detached outputs)
# ────────────────────────────────────────────────
def rewrite_if_meta_response(text: str) -> str:
    """
    Detect detached or report-like responses (e.g. 'The user is asking...')
    and rewrite them into natural chat tone.
    """
    lowered = text.lower().strip()

    # Quick pattern detection
    if lowered.startswith("the user is") or lowered.startswith("the user wants") \
       or lowered.startswith("the user has") or lowered.startswith("the user seeks"):
        # Replace patterns
        text = re.sub(r"(?i)^the user is (asking|inquiring|seeking|wondering)", "You're", text)
        text = re.sub(r"(?i)^the user wants to know", "You’d like to know", text)
        text = re.sub(r"(?i)^the user", "You", text)
        if not text.endswith("."):
            text += "."
        text += " Here's what I can tell you:"
    return text


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
            temperature=0.6,
            top_p=0.9,
            do_sample=True,
            pad_token_id=tokenizer.eos_token_id,
        ),
    )
    thread.start()

    full_response = ""
    try:
        for new_text in streamer:
            full_response += new_text
            msg = json.dumps({"id": uid, "token": new_text})
            conn.sendall(msg.encode() + b"\n")
    except Exception as e:
        conn.sendall(json.dumps({"id": uid, "error": str(e)}).encode() + b"\n")

    # ✅ Wrap up
    cleaned = full_response.strip()
    cleaned = rewrite_if_meta_response(cleaned)
    conn.sendall(json.dumps({"id": uid, "final": cleaned, "done": True}).encode() + b"\n")
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
