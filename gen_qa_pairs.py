#!/usr/bin/env python3
import csv
import json
import socket
import time
import re

SOCKET_PATH = "/tmp/infer_b.sock"
INPUT_CSV = "rqs.csv"
OUTPUT_JSONL = "reqs.jsonl"

def clean_text(t: str) -> str:
    if not t:
        return ""
    t = re.sub(r"h\d\.", "", t)
    t = re.sub(r"\|\|", "", t)
    t = re.sub(r"\|", " ", t)
    t = re.sub(r"!\S+!", "", t)
    t = re.sub(r"\[https?://[^\]]+\]", "", t)
    t = re.sub(r"\s+", " ", t).strip()
    return t

def ask_infer(uid: str, text: str) -> str:
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(SOCKET_PATH)
    req = {"id": uid, "text": text}
    sock.sendall(json.dumps(req).encode())
    resp = sock.recv(65536).decode()
    sock.close()
    return resp.strip()

def parse_response(resp: str):
    """Try to extract QA pairs in multiple possible formats."""
    # Try JSON directly
    try:
        data = json.loads(resp)
        if isinstance(data, list):
            return [{"question": str(d.get("question", "")), "answer": str(d.get("answer", ""))} for d in data if isinstance(d, dict)]
    except Exception:
        pass

    # Try to find JSON array inside text
    match = re.search(r'(\[.*\])', resp, re.S)
    if match:
        try:
            data = json.loads(match.group(1))
            if isinstance(data, list):
                return [{"question": str(d.get("question", "")), "answer": str(d.get("answer", ""))} for d in data if isinstance(d, dict)]
        except Exception:
            pass

    # Fallback: extract Q/A lines manually
    pairs = []
    qa_matches = re.findall(r"Q[:\-]\s*(.*?)\s*A[:\-]\s*(.*?)(?=\nQ[:\-]|$)", resp, re.S | re.I)
    for q, a in qa_matches:
        pairs.append({"question": q.strip(), "answer": a.strip()})
    return pairs

def main():
    with open(INPUT_CSV, newline='', encoding='utf-8') as f, open(OUTPUT_JSONL, "w", encoding='utf-8') as out:
        reader = csv.DictReader(f)
        for i, row in enumerate(reader, start=1):
            story_parts = [row.get(k, "") for k in ["Title", "Description", "Acceptance Criteria", "Preconditions"]]
            story = "\n".join(filter(None, map(clean_text, story_parts))).strip()
            if not story:
                continue

            prompt_text = f"""
Given the following user story, generate 7 diverse question–answer pairs.
Each pair should be JSON objects with "question" and "answer" keys, e.g.
[{{"question": "...", "answer": "..."}}]

User story:
{story}
"""

            print(f"[{i}] Sending story ID={row.get('ID')} to inference...")
            try:
                response = ask_infer(row.get("ID", f"row{i}"), prompt_text)
                data = parse_response(response)

                if not data:
                    print(f"⚠️  Could not parse QA JSON for ID {row.get('ID')}, skipping.")
                    continue

                for qa in data:
                    q = qa.get("question", "").strip()
                    a = qa.get("answer", "").strip()
                    if not q or not a:
                        continue
                    out.write(json.dumps({
                        "prompt": f"Q: {q}\nA:",
                        "completion": f" {a}"
                    }, ensure_ascii=False) + "\n")

                out.flush()
                print(f"✅  Saved {len(data)} QAs for {row.get('ID')}")
            except Exception as e:
                print(f"❌ Error processing {row.get('ID')}: {e}")

            time.sleep(0.5)

    print(f"\n✅ All done! Output saved to: {OUTPUT_JSONL}")

if __name__ == "__main__":
    main()
