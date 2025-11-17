# main.py — clean orchestration entry

import sys, os, json, socket, warnings
import torch
from transformers.utils import logging as hf_logging

from worker_mistral7b.inference.engine import load_engine
from worker_mistral7b.inference.dispatcher import handle_request
from worker_mistral7b.net.server import read_json_lines


def main():
    if len(sys.argv) < 3:
        print("Usage: worker_mistral7b.py <SOCK_PATH> <MODEL_NAME>", flush=True)
        sys.exit(1)

    SOCK_PATH, MODEL_NAME = sys.argv[1:3]

    # Cleanup old socket
    try:
        os.remove(SOCK_PATH)
    except FileNotFoundError:
        pass

    # Quiet transformers
    os.environ.update({
        "TRANSFORMERS_VERBOSITY": "error",
        "TOKENIZERS_PARALLELISM": "false",
        "HF_HUB_DISABLE_TELEMETRY": "1",
    })
    warnings.filterwarnings("ignore")
    hf_logging.set_verbosity_error()

    # Load model engine
    tokenizer, model, device = load_engine(MODEL_NAME)
    print(f"🧠 Loaded {MODEL_NAME} on {device}", flush=True)

    # Socket server
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
        server.bind(SOCK_PATH)
        server.listen(8)

        print(f"🔗 Listening on {SOCK_PATH}", flush=True)

        while True:
            conn, _ = server.accept()

            with conn:
                try:
                    for req in read_json_lines(conn):
                        uid = req.get("id")
                        prompt = req.get("text") or ""
                        chat_id = req.get("chat_id") or req.get("session_id")

                        print(
                            f"🔍 {uid=} {chat_id=} text={prompt[:60]!r}",
                            flush=True,
                        )

                        # Dispatch full request to inference handler
                        handle_request(
                            prompt,
                            conn,
                            uid,
                            tokenizer,
                            model,
                            device,
                            chat_id,
                        )

                except Exception as e:
                    err = {"error": str(e)}
                    try:
                        conn.sendall(json.dumps(err).encode() + b"\n")
                    except:
                        pass
                    print(f"⚠️ Worker crashed: {e}", flush=True)


if __name__ == "__main__":
    main()
