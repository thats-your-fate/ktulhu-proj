# stream_infer.py (FULLY FIXED VERSION)
import json, time, torch, requests
from threading import Thread
from transformers import TextIteratorStreamer

from .prompts import build_reasoning_prompt, rewrite_if_meta_response
from .summarizer import make_summary_with_model
from .search_classifier import ask_need_search
from .brave import run_brave_search
from worker_mistral7b.helpers.llm_helpers import generate_mistral_output

PERSISTENCE_API = "https://persistence.ktulhu.com/chat-summary/last"



def send_system(conn, uid, chat_id, message: str):
    payload = {
        "id": uid,
        "type": "system",
        "chat_id": chat_id,
        "system": message,   # 👈 FIXED
        "ts": time.time()
    }
    conn.sendall((json.dumps(payload) + "\n").encode())


# ----------------------------------------------------------
# SUMMARY FETCH
# ----------------------------------------------------------
def fetch_existing_summary(chat_id: str) -> str | None:
    if not chat_id:
        return None
    try:
        resp = requests.get(PERSISTENCE_API, timeout=2)
        resp.raise_for_status()
        data = resp.json()
        chats = data.get("chats", [])
        if not isinstance(chats, list):
            return None
        for chat in chats:
            if chat.get("chat_id") == chat_id and chat.get("summary"):
                return chat["summary"].strip()
    except Exception:
        return None


# ----------------------------------------------------------
# FIXED: extract only REAL user message text
# ----------------------------------------------------------
def extract_user_question(prompt):
    """
    Handles prompt passed as a dict or JSON string.
    Ensures we ALWAYS return the plain user question text.
    """
    if isinstance(prompt, dict):
        return prompt.get("text", "")

    # prompt is likely a JSON string
    if isinstance(prompt, str):
        try:
            obj = json.loads(prompt)
            return obj.get("text", prompt)
        except:
            return prompt

    return str(prompt)


# ----------------------------------------------------------
# BUILD SEARCH BLOCK (ONLY the user content!)
# ----------------------------------------------------------
def build_search_augmented_prompt(
    user_question: str,
    rewritten_query: str,
    search_json: dict
) -> str:
    """
    Build the section of the prompt that includes:
    - the original user question
    - the rewritten search query
    - the extracted external information
    """

    parts = []
    parts.append(f"User question: {user_question}\n")
    parts.append(f"Search query used: {rewritten_query}\n")
    parts.append("Relevant external information from web search:\n")

    # -----------------------------------
    # Case: Brave error or missing data
    # -----------------------------------
    if not search_json or search_json.get("error"):
        parts.append(f"(Search error: {search_json.get('error')})")
        parts.append("No external sources available.\n")
        parts.append("Answer based on general knowledge.")
        return "\n".join(parts)

    results = search_json.get("results") or []

    # -----------------------------------
    # Case: no results
    # -----------------------------------
    if not results:
        parts.append("(No search results found.)")
        parts.append("Answer based on general knowledge.\n")
        return "\n".join(parts)

    # -----------------------------------
    # Normal case: add results
    # -----------------------------------
    for r in results:
        title = r.get("title") or r.get("headline") or "Untitled"
        url = r.get("url", "")
        paragraphs = r.get("paragraphs") or []
        snippet = r.get("snippet") or ""

        parts.append(f"Source: {title}")
        parts.append(f"URL: {url}")

        # Prefer paragraphs
        if paragraphs:
            for p in paragraphs[:3]:
                parts.append(f"Paragraph: {p}")
        # Fallback: snippet (rare)
        elif snippet:
            parts.append(f"Paragraph: {snippet}")
        else:
            parts.append("Paragraph: (no text available)")

        parts.append("")  # blank line between sources

    parts.append("Use the information above to answer the user's question accurately.")
    return "\n".join(parts)



# ----------------------------------------------------------
# BUILD FINAL PROMPT (SYSTEM + USER + ASSISTANT)
# ----------------------------------------------------------
def build_final_prompt_with_search(raw_question, rewritten_query, search_data):
    """
    Creates a rich, structured search-augmented prompt that also includes
    a 'Sources Used' section with titles and URLs.
    """
    user_block = build_search_augmented_prompt(raw_question, rewritten_query, search_data)

    final = (
        "SYSTEM:\n"
        "You are an advanced AI assistant.\n"
        "Your job is to provide clear, comprehensive, and well-structured answers.\n"
        "Use all available external information.\n"
        "If search results are provided, analyze them and create a merged, factual summary.\n"
        "Present your answer with clear sections, bullets, and hierarchy.\n"
        "Do NOT be overly brief — provide meaningful detail when relevant.\n"
        "Avoid hallucinations. If data is missing or uncertain, acknowledge it.\n"
        "Always include a final section titled 'Sources' listing all used sources.\n"
        "For each source: provide title + URL (only if present in search results).\n"
        "Do not invent sources or URLs.\n"
        "If no valid sources exist, write: 'No external sources available.'\n\n"

        "USER:\n"
        f"{user_block}\n\n"

        "ASSISTANT:\n"
        "Please provide your answer in this structure:\n"
        "1. **Executive Summary**\n"
        "2. **Detailed Explanation**\n"
        "3. **Key Facts / Bullet Points**\n"
        "4. **If applicable: Latest Developments**\n"
        "5. **Sources** (with title + URL)\n"
        "\n"
    )
    return final




def rewrite_search_query(raw_question: str, tokenizer, model, device) -> str:
    """
    Rewrite the user question into a concise, effective web search query.
    This version matches your worker call:
        rewrite_search_query(raw_question, tokenizer, model, device)
    """

    prompt = f"""
Rewrite the following user question into a concise, effective web search query.
Rules:
- Keep only essential keywords.
- Remove pronouns, filler words, greetings, or polite phrases.
- Use correct spelling and proper names.
- Output ONLY the rewritten query, nothing else.

User question:
{raw_question}

Search query:
""".strip()

    # your existing inference helper
    response = generate_mistral_output(prompt, tokenizer, model, device)

    return response.strip()



# ----------------------------------------------------------
# MAIN STREAMING INFERENCE
# ----------------------------------------------------------
def stream_infer(prompt, conn, uid, tokenizer, model, device, chat_id=None):

    send_system(conn, uid, chat_id, "Processing request…")

    # ------------------------------------------------------
    # Step 1 — Fetch or generate summary
    # ------------------------------------------------------
    send_system(conn, uid, chat_id, "Retrieving conversation summary…")

    summary = fetch_existing_summary(chat_id) if chat_id else None
    if not summary:
        try:
            summary = make_summary_with_model(prompt, tokenizer, model, device)
        except:
            summary = "General request"

    conn.sendall(json.dumps({"id": uid, "summary": summary}).encode() + b"\n")
    send_system(conn, uid, chat_id, "Summary ready.")

    # ------------------------------------------------------
    # Step 2 — Extract only user's real question
    # ------------------------------------------------------
    send_system(conn, uid, chat_id, "Extracting user question…")
    raw_question = extract_user_question(prompt)
    print(f"🔍 Correct raw_question = {raw_question}", flush=True)
    send_system(conn, uid, chat_id, f"User question detected: {raw_question}")

    # ------------------------------------------------------
    # Step 3 — Determine if search is needed
    # ------------------------------------------------------
    send_system(conn, uid, chat_id, "Determining if a web search is needed…")
    needs_search = ask_need_search(raw_question, tokenizer, model, device)
    print(f"🔍 needs_search = {needs_search}", flush=True)

    if needs_search:
        send_system(conn, uid, chat_id, "Search required.")
    else:
        send_system(conn, uid, chat_id, "Search not required. Proceeding with reasoning.")

    rewritten_query = None
    search_data = None

    # ------------------------------------------------------
    # Step 4 — Rewrite query + run Brave search
    # ------------------------------------------------------
    if needs_search:
        send_system(conn, uid, chat_id, "Rewriting search query…")
        rewritten_query = rewrite_search_query(raw_question, tokenizer, model, device)
        send_system(conn, uid, chat_id, f"Search query rewritten: {rewritten_query}")
        print(f"⚡ Rewritten query = {rewritten_query}", flush=True)

        send_system(conn, uid, chat_id, "Running Brave search…")
        search_data = run_brave_search(rewritten_query)

        if not search_data or "results" not in search_data:
            send_system(conn, uid, chat_id, "Search returned no valid results. Falling back.")
            print("⚠️ Brave search returned no valid results. Falling back.", flush=True)
            final_prompt = build_reasoning_prompt(raw_question)
        else:
            send_system(conn, uid, chat_id, "Search complete. Processing results…")

            # stream discovered sources
            for r in search_data.get("results", []):
                title = r.get("title", "unknown source")
                send_system(conn, uid, chat_id, f"Source found: {title}")

            print("⚡ Using SEARCH-AUGMENTED final prompt", flush=True)
            send_system(conn, uid, chat_id, "Building search-augmented answer…")
            final_prompt = build_final_prompt_with_search(raw_question, rewritten_query, search_data)

    else:
        print("⚡ Using reasoning prompt (no search)", flush=True)
        send_system(conn, uid, chat_id, "Building reasoning-only answer…")
        final_prompt = build_reasoning_prompt(raw_question)

    # ------------------------------------------------------
    # Step 5 — Prompt debug dump
    # ------------------------------------------------------
    print("\n====================== FINAL PROMPT ======================")
    print(final_prompt[:2000])
    print("================== END FINAL PROMPT =====================\n")

    # SAFETY FALLBACK
    if not final_prompt:
        final_prompt = build_reasoning_prompt(raw_question)

    send_system(conn, uid, chat_id, "Generating answer…")

    # ------------------------------------------------------
    # Step 6 — Stream Mistral output
    # ------------------------------------------------------
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

    full_response = ""

    try:
        for token in streamer:
            full_response += token
            conn.sendall(json.dumps({"id": uid, "token": token}).encode() + b"\n")
    except Exception as e:
        conn.sendall(json.dumps({"id": uid, "error": str(e)}).encode() + b"\n")

    # ------------------------------------------------------
    # Step 7 — Final output
    # ------------------------------------------------------
    cleaned = rewrite_if_meta_response(full_response.strip())
    
    conn.sendall(json.dumps({"id": uid, "final": cleaned, "done": True}).encode() + b"\n")

    thread.join()
    time.sleep(0.05)