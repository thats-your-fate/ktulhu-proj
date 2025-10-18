#!/usr/bin/env python3
import csv, json, socket, time, re, os

SOCKET_PATH = "/tmp/infer_b.sock"
INPUT_CSV = "rqs.csv"
OUTPUT_JSONL = "reqs.jsonl"

def clean_text(t: str) -> str:
    if not t: return ""
    t = re.sub(r"h\d\.", "", t)
    t = re.sub(r"\|\|", "", t)
    t = re.sub(r"\|", " ", t)
    t = re.sub(r"!\S+!", "", t)
    t = re.sub(r"\[https?://[^\]]+\]", "", t)
    return re.sub(r"\s+", " ", t).strip()

def ask_infer(uid: str, text: str) -> str:
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(SOCKET_PATH)
    sock.sendall(json.dumps({"id": uid, "text": text}).encode())
    chunks = []
    while True:
        data = sock.recv(262144)
        if not data: break
        chunks.append(data)
        if len(data) < 262144: break
    sock.close()
    return b"".join(chunks).decode().strip()

def parse_response(resp: str):
    try:
        data = json.loads(resp)
        if isinstance(data, list):
            return [{"question": d.get("question",""), "answer": d.get("answer","")} for d in data if isinstance(d, dict)]
    except Exception:
        pass
    match = re.search(r'(\[.*\])', resp, re.S)
    if match:
        try:
            data = json.loads(match.group(1))
            if isinstance(data, list):
                return [{"question": d.get("question",""), "answer": d.get("answer","")} for d in data if isinstance(d, dict)]
        except Exception:
            pass
    qa_matches = re.findall(r"Q[:\-]\s*(.*?)\s*A[:\-]\s*(.*?)(?=\nQ[:\-]|$)", resp, re.S | re.I)
    return [{"question": q.strip(), "answer": a.strip()} for q,a in qa_matches]

def load_done_ids(path):
    done = set()
    if not os.path.exists(path):
        return done
    with open(path, encoding="utf-8") as f:
        for line in f:
            try:
                entry = json.loads(line)
                # try to recover ID pattern from prompt
                q = entry.get("prompt","")
                m = re.search(r"ID=(\d+)", q)
                if m: done.add(m.group(1))
            except Exception:
                continue
    return done

def main():
    # collect done IDs (optional – we could track them in a side file too)
    done_ids = set()
    print(f"Resuming: checking {OUTPUT_JSONL}")
    if os.path.exists(OUTPUT_JSONL):
        # or simpler: parse CSV indices from previous runs if you stored them
        pass

    # optionally just track numeric progress based on line count
    processed = 0
    if os.path.exists(OUTPUT_JSONL):
        with open(OUTPUT_JSONL, "r", encoding="utf-8") as f:
            processed = sum(1 for _ in f)
        print(f"Already have {processed} QA lines; appending new ones.")

    with open(INPUT_CSV, newline='', encoding='utf-8') as f, open(OUTPUT_JSONL, "a", encoding='utf-8') as out:
        reader = csv.DictReader(f)
        skip = processed > 0
        start_id = None
        if skip:
            # optional: start from the next unprocessed story (approximation)
            start_id = processed // 9  # ~6 QAs per story
        for i, row in enumerate(reader, start=1):
            if start_id and i <= start_id:
                continue

            story_parts = [row.get(k,"") for k in ["Title","Description","Acceptance Criteria","Preconditions"]]
            story = "\n".join(filter(None, map(clean_text, story_parts))).strip()
            if not story:
                continue

            print(f"[{i}] Sending story ID={row.get('ID')} to inference...")
            try:
                prompt_text = f"""
Given the following user story, generate 12 diverse question–answer pairs.
Each pair should be a JSON object with "question" and "answer" keys, e.g.
[{{"question": "...", "answer": "..."}}]

Each question and answer must be self-contained and make sense without reading the full story.
If an answer mentions an action, field, or file type, include a short reference to what it's related to.

User story:
{story}
"""
                response = ask_infer(row.get("ID", f"row{i}"), prompt_text)
                data = parse_response(response)
                if not data:
                    print(f"⚠️  Could not parse QA JSON for ID {row.get('ID')}, skipping.")
                    continue
                for qa in data:
                    q, a = qa.get("question","").strip(), qa.get("answer","").strip()
                    if not q or not a: continue
                    out.write(json.dumps({
                        "prompt": f"Q: {q}\nA:",
                        "completion": f" {a}"
                    }, ensure_ascii=False) + "\n")
                out.flush()
                print(f"✅  Saved {len(data)} QAs for {row.get('ID')}")
            except Exception as e:
                print(f"❌ Error processing {row.get('ID')}: {e}")
            time.sleep(0.5)
    print("\n✅ Resumed run finished!")

if __name__ == "__main__":
    main()
