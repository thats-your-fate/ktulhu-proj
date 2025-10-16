#!/usr/bin/env python3
import bz2
import xml.etree.ElementTree as ET
import csv
import json
import socket
import time
import os
import traceback
from pathlib import Path
import random

# CONFIG
SOCK_PATH = "/tmp/infer_a.sock"
INPUT_FILE = "/media/yaro/ENWIKI/enwiki/latest/enwiki-latest-pages-articles3.xml-p151574p311329.bz2"
OUTPUT_FILE = "distilled_articles.csv"
THRESHOLD = 0.8  # confidence threshold for YES verdicts
LOG_EVERY = 10   # how often to log progress details

def log(msg: str):
    """Print log messages immediately (for long runs)."""
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)

def send_infer(text: str, retries: int = 4):
    """Send a single inference request via Unix socket with retries and backoff."""
    req = json.dumps({"id": str(time.time()), "text": text}).encode()

    for attempt in range(1, retries + 1):
        try:
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
                s.connect(SOCK_PATH)
                s.sendall(req)
                data = s.recv(65536)
                resp = json.loads(data.decode())
                log(f"🧠 Inferred -> verdict={resp.get('verdict')} conf={resp.get('confidence')}")
                return resp
        except (ConnectionRefusedError, BrokenPipeError) as e:
            wait = 0.3 * attempt + random.uniform(0, 0.2)
            log(f"[WARN] Socket busy (attempt {attempt}/{retries}): {e}, retrying in {wait:.2f}s...")
            time.sleep(wait)
        except Exception as e:
            log(f"[ERROR] Socket communication failed: {e}")
            traceback.print_exc()
            break

    log("❌ Max retries reached, returning default NO")
    return {"verdict": "NO", "confidence": 0.0}


def extract_pages(bz_path):
    """Stream Wikipedia pages from a bz2 XML dump."""
    with bz2.open(bz_path, "rb") as f:
        buf = b""
        for line in f:
            buf += line
            if b"</page>" in line:
                try:
                    page = ET.fromstring(buf.decode("utf-8", errors="ignore"))
                    title = page.findtext("./title") or ""
                    text = page.findtext("./revision/text") or ""
                    if text.strip():
                        yield {"title": title.strip(), "text": text.strip()}
                except Exception as e:
                    log(f"[WARN] XML parse failed: {e}")
                buf = b""

def main():
    os.makedirs(Path(OUTPUT_FILE).parent, exist_ok=True)
    total = kept = 0

    log(f"🚀 Starting distillation from {INPUT_FILE}")
    log(f"🔗 Connecting to inference socket: {SOCK_PATH}")
    log(f"🧾 Writing output to: {OUTPUT_FILE}")

    with open(OUTPUT_FILE, "w", newline="", encoding="utf-8") as csvfile:
        writer = csv.writer(csvfile)
        writer.writerow(["title", "confidence", "text"])

        for page in extract_pages(INPUT_FILE):
            total += 1
            text = page["text"][:2048]  # truncate long pages

            log(f"\n📄 Processing page #{total}: {page.get('title','')[:80]!r}")
            preview = (page.get("text", "")[:120]).replace("\n", " ").replace("\r", " ")
            log(f"🧾 Text preview: {preview}...")
            result = send_infer(text)

            #result = send_infer(text)

            if result.get("verdict") == "YES" and result.get("confidence", 0) >= THRESHOLD:
                writer.writerow([page["title"], result["confidence"], page["text"].replace("\n", " ")])
                kept += 1
                log(f"✅ Accepted ({result['confidence']:.3f}) — total kept: {kept}")
            else:
                log(f"❌ Rejected ({result.get('confidence', 0):.3f})")

            if total % LOG_EVERY == 0:
                ratio = kept / total if total > 0 else 0
                log(f"📊 Progress: processed {total}, kept {kept} ({ratio:.2%})")

    log(f"✅ Done! Kept {kept}/{total} articles into {OUTPUT_FILE}")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        log("🛑 Interrupted by user (Ctrl+C)")
    except Exception as e:
        log(f"[FATAL] Unexpected error: {e}")
        traceback.print_exc()
