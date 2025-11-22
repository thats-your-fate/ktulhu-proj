# worker_mistral7b/summary_manager.py

import requests
from worker_mistral7b.summarizer import make_summary_with_model

PERSISTENCE_API = "https://persistence.ktulhu.com/chat-summary/last"


def get_conversation_summary(chat_id: str, user_prompt, tokenizer, model, device):
    """
    Fetch summary from the persistence API list.
    If a summary for the given chat_id exists → use it.
    Otherwise → generate a fresh summary using the model.
    """

    # No chat ID → always generate
    if not chat_id:
        try:
            return make_summary_with_model(user_prompt, tokenizer, model, device)
        except:
            return "General request"

    # -----------------------------
    # 1) Fetch summary list
    # -----------------------------
    try:
        resp = requests.get(PERSISTENCE_API, timeout=2)
        resp.raise_for_status()
        data = resp.json()

        chats = data.get("chats", [])
        if isinstance(chats, list):
            for chat in chats:
                if chat.get("chat_id") == chat_id and chat.get("summary"):
                    return chat["summary"].strip()
    except Exception:
        pass

    # -----------------------------
    # 2) Fallback → generate fresh summary
    # -----------------------------
    try:
        return make_summary_with_model(user_prompt, tokenizer, model, device)
    except:
        return "General request"
