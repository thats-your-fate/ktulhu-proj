import requests

PERSISTENCE_API = "https://persistence.ktulhu.com/chat-summary/last"

def fetch_summary(chat_id: str):
    if not chat_id:
        return None
    try:
        resp = requests.get(PERSISTENCE_API, timeout=2).json()
        for chat in resp.get("chats", []):
            if chat.get("chat_id") == chat_id:
                return (chat.get("summary") or "").strip() or None
    except:
        return None
