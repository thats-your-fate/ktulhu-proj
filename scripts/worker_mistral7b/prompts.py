import re
import json

from worker_mistral7b.search_prompt_builder import build_search_augmented_prompt

def build_final_prompt_with_search(raw_question, rewritten, search_json):
    return f"""
SYSTEM:
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

USER:
{build_search_augmented_prompt(raw_question, rewritten, search_json)}

ASSISTANT:
1. **Summary**
2. **Detailed Explanation**
3. **Key Facts**
4. **Sources**
""".strip()




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



def sanitize_memory(value):
    """
    Converts Python structures into safe JSON strings,
    escaping curly braces so f-strings can't break.
    """
    text = json.dumps(value, indent=2, ensure_ascii=False)
    return text.replace("{", "{{").replace("}", "}}")


def build_prompt_with_memory(question, state, search_block=None):
    intents = state.get("intents", [])
    facts = state.get("facts", [])
    summary = state.get("summary") or "None"

    safe_intents = sanitize_memory(intents)
    safe_facts = sanitize_memory(facts)
    safe_summary = sanitize_memory(summary)

    memory_block = f"""
=== MEMORY START ===
Known user intents:
{safe_intents}

Known structured facts:
{safe_facts}

Conversation summary:
{safe_summary}
=== MEMORY END ===
""".strip()

    # Optional search augmentation
    search_text = ""
    if search_block:
        # 🔥 FIX: Ensure dictionary → JSON string, then escape braces
        if not isinstance(search_block, str):
            search_block = json.dumps(search_block, ensure_ascii=False, indent=2)

        safe_search = search_block.replace("{", "{{").replace("}", "}}")

        search_text = f"""
=== EXTERNAL KNOWLEDGE ===
{safe_search}
=== END EXTERNAL ===
""".rstrip()

    # FINAL PROMPT
    return f"""
SYSTEM:
You are an intelligent AI assistant with persistent memory.
Use the memory block to stay consistent with the user's long-term preferences
and avoid contradictions.

{memory_block}

USER QUESTION:
{question}

{search_text}

ASSISTANT:
Provide the best possible answer by combining:
1. The user's past intents and known facts (memory)
2. The current question
3. External search results (if present)

Be direct, helpful, and consistent.
""".strip()

