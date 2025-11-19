import requests

MEMORY_API = "https://persistence.ktulhu.com/state-delta/history"

def fetch_memory(chat_id: str):
    if not chat_id:
        return {"intents": [], "facts": [], "summary": ""}

    try:
        res = requests.get(f"{MEMORY_API}/{chat_id}", timeout=2)
        if not res.ok:
            return {"intents": [], "facts": [], "summary": ""}

        history = res.json().get("history", [])
        intents, facts, summary = [], [], ""

        for entry in history:
            delta = entry.get("state_delta", {})

            if "user_intent" in delta:
                intents.append(delta["user_intent"])

            if "message_summary" in delta:
                summary = delta["message_summary"]

            if "fact" in delta:
                facts.append(delta["fact"])

        return {
            "intents": intents,
            "facts": facts,
            "summary": summary
        }

    except Exception as e:
        print("Memory fetch error:", e)
        return {"intents": [], "facts": [], "summary": ""}
