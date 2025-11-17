import torch
from transformers import AutoTokenizer, AutoModelForCausalLM

def load_engine(model_name: str):
    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"🧠 Loading {model_name} on {device}...", flush=True)

    tokenizer = AutoTokenizer.from_pretrained(model_name)

    model = AutoModelForCausalLM.from_pretrained(
        model_name,
        torch_dtype=torch.float16 if device == "cuda" else torch.float32,
        device_map="auto"
    ).eval()

    return tokenizer, model, device
