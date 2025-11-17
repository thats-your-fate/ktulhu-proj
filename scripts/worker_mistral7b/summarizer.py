import re
import torch

from worker_mistral7b.util.safe import escape_curly



def make_summary_with_model(user_text: str, tokenizer, model, device="cuda") -> str:
    """
    Generate a concise 3-word title (used as summary) for the user's request.
    Uses the same model/device as main inference.
    """

    try:
        prompt = (
            "Create a very short 3-word title describing the user's request.\n\n"
            f"User message: {user_text}\n\n"
            "Title:"
        )

        with torch.inference_mode():
            inputs = tokenizer(prompt, return_tensors="pt").to(device)
            output = model.generate(
                **inputs,
                max_new_tokens=16,
                temperature=0.4,
                top_p=0.8,
                do_sample=True,
                pad_token_id=tokenizer.eos_token_id,
            )

        # Decode and clean up
        text = tokenizer.decode(output[0], skip_special_tokens=True)
        title = re.sub(r"(?is)^.*?Title:\s*", "", text).strip()
        title = re.split(r"[\n\.\!\?]", title)[0].strip()

        # Enforce max 3 words
        words = title.split()
        title = " ".join(words[:3]) if words else "General request"

        return title or "General request"

    except Exception as e:
        print(f"⚠️ make_summary_with_model failed: {e}", flush=True)
        return "General request"



def make_advanced_context_summary(message, tokenizer, model, device):
    """
    Safe version: never uses .format(), so JSON and curly braces cannot break.
    Everything is concatenated with f-strings only.
    """

    # No escaping needed, since no .format() is used.
    prompt = (
        "You are an expert at extracting structured conversational memory.\n\n"
        "Given a single user message, produce a compact, factual, long-term summary in this format:\n\n"
        "- **Intent**: What the user is trying to do or ask.\n"
        "- **Entities**: People, places, objects, APIs, models, or special terms.\n"
        "- **Relevant context**: Facts that will matter in future turns.\n"
        "- **User preferences**: Any expressed preferences or style.\n"
        "- **Memory candidates**: Information that should be stored for future responses.\n\n"
        "Rules:\n"
        "- Do NOT include hallucinations.\n"
        "- Only summarize what is actually present.\n"
        "- Keep the summary under 120 tokens.\n"
        "- Format as clean markdown.\n\n"
        f"User message:\n{message}\n\n"
        "Summary:\n"
    )

    inputs = tokenizer(prompt, return_tensors="pt").to(device)

    out = model.generate(
        **inputs,
        max_new_tokens=180,
        temperature=0.2,
        top_p=0.9,
        do_sample=True,
        pad_token_id=tokenizer.eos_token_id,
    )

    decoded = tokenizer.decode(
        out[0][inputs["input_ids"].shape[1]:],
        skip_special_tokens=True
    )

    return decoded.strip()
