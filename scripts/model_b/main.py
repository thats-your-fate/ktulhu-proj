#!/usr/bin/env python3
import os, torch
from common import load_model, run_server

SOCK_PATH = "/tmp/infer_b.sock"
MODEL_NAME = "distilbert-base-uncased-finetuned-sst-2-english"

tokenizer, model = load_model(MODEL_NAME)

def infer(text: str):
    inputs = tokenizer(text, return_tensors="pt", truncation=True, max_length=512)
    with torch.no_grad():
        logits = model(**inputs).logits
        probs = torch.nn.functional.softmax(logits, dim=-1).squeeze()
    conf, idx = torch.max(probs, dim=-1)
    verdict = "YES" if idx.item() == 1 else "NO"
    return {
        "output": f"Verdict: {verdict} (confidence={conf.item():.2f})",
        "verdict": verdict,
        "confidence": round(conf.item(), 3),
        "tokens": len(text.split())
    }

if __name__ == "__main__":
    run_server(SOCK_PATH, infer)
