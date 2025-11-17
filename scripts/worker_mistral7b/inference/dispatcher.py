import json
from ..pipeline.search_router import route_request
from ..summarizer_delta import generate_summary_delta
from .streaming import stream_answer
from ..net.conn import send_event

def handle_request(prompt, conn, uid, tokenizer, model, device, chat_id):
    # Extract old context JSON from request
    # Parent caller sends req["context_state"]
    try:
        old_state = json.loads(prompt).get("context_state", {})
    except:
        old_state = {}

    # Generate delta summary for the new message
    delta = generate_summary_delta(
        message=prompt,
        old_state=old_state,
        tokenizer=tokenizer,
        model=model,
        device=device
    )

    # Send delta summary to Node
    send_event(conn, {
        "id": uid,
        "state_delta": delta
    })

    # Build final prompt (search / reasoning logic)
    final_prompt, summary = route_request(
        prompt,
        tokenizer,
        model,
        device,
        conn,
        uid,
        chat_id
    )

    # Send normal summary as before
    send_event(conn, {
        "id": uid,
        "summary": summary
    })

    # Now stream the answer to the user
    stream_answer(conn, uid, chat_id, tokenizer, model, device, final_prompt)
