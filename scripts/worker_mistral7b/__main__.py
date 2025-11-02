import sys, os, torch, json, socket, warnings, re, time
from threading import Thread
from transformers import AutoTokenizer, AutoModelForCausalLM, TextIteratorStreamer
from transformers.utils import logging as hf_logging

from .prompts import normalize_prompt, build_reasoning_prompt, rewrite_if_meta_response
from .summarizer import make_summary_with_model
from .inference import stream_infer

def main():
    if len(sys.argv) < 3:
        print("Usage: worker_mistral7b.py <SOCK_PATH> <MODEL_NAME>", flush=True)
        sys.exit(1)

    SOCK_PATH, MODEL_NAME = sys.argv[1:3]
    try:
        os.remove(SOCK_PATH)
    except FileNotFoundError:
        pass

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

    # ─ Socket server loop ─
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
                    stream_infer(text, conn, uid, tokenizer, model, device)
                except Exception as e:
                    conn.sendall(json.dumps({"error": str(e)}).encode() + b"\n")
