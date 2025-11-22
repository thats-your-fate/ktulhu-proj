def make_classifier_prompt(question: str) -> str:
    return f"""
You are a strict binary classifier.

Your job is to output SEARCH only when the user explicitly asks about
real-time, changing, or time-dependent information.

You must output SEARCH only if the question clearly asks about:
- news
- current events
- something happening today, now, or this week
- current prices, exchange rates, stocks, crypto
- weather or forecasts
- anything that changes over time

All other questions must be NOSEARCH.

Questions about general knowledge, geography, definitions, climate,
history, math, or writing tasks MUST be NOSEARCH.

Question: {question}

Respond with exactly one word: SEARCH or NOSEARCH.

Answer now:
""".strip()


def ask_need_search(question: str, tokenizer, model, device) -> bool:
    clf_prompt = make_classifier_prompt(question)

    # Encode
    inputs = tokenizer(clf_prompt, return_tensors="pt").to(device)

    # Generate: allow ONLY 1–2 tokens
    out = model.generate(
        **inputs,
        max_new_tokens=5,
        temperature=0.0,
        top_p=1.0,
        do_sample=False,
        pad_token_id=tokenizer.eos_token_id,
    )

    # Decode only the new tokens
    decoded = tokenizer.decode(out[0][inputs["input_ids"].shape[1]:], 
                               skip_special_tokens=True)

    raw = decoded.strip().upper()
    print("🔍 Classifier raw:", raw)

    # Extract only first word, strip punctuation
    word = raw.replace(".", "").replace(",", "").replace("?", "").split()[0] \
           if raw else ""

    if word == "SEARCH":
        return True
    if word == "NOSEARCH":
        return False

    print("⚠️ Classifier unclear → default NOSEARCH")
    return False


