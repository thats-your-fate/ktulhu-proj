import socket, json, re, torch
def make_summary_with_model(user_text: str, tokenizer=None, model=None, device="cpu") -> str:
    sock_path = "/tmp/infer_c.sock"
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.connect(sock_path)
            payload = json.dumps({"id": "summary_req", "text": user_text}).encode()
            client.sendall(payload)
            data = client.recv(4096)
            if not data:
                raise ValueError("Empty response")
            res = json.loads(data.decode())
            return res.get("summary", "General request")
    except Exception:
        pass

    if tokenizer and model:
        try:
            prompt = (
                "Summarize the following user message in 5–8 words:\n\n"
                f"Message: {user_text}\n\nSummary:"
            )
            inputs = tokenizer(prompt, return_tensors="pt").to(device)
            output = model.generate(**inputs, max_new_tokens=32, temperature=0.3, top_p=0.8)
            summary_text = tokenizer.decode(output[0], skip_special_tokens=True)
            summary_text = re.sub(r"(?is)^.*?Summary:\s*", "", summary_text).split("\n")[0].strip()
            return summary_text or "General request"
        except Exception:
            return "General request"
    return "General request"
