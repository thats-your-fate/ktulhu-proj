import json, time

def send_event(conn, event: dict):
    """
    Safely send an event to the client.

    If conn is None (API mode), do nothing.
    If the socket is closed or write fails, silently ignore the error.
    """
    if conn is None:
        return  # API mode: no streaming

    try:
        conn.sendall((json.dumps(event) + "\n").encode())
    except Exception:
        # Prevent worker crashes if socket closed
        pass


def send_system(conn, uid, chat_id, msg: str):
    """
    Send a system message using send_event(), safe for conn=None.
    """
    send_event(conn, {
        "id": uid,
        "type": "system",
        "chat_id": chat_id,
        "system": msg,
        "ts": time.time(),
    })
