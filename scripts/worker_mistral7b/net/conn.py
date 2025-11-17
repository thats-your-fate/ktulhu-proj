import json, time

def send_event(conn, event: dict):
    conn.sendall((json.dumps(event) + "\n").encode())

def send_system(conn, uid, chat_id, msg: str):
    send_event(conn, {
        "id": uid,
        "type": "system",
        "chat_id": chat_id,
        "system": msg,
        "ts": time.time()
    })
