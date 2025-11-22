import re
import json

from worker_mistral7b.search_prompt_builder import build_search_augmented_prompt


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

# worker_mistral7b/prompts/memory_prompt.py

import json

def build_prompt_with_memory(question, state, search_block=None):
    intents = state.get("intents") or []
    facts = state.get("facts") or []
    summary = state.get("summary") or "No summary available."

    memory_json = json.dumps({
        "intents": intents,
        "facts": facts,
        "summary": summary
    }, indent=2, ensure_ascii=False)

    external_json = (
        json.dumps(search_block, indent=2, ensure_ascii=False)
        if search_block else "None"
    )

    return f"""
## SYSTEM
You are an intelligent assistant. You may use the provided memory and optional external knowledge to give a helpful, well-structured answer.

Always respond directly to the user in first person. 
Do not describe the user’s actions or thoughts. 
Your output should be in **Markdown format**.  
You may structure your answer as you see fit (headings, lists, tables, summaries), as long as it is clear and helpful.

---

### Memory (for your reasoning only — DO NOT include this section in the output)
{memory_json}

### External Knowledge (optional — DO NOT include this section in the output)
{external_json}

---

## USER QUESTION
"{question}"

---

## ASSISTANT
Please answer in **Markdown**, using any structure you consider useful, include links if ### External Knowledge is present.  

If necessary, make your answer broad and comprehensive, do not invent facts

Focus on clarity, correctness, and helpfulness.
""".strip()


def build_narrow_with_memory(question, state, search_block=None):
    intents = state.get("intents") or []
    facts = state.get("facts") or []
    summary = state.get("summary") or "No summary available."

    memory_json = json.dumps({
        "intents": intents,
        "facts": facts,
        "summary": summary
    }, indent=2, ensure_ascii=False)

    external_json = (
        json.dumps(search_block, indent=2, ensure_ascii=False)
        if search_block else "None"
    )

    return f"""
## SYSTEM
Always respond directly to the user in first person. 
answer clearly and conversationally.
Do not describe the users actions or thoughts. 
You may use the provided memory and optional external knowledge to give a helpful answer.

---

### Memory (for your reasoning only — DO NOT include this section in the output)
{memory_json}

### External Knowledge (optional — DO NOT include this section in the output)
{external_json}

---

## USER QUESTION
"{question}"

---

## ASSISTANT

Make you answer brief.

Focus on clarity, correctness, and helpfulness.
""".strip()
