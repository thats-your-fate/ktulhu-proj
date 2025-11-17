# summarizer_delta.py

import json
import re

DELTA_PROMPT_HEADER = """
You are an AI that produces structured JSON deltas for conversational memory.

Given:
1. The user's latest message.
2. The previous conversation state (JSON object).

Output:
A *JSON delta* describing ONLY the new information from this message.

Rules:
- Do NOT repeat information already present in old_state.
- Include only NEW entities, facts, preferences, goals, or memory-relevant details.
- Always produce VALID JSON.
- Keys to use when applicable:
    - "new_entities": []
    - "new_facts": []
    - "new_preferences": []
    - "user_intent": ""
    - "message_summary": ""
- If nothing new exists, produce an empty object {}.

Format:
<JSON ONLY — no explanation>
""".strip()


def clean_json_output(text: str):
    """
    Extract valid JSON from model output.
    """
    match = re.search(r"\{[\s\S]*\}", text)
    if not match:
        return {}
    try:
        return json.loads(match.group(0))
    except Exception:
        return {}


def generate_summary_delta(message: str, old_state: dict,
                           tokenizer, model, device) -> dict:
    """
    Fully safe delta generation, with NO .format() usage.
    """

    # Convert old_state cleanly to pretty JSON
    old_state_json = json.dumps(old_state, indent=2)

    # Build prompt entirely with f-strings — cannot crash on braces
    prompt = (
        f"{DELTA_PROMPT_HEADER}\n\n"
        f"Message:\n{message}\n\n"
        f"Old state:\n{old_state_json}\n\n"
        f"JSON delta:\n"
    )

    inputs = tokenizer(prompt, return_tensors="pt").to(device)

    out = model.generate(
        **inputs,
        max_new_tokens=256,
        temperature=0.2,
        top_p=0.9,
        do_sample=True,
        pad_token_id=tokenizer.eos_token_id,
    )

    decoded = tokenizer.decode(
        out[0][inputs['input_ids'].shape[1]:],
        skip_special_tokens=True,
    )

    return clean_json_output(decoded)
