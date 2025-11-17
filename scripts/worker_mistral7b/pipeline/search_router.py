from worker_mistral7b.helpers.llm_helpers import generate_mistral_output
from .prompts import build_final_prompt_with_search
from worker_mistral7b.summarizer import make_summary_with_model
from .extractor import extract_user_question
from ..net.persistence import fetch_summary
from ..net.conn import send_system
from ..brave import run_brave_search
from ..search_classifier import ask_need_search
from ..prompts import build_reasoning_prompt
from ..util.safe import escape_curly
def rewrite_search_query(question, tokenizer, model, device):
    prompt = f"""
Rewrite the following user question into a concise search query...
{question}
Search query:
"""
    return generate_mistral_output(prompt, tokenizer, model, device).strip()

def route_request(prompt, tokenizer, model, device, conn, uid, chat_id):
    send_system(conn, uid, chat_id, "Extracting user question…")

    question = extract_user_question(prompt) 

    # Summary
    summary = fetch_summary(chat_id) or make_summary_with_model(
        prompt, tokenizer, model, device
    )

    # Decide whether to search
    send_system(conn, uid, chat_id, "Determining if web search is needed…")
    needs_search = ask_need_search(question, tokenizer, model, device)

    if not needs_search:
        send_system(conn, uid, chat_id, "Search not required.")
        return build_reasoning_prompt(question), summary

    send_system(conn, uid, chat_id, "Rewriting search query…")
    rewritten = rewrite_search_query(question, tokenizer, model, device)

    send_system(conn, uid, chat_id, f"Running Brave search: {rewritten}")
    search_data = run_brave_search(rewritten)

    final_prompt = (
        build_final_prompt_with_search(question, rewritten, search_data)
        if search_data else
        build_reasoning_prompt(question)
    )

    return final_prompt, summary
