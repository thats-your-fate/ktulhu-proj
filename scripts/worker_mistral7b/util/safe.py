def escape_curly(text: str) -> str:
    """
    Escapes { and } so that inserting JSON into f-strings or .format() is safe.
    """
    if not isinstance(text, str):
        text = str(text)
    return text.replace("{", "{{").replace("}", "}}")
