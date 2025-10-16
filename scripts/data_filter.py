#!/usr/bin/env python3
import bz2
import xml.etree.ElementTree as ET
from pathlib import Path
import subprocess
import json

INPUT_FILE = "/media/yaro/ENWIKI/enwiki/latest/enwiki-latest-pages-articles3.xml-p151574p311329.bz2"
OUTPUT_FILE = "distilled_articles.jsonl"
THRESHOLD = 0.75  # adjust based on model output

def extract_pages(bz_path):
    """Stream pages from a bz2 XML Wikipedia dump."""
    with bz2.open(bz_path, 'rb') as f:
        buf = b""
        for line in f:
            buf += line
            if b"</page>" in line:
                try:
                    page = ET.fromstring(buf.decode('utf-8', errors='ignore'))
                    title = page.findtext('./title')
                    text = page.findtext('./revision/text')
                    if text:
                        yield {"title": title, "text": text}
                except Exception:
                    pass
                buf = b""

def infer(text):
    """Call your inference worker (Python or Rust-bridged) for BERT classification."""
    # example: call your Python worker directly
    result = subprocess.run(
        ["python3", "scripts/inference.py", "--text", text],
        capture_output=True,
        text=True,
    )
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"score": 0.0}

def main():
    out = open(OUTPUT_FILE, "w")
    for page in extract_pages(INPUT_FILE):
        result = infer(page["text"][:2048])  # truncate to reasonable token length
        score = result.get("score", 0.0)
        if score >= THRESHOLD:
            json.dump({
                "title": page["title"],
                "score": score,
                "text": page["text"],
            }, out)
            out.write("\n")
    out.close()

if __name__ == "__main__":
    main()
