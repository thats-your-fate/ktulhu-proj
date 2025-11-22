import json
from ..pipeline.search_router import route_request
from .streaming import stream_answer
from ..net.conn import send_event, send_system


def run_raw_generation(prompt, tokenizer, model, device):
    inputs = tokenizer(prompt, return_tensors="pt").to(device)

    output = model.generate(
        **inputs,
        max_new_tokens=512,
        do_sample=True,
        temperature=0.7,
        top_p=0.9,
        eos_token_id=tokenizer.eos_token_id,
        pad_token_id=tokenizer.eos_token_id,
    )

    text = tokenizer.decode(output[0], skip_special_tokens=True)
    return text


def generate_full_answer(tokenizer, model, device, prompt):
    inputs = tokenizer(prompt, return_tensors="pt").to(device)

    output_ids = model.generate(
        **inputs,
        max_new_tokens=512,
        do_sample=True,
        temperature=0.7,
        top_p=0.9,
        eos_token_id=tokenizer.eos_token_id,
        pad_token_id=tokenizer.eos_token_id,
    )

    text = tokenizer.decode(output_ids[0], skip_special_tokens=True)
    return text



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


def handle_request(prompt, conn, uid, tokenizer, model, device, chat_id, stream=True):


    send_system(conn, uid, chat_id, " Receiving request…")

    try:
        req = json.loads(prompt)
        send_system(conn, uid, chat_id, " JSON parsed.")
    except:
        send_system(conn, uid, chat_id, " Not JSON. Using raw text.")
        req = {"text": prompt}

    send_system(conn, uid, chat_id, " Extracting user message…")
    user_prompt = extract_user_prompt(req)


    # 3. FINAL PROMPT
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

    # 4. STREAM ANSWER (original behavior)
    stream_answer(conn, uid, chat_id, tokenizer, model, device, final_prompt)
