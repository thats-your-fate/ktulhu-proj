import json

def extract_user_question(prompt):

    # Normal: dict from Node
    if isinstance(prompt, dict):
        value = prompt.get("text") or prompt.get("prompt") or prompt.get("message")
        return _extract_recursive(value)

    # JSON string
    if isinstance(prompt, str):
        return _extract_recursive(prompt)

    return str(prompt)


def _extract_recursive(value):
    """
    Recursively unwrap values until they stop being JSON strings.
    Handles the nested/double JSON string you showed in logs.
    """
    if not isinstance(value, str):
        return value

    # Try to parse JSON until it fails
    for _ in range(3):  # max depth
        try:
            parsed = json.loads(value)
            if isinstance(parsed, dict):
                # Try common user text keys
                inner = parsed.get("text") or parsed.get("prompt") or parsed.get("message")
                if inner:
                    value = inner
                    continue
                else:
                    return value  # dict but no more text fields
            else:
                return parsed
        except:
            return value  # not JSON anymore

    return value
