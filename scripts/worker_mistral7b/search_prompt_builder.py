# search_prompt_builder.py

def build_search_augmented_prompt(
    raw_question: str,
    rewritten_query: str,
    search_json: dict
) -> str:
    """
    Build the section of the prompt that includes:
    - the original user question
    - the rewritten search query
    - the extracted external information
    """

    parts = []
    parts.append(f"User question: {raw_question}\n")
    parts.append(f"Search query used: {rewritten_query}\n")
    parts.append("Relevant external information from web search:\n")

    # ---- Brave search failure case ----
    if not search_json or search_json.get("error"):
        parts.append(f"(Search error: {search_json.get('error')})")
        parts.append("No external sources available.\n")
        parts.append("Answer based on general knowledge.")
        return "\n".join(parts)

    results = search_json.get("results") or []

    # ---- No results case ----
    if not results:
        parts.append("(No search results found.)")
        parts.append("Answer based on general knowledge.\n")
        return "\n".join(parts)

    # ---- Normal case: add extracted results ----
    for r in results:
        title = r.get("title") or r.get("headline") or "Untitled"
        url = r.get("url") or ""
        paragraphs = r.get("paragraphs") or []
        snippet = r.get("snippet") or ""

        parts.append(f"Source: {title}")
        parts.append(f"URL: {url}")

        if paragraphs:
            for p in paragraphs[:3]:
                parts.append(f"Paragraph: {p}")
        elif snippet:
            parts.append(f"Paragraph: {snippet}")
        else:
            parts.append("Paragraph: (no text available)")

        parts.append("")  # blank line

    parts.append(
        "Use the information above to answer the user's question accurately."
    )

    return "\n".join(parts)
