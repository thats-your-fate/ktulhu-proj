import json
from  ..util.safe import escape_curly
def extract_user_question(prompt):
    raw = None

    if isinstance(prompt, dict):
        raw = prompt.get("text", "")

    elif isinstance(prompt, str):
        try:
            raw = json.loads(prompt).get("text", prompt)
        except Exception:
            raw = prompt

    else:
        raw = str(prompt)

    return escape_curly(raw)
