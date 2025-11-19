from worker_mistral7b.helpers.llm_helpers import generate_mistral_output
from worker_mistral7b.brave import run_brave_search

def rewrite_query(q, tokenizer, model, device):
    q = q.strip()
    if not q:
        return q

    if len(q) > 512:
        q = q[:512]

    prompt = f"""
Rewrite the following user question into a concise search query that would work well for a web search engine.
Only output the query.

User question:
{q}

Search query:
""".strip()

    try:
        out = generate_mistral_output(prompt, tokenizer, model, device).strip()
        if len(out) > 3:
            return out
        return q
    except:
        return q


def perform_search(query):
    return run_brave_search(query)
