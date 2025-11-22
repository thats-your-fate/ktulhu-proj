import requests

MEMORY_API = "https://persistence.ktulhu.com/state-delta/history"


def fetch_memory(chat_id: str):
    if not chat_id:
        return {"intents": [], "facts": [], "summary": ""}

    try:
        res = requests.get(f"{MEMORY_API}/{chat_id}", timeout=2)
        if not res.ok:
            return {"intents": [], "facts": [], "summary": ""}

        data = res.json()
        deltas = data.get("history", [])

        intents = []
        facts = []
        summary = ""

        for delta in deltas:
            # 1) direct fields
            if delta.get("user_intent"):
                intents.append(delta["user_intent"])

            if delta.get("new_facts"):
                facts.extend(delta["new_facts"])

            if delta.get("message_summary"):
                summary = delta["message_summary"]

            # 2) merged state payload
            state = delta.get("state")
            if state:
                intents.extend(state.get("intents", []))
                facts.extend(state.get("facts", []))
                if state.get("summary"):
                    summary = state["summary"]

        return {
            "intents": intents,
            "facts": facts,
            "summary": summary
        }

    except Exception as e:
        print("Memory fetch error:", e)
        return {"intents": [], "facts": [], "summary": ""}
