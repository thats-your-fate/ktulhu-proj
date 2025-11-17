import json

def extract_user_question(prompt):
    if isinstance(prompt, dict):
        return prompt.get("text", "")
    if isinstance(prompt, str):
        try:
            return json.loads(prompt).get("text", prompt)
        except:
            return prompt
    return str(prompt)
