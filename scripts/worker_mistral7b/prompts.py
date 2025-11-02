import re

def normalize_prompt(raw: str) -> str:
    text = raw.strip()
    text = re.sub(r"\s+", " ", text)
    if len(text.split()) < 5 and not text.endswith("?"):
        text = f"Write a short, clear explanation about {text.lower()}."
    if text and text[0].islower():
        text = text[0].upper() + text[1:]
    return text

def build_reasoning_prompt(user_text: str) -> str:
    normalized = normalize_prompt(user_text)
    return (
        "You are a friendly conversational reasoning assistant. "
        "Always respond directly to the user in first person. "
        "Do not describe the user’s actions or thoughts. "
        "Just answer clearly and conversationally.\n\n"
        f"User request: {normalized}\n\nAssistant:"
    )

def rewrite_if_meta_response(text: str) -> str:
    lowered = text.lower().strip()
    if lowered.startswith(("the user is", "the user wants", "the user has", "the user seeks")):
        text = re.sub(r"(?i)^the user is (asking|inquiring|seeking|wondering)", "You're", text)
        text = re.sub(r"(?i)^the user wants to know", "You’d like to know", text)
        text = re.sub(r"(?i)^the user", "You", text)
        if not text.endswith("."):
            text += "."
        text += " Here's what I can tell you:"
    return text
