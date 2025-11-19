import json
from ..pipeline.search_router import route_request
from ..summarizer_delta import generate_summary_delta
from .streaming import stream_answer
from ..net.conn import send_event, send_system


def extract_user_prompt(req):
    raw = req.get("text") or req.get("prompt") or req.get("message") or ""

    if isinstance(raw, dict):
        return raw.get("text") or raw.get("prompt") or raw.get("message") or ""

    if isinstance(raw, str):
        cleaned = raw.strip()
        if cleaned.startswith("{") and cleaned.endswith("}"):
            try:
                inner = json.loads(cleaned)
                return (
                    inner.get("text")
                    or inner.get("prompt")
                    or inner.get("message")
                    or cleaned
                )
            except:
                return cleaned
        return cleaned

    return str(raw)


def handle_request(prompt, conn, uid, tokenizer, model, device, chat_id):

    send_system(conn, uid, chat_id, " Receiving request…")

    try:
        req = json.loads(prompt)
        send_system(conn, uid, chat_id, " JSON parsed.")
    except:
        send_system(conn, uid, chat_id, " Not JSON. Using raw text.")
        req = {"text": prompt}

    send_system(conn, uid, chat_id, " Extracting user message…")
    user_prompt = extract_user_prompt(req)
    send_system(conn, uid, chat_id, f"🗣️ User said: {user_prompt}")

    # ---------------------------------------------------------
    # 1. GENERATE MEMORY DELTA (intent, facts, summary)
    # ---------------------------------------------------------
    send_system(conn, uid, chat_id, " Generating memory delta…")
    delta = generate_summary_delta(
        message=user_prompt,
        old_state={},     # we no longer use Node-side memory
        tokenizer=tokenizer,
        model=model,
        device=device
    )
    send_system(conn, uid, chat_id, f"🧩Delta generated: {delta}")

    # ---------------------------------------------------------
    # 2. SEND TO NODE (Node forwards to persistence API)
    # ---------------------------------------------------------
    send_system(conn, uid, chat_id, " Sending memory delta to Node…")
    send_event(conn, {"id": uid, "state_delta": delta})
    send_system(conn, uid, chat_id, " Delta forwarded to persistence server.")

    # ---------------------------------------------------------
    # 3. BUILD FINAL PROMPT USING PERSISTENCE MEMORY
    # ---------------------------------------------------------
    final_prompt, summary = route_request(
        user_prompt,
        tokenizer,
        model,
        device,
        conn,
        uid,
        chat_id
    )

    send_event(conn, {"id": uid, "summary": summary})
    send_system(conn, uid, chat_id, f" Updated summary: {summary}")

    # ---------------------------------------------------------
    # 4. STREAM ANSWER
    # ---------------------------------------------------------
    stream_answer(conn, uid, chat_id, tokenizer, model, device, final_prompt)

