import re
import torch

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
