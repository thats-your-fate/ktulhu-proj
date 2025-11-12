import json, time, torch, requests
from threading import Thread
from transformers import TextIteratorStreamer
from .prompts import build_reasoning_prompt, rewrite_if_meta_response
from .summarizer import make_summary_with_model


PERSISTENCE_API = "https://persistence.ktulhu.com/chat-summary/last"

def fetch_existing_summary(chat_id: str) -> str | None:
    """Return summary text from persistence API if this chat_id already exists."""
    if not chat_id:
        return None

    try:
        resp = requests.get(PERSISTENCE_API, timeout=2)
        resp.raise_for_status()
        data = resp.json()

        chats = data.get("chats", [])
        if not isinstance(chats, list):
            print(f"⚠️ Unexpected 'chats' structure: {type(chats)}", flush=True)
            return None

        for chat in chats:
            cid = chat.get("chat_id")
            summary = chat.get("summary")
            if cid == chat_id and summary and summary.strip():
                return summary.strip()

    except Exception as e:
        print(f"⚠️ Could not fetch existing summary: {e}", flush=True)

    return None


def stream_infer(prompt, conn, uid, tokenizer, model, device, chat_id=None):
    """
    Stream model inference with persistence-aware summary generation.
    """

    final_prompt = build_reasoning_prompt(prompt)
    inputs = tokenizer(final_prompt, return_tensors="pt").to(device)
    streamer = TextIteratorStreamer(tokenizer, skip_prompt=True, skip_special_tokens=True)

    # --- 1️⃣ Try to reuse persisted summary ---
    summary = None
    if chat_id:
        summary = fetch_existing_summary(chat_id)
        if summary:
            print(f"🧩 Using existing summary from persistence for {chat_id}: {summary}", flush=True)

    # --- 2️⃣ Generate if not found ---
    if not summary:
        try:
            summary = make_summary_with_model(prompt, tokenizer, model, device)
            print(f"🧩 Generated new summary for {chat_id}: {summary}", flush=True)
        except Exception as e:
            print(f"⚠️ Summary generation failed: {e}", flush=True)
            summary = "General request"

    conn.sendall(json.dumps({"id": uid, "summary": summary}).encode() + b"\n")

    # --- 3️⃣ Start streaming inference ---
    thread = Thread(target=model.generate, kwargs=dict(
        **inputs,
        streamer=streamer,
        max_new_tokens=1024,
        temperature=0.6,
        top_p=0.9,
        do_sample=True,
        pad_token_id=tokenizer.eos_token_id,
    ))
    thread.start()

    full_response = ""
    try:
        for new_text in streamer:
            full_response += new_text
            conn.sendall(json.dumps({"id": uid, "token": new_text}).encode() + b"\n")
    except Exception as e:
        conn.sendall(json.dumps({"id": uid, "error": str(e)}).encode() + b"\n")

    cleaned = rewrite_if_meta_response(full_response.strip())
    conn.sendall(json.dumps({"id": uid, "final": cleaned, "done": True}).encode() + b"\n")

    thread.join()
    time.sleep(0.05)

