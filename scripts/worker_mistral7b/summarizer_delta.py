# summarizer_delta.py

import json
import re
import time


def extract_json(text: str):
    """Extracts the first valid JSON object by counting braces."""
    start = text.find("{")
    if start == -1:
        return {}

    depth = 0
    for i in range(start, len(text)):
        char = text[i]

        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                json_str = text[start:i+1]
                try:
                    return json.loads(json_str)
                except Exception:
                    return {}
    return {}


def build_extraction_prompt(chat_id: str, ts: int, message: str, old_state: dict):


    return f"""
    You are a memory extraction engine.

    Return ONLY this JSON format:

    {{
    "summary": "",
    "intent": null,
    "facts": []
    }}

    Rules:
    - summary = 1–2 sentences summarizing the user message.
    - intent = user's goal, or null.
    - facts = list of small fact objects OR [].
    - DO NOT add fields.
    - DO NOT invent metadata.
    - DO NOT wrap inside another object.
    - DO NOT output text before or after JSON.

    User message:
    {message}

    JSON OUTPUT ONLY:
    """




def generate_state_delta(chat_id: str, message: str, ts: int,
                         old_state: dict, tokenizer, model, device) -> dict:
    """Generate a COMPLETE StateDelta JSON (no wrapper)."""
    prompt = build_extraction_prompt(chat_id, ts, message, old_state)

    inputs = tokenizer(prompt, return_tensors="pt").to(device)

    out = model.generate(
        **inputs,
        max_new_tokens=256,
        temperature=0.2,
        top_p=0.9,
        do_sample=False,
        pad_token_id=tokenizer.eos_token_id,
    )

    decoded = tokenizer.decode(
        out[0][inputs['input_ids'].shape[1]:],
        skip_special_tokens=True,
    )

    print(decoded)
    return extract_json(decoded)
