import json

def read_json_lines(conn):
    buffer = ""

    while True:
        chunk = conn.recv(65536)
        if not chunk:
            break

        buffer += chunk.decode()

        # Process each line
        while "\n" in buffer:
            line, buffer = buffer.split("\n", 1)
            line = line.strip()

            if not line:
                continue

            try:
                yield json.loads(line)
            except Exception:
                print(f"⚠️ Invalid JSON ignored: {line!r}", flush=True)
                continue
