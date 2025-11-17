from threading import Thread
from transformers import TextIteratorStreamer
from ..net.conn import send_event, send_system
from ..prompts import rewrite_if_meta_response

def stream_answer(conn, uid, chat_id, tokenizer, model, device, final_prompt):
    send_system(conn, uid, chat_id, "Generating answer…")

    inputs = tokenizer(final_prompt, return_tensors="pt").to(device)
    streamer = TextIteratorStreamer(tokenizer, skip_prompt=True, skip_special_tokens=True)

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

    full = ""
    for tok in streamer:
        full += tok
        send_event(conn, {"id": uid, "token": tok})

    cleaned = rewrite_if_meta_response(full.strip())
    send_event(conn, {"id": uid, "final": cleaned, "done": True})

    thread.join()
