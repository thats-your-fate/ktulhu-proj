from worker_mistral7b.search_classifier import ask_need_search

def should_search(question, tokenizer, model, device):
    return ask_need_search(question, tokenizer, model, device)
