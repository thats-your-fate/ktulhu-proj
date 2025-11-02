import json, time
from threading import Thread
from transformers import TextIteratorStreamer
from .prompts import build_reasoning_prompt, rewrite_if_meta_response
from .summarizer import make_summary_with_model

def stream_infer(prompt, conn, uid, tokenizer, model, device):
    final_prompt = build_reasoning_prompt(prompt)
    inputs = tokenizer(final_prompt, return_tensors="pt").to(device)
    streamer = TextIteratorStreamer(tokenizer, skip_prompt=True, skip_special_tokens=True)

    try:
        summary = make_summary_with_model(prompt, tokenizer, model, device)
        conn.sendall(json.dumps({"id": uid, "summary": summary}).encode() + b"\n")
        print(f"🧩 Emitted early summary: {summary}", flush=True)
    except Exception as e:
        print(f"⚠️ Summary generation failed: {e}", flush=True)

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
