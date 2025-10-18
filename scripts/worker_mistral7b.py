#!/usr/bin/env python3
import socket, json, os, sys, torch, re
from transformers import AutoTokenizer, AutoModelForCausalLM

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
print(f"✅ Ready on {SOCK_PATH}", flush=True)


def extract_all_json_arrays(text: str):
    """Find every JSON array [ ... ] inside text and parse it."""
    text = re.sub(r"```json|```", "", text, flags=re.I).strip()
    arrays = re.findall(r"(\[.*?\])", text, re.S)
    results = []
    for arr in arrays:
        try:
            data = json.loads(arr)
            if isinstance(data, list):
                for d in data:
                    if isinstance(d, dict) and "question" in d and "answer" in d:
                        results.append(d)
        except Exception:
            continue
    return results


def infer(prompt: str):
    """Run model and parse any JSON arrays in its response."""
    instruction = (
        "Generate 7 diverse question–answer pairs in pure JSON array format. "
        "Each entry must have 'question' and 'answer' fields. "
        "Return ONLY valid JSON arrays; multiple arrays are OK. No commentary.\n\n"
    )

    inputs = tokenizer(instruction + prompt, return_tensors="pt").to(device)
    with torch.no_grad():
        output = model.generate(
            **inputs,
            max_new_tokens=1300,   # or even 1536 if VRAM allows
            temperature=0.6,
            top_p=0.9,
        )

    text = tokenizer.decode(output[0], skip_special_tokens=True)
    qa_pairs = extract_all_json_arrays(text)
    if not qa_pairs:
        qa_pairs = [{"question": "Parse error", "answer": text.strip()}]
    return qa_pairs, len(text.split())


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
                output, tok = infer(text)
                msg = json.dumps(
                    {"id": uid, "output": output, "tokens": tok}, ensure_ascii=False
                ).encode() + b"\n"
                conn.sendall(msg)
            except Exception as e:
                conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
