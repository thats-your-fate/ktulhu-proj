
# search_prompt_builder.py

def build_search_augmented_prompt(original: str, search_json: dict) -> str:
    """
    Build final prompt including scraped web results.
    """
    results = search_json.get("results", [])

    parts = ["Here is external information found via web search:\n"]

    for r in results:
        title = r.get("title", "")
        url = r.get("url", "")
        headline = r.get("headline", "")
        paras = r.get("paragraphs", [])

        parts.append(f"Source: {title}\nURL: {url}\nHeadline: {headline}")
        for p in paras[:3]:
            parts.append(f"Paragraph: {p}")
        parts.append("")

    search_block = "\n".join(parts)

    final = f"""
Use the external information below to answer the user accurately.

{search_block}

User question: {original}

Answer clearly and concisely:
""".strip()

    return final

