#!/usr/bin/env python3
import socket, json, os, sys, torch, re, textwrap, warnings
from transformers import AutoTokenizer, AutoModelForCausalLM, BitsAndBytesConfig

# ────────────────────────────────────────────────
# ⚙️ Setup
# ────────────────────────────────────────────────
if len(sys.argv) < 3:
    print("Usage: worker_mistral_summary_8gb.py <SOCK_PATH> <MODEL_NAME>", flush=True)
    sys.exit(1)

SOCK_PATH, MODEL_NAME = sys.argv[1:3]
try:
    os.remove(SOCK_PATH)
except FileNotFoundError:
    pass

warnings.filterwarnings("ignore")
os.environ["TOKENIZERS_PARALLELISM"] = "false"

device = "cuda" if torch.cuda.is_available() else "cpu"
print(f"⚙️ Loading summarizer model {MODEL_NAME} on {device} (optimized for ≤8GB VRAM)...", flush=True)

# ────────────────────────────────────────────────
# 🧠 Memory-efficient model load (4-bit quantization)
# ────────────────────────────────────────────────
bnb_config = BitsAndBytesConfig(
    load_in_4bit=True,
    bnb_4bit_compute_dtype=torch.float16,
    bnb_4bit_use_double_quant=True,
    bnb_4bit_quant_type="nf4"
)

try:
    tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
    model = AutoModelForCausalLM.from_pretrained(
        MODEL_NAME,
        quantization_config=bnb_config,
        device_map="auto"
    )
except Exception as e:
    print(f"⚠️ GPU quantized load failed: {e}\n➡️ Falling back to CPU offload.", flush=True)
    tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
    model = AutoModelForCausalLM.from_pretrained(
        MODEL_NAME,
        torch_dtype=torch.float16 if torch.cuda.is_available() else torch.float32,
        low_cpu_mem_usage=True,
        device_map="auto"
    )

model.eval()
print(f"✅ Summarizer ready and listening on {SOCK_PATH}", flush=True)


# ────────────────────────────────────────────────
# 🧠 Smart summarization helper
# ────────────────────────────────────────────────
def summarize(raw_text: str) -> str:
    """Generate a concise, natural summary or short title."""
    text = raw_text.strip()
    text = re.sub(r"http\S+", "", text)
    text = re.sub(r"[{}[\]<>`*#_]", "", text)
    text = re.sub(r"\s+", " ", text)
    if not text:
        return "General request"

    words = text.split()
    if len(words) > 400:
        text = " ".join(words[:400])
    preview = textwrap.shorten(text, width=200, placeholder="…")
    print(f"📥 Preview:\n{preview}\n", flush=True)

    prompt = (
        "[INST] You are an expert assistant trained to summarize text. "
        "Write a concise, human-like one-sentence summary or short paragraph "
        "that captures the user's intent or question. "
        "Avoid meta phrases like 'The user is asking'. Focus on content.\n\n"
        f"User message: {text}\n\nSummary: [/INST]"
    )

    inputs = tokenizer(prompt, return_tensors="pt").to(device)
    with torch.inference_mode():
        output = model.generate(
            **inputs,
            max_new_tokens=80,
            temperature=0.6,
            top_p=0.9,
            do_sample=True,
            repetition_penalty=1.1,
            pad_token_id=tokenizer.eos_token_id
        )

    decoded = tokenizer.decode(output[0], skip_special_tokens=True)
    print(f"🧩 Raw output: {repr(decoded)}", flush=True)

    out = decoded.replace(prompt, "").strip()
    out = re.sub(r"(?i)(summary|headline|title|text)[:\-]\s*", "", out)
    out = re.sub(r"\s+", " ", out).strip()
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
            buffer = b""
            while True:
                chunk = conn.recv(4096)
                if not chunk:
                    break
                buffer += chunk
                if b"\n" in chunk:
                    break

            if not buffer:
                continue
            req = json.loads(buffer.decode().strip())
            req_id = req.get("id", "unknown")
            text = req.get("text", "")
            print(f"\n📨 Request received (id={req_id})", flush=True)

            summary = summarize(text)
            msg = json.dumps({"id": req_id, "summary": summary}).encode() + b"\n"
            conn.sendall(msg)
            time.sleep(0.05)
            print(f"📤 Sent summary response for id={req_id}\n", flush=True)

        except Exception as e:
            err = str(e)
            print(f"⚠️ Error handling request: {err}", flush=True)
            try:
                conn.sendall(json.dumps({"error": err}).encode() + b"\n")
            except BrokenPipeError:
                print("⚠️ Client disconnected early", flush=True)
