
from ..net.conn import send_system
from ..net.persistence import fetch_summary
from worker_mistral7b.summarizer import make_summary_with_model
from .extractor import extract_user_question

from .memory import fetch_memory
import json
from .search_utils import rewrite_query, perform_search
from .routing_logic import should_search

from worker_mistral7b.prompts import build_prompt_with_memory, build_narrow_with_memory
from worker_mistral7b.search_classifier import ask_need_search
from worker_mistral7b.summary_manager import get_conversation_summary
from worker_mistral7b.general_classifier import ask_need_broad_answer
def route_request(user_prompt, tokenizer, model, device, conn, uid, chat_id):

    # ------------------------------------------------------
    # Step 1 — Fetch summary using the restored logic
    # ------------------------------------------------------
    send_system(conn, uid, chat_id, "Computing conversation summary…")

    summary = get_conversation_summary(
        chat_id,
        user_prompt,
        tokenizer,
        model,
        device
    )

    conn.sendall(json.dumps({"id": uid, "summary": summary}).encode() + b"\n")
    send_system(conn, uid, chat_id, "Summary ready.")

    send_system(conn, uid, chat_id, "Extracting user question…")
    question = extract_user_question(user_prompt)

    send_system(conn, uid, chat_id, "Fetching conversation memory…")
    memory = fetch_memory(chat_id)



    send_system(conn, uid, chat_id, "Classifying answer style…")
    needs_broad = ask_need_broad_answer(question, tokenizer, model, device)


    send_system(conn, uid, chat_id, "Determining if web search is needed…")
    needs_search = ask_need_search(question, tokenizer, model, device)

    if not needs_search:
        send_system(conn, uid, chat_id, "Search not required.")
        final_prompt = build_prompt_with_memory(
            question=question,
            state=memory,
            search_block=None
        )
        return final_prompt, summary

    send_system(conn, uid, chat_id, "Rewriting search query…")
    rewritten = rewrite_query(question, tokenizer, model, device)

    send_system(conn, uid, chat_id, f"Running Brave search: {rewritten}")
    search_results = perform_search(rewritten)

    print(search_results)

    
    if needs_broad:
        send_system(conn, uid, chat_id, "Broad answer requested.")
        final_prompt = build_prompt_with_memory(
        question=question,
        state=memory,
        search_block=search_results
    )

    else:
        send_system(conn, uid, chat_id, "Narrow answer requested.") 
        final_prompt = build_narrow_with_memory(
        question=question,
        state=memory,
        search_block=search_results
    )
    return final_prompt, summary
