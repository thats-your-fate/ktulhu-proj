def generate_mistral_output(prompt: str, tokenizer, model, device) -> str:
    """
    Universal inference helper used for:
    - search query rewriting
    - classification
    - summarization
    - meta-response correction
    - any short single-turn inference

    Returns the full model output as plain text.
    """

    inputs = tokenizer(prompt, return_tensors="pt").to(device)

    output_ids = model.generate(
        **inputs,
        max_new_tokens=128,
        do_sample=False,
        temperature=0.0,
        top_p=1.0,
        pad_token_id=tokenizer.eos_token_id,
        eos_token_id=tokenizer.eos_token_id,
    )

    # Decode full output (including prompt)
    decoded = tokenizer.decode(output_ids[0], skip_special_tokens=True)

    # Remove prompt prefix → extract only model response
    if prompt in decoded:
        decoded = decoded.replace(prompt, "", 1).strip()

    return decoded.strip()