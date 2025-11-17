from ..pipeline.search_router import route_request
from .streaming import stream_answer
import json
def handle_request(prompt, conn, uid, tokenizer, model, device, chat_id):
    final_prompt, summary = route_request(
        prompt, tokenizer, model, device, conn, uid, chat_id
    )

    conn.sendall(
        json.dumps({"id": uid, "summary": summary}).encode() + b"\n"
    )

    stream_answer(conn, uid, chat_id, tokenizer, model, device, final_prompt)
