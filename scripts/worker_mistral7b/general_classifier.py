def make_broad_classifier_prompt(question: str) -> str:
    return f"""
You are a strict binary classifier.

Your task is to decide whether the user expects a BROAD answer or a NARROW answer.

Output BROAD only if the question asks for:
- an explanation
- a high-level overview
- general advice or brainstorming
- comparisons, pros/cons, lists of ideas
- conceptual or open-ended answers

Output NARROW only if the question asks for:
- a specific fact or number
- a targeted detail
- a concrete definition
- a direct instruction
- a yes/no style answer

Do NOT overpredict BROAD — only choose it when clearly required.

Question: {question}

Respond with exactly one word: BROAD or NARROW.

Answer now:
""".strip()


def ask_need_broad_answer(question: str, tokenizer, model, device) -> bool:
    clf_prompt = make_broad_classifier_prompt(question)

    inputs = tokenizer(clf_prompt, return_tensors="pt").to(device)

    out = model.generate(
        **inputs,
        max_new_tokens=5,
        temperature=0.0,
        top_p=1.0,
        do_sample=False,
        pad_token_id=tokenizer.eos_token_id,
    )

    decoded = tokenizer.decode(
        out[0][inputs["input_ids"].shape[1]:],
        skip_special_tokens=True
    ).strip().upper()

    print("🧭 Broad/Narrow raw:", decoded)

    word = (
        decoded.replace(".", "")
               .replace(",", "")
               .replace("?", "")
               .split()[0]
        if decoded else ""
    )

    if word == "BROAD":
        return True
    if word == "NARROW":
        return False

    print("⚠️ Broad/Narrow unclear → default NARROW")
    return False
