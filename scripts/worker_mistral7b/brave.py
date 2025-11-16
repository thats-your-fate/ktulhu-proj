import json
import socket
import os

# Path to the UNIX socket used by Rust scraper
SCRAPER_SOCKET = "/tmp/search_scraper.sock"


def run_brave_search(query: str) -> dict:
    """
    Sends the query to Rust scraper via UNIX socket.
    Returns identical structure as the old Perl wrapper.
    """

    query = query.strip()
    if not query:
        return {"query": "", "results": [], "error": "empty_query"}

    # Check socket exists
    if not os.path.exists(SCRAPER_SOCKET):
        return {
            "query": query,
            "results": [],
            "error": f"scraper_socket_not_found: {SCRAPER_SOCKET}"
        }

    # JSON request payload
    payload = json.dumps({"query": query})

    try:
        # Connect to the UNIX socket
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.settimeout(10)                     # <-- timeout protection
        client.connect(SCRAPER_SOCKET)

        # Send JSON
        client.sendall(payload.encode("utf-8"))
        client.shutdown(socket.SHUT_WR)          # Stop writing, signal EOF

        # Receive response until socket closes
        data = b""
        while True:
            chunk = client.recv(4096)
            if not chunk:
                break
            data += chunk

        client.close()

        if not data:
            return {
                "query": query,
                "results": [],
                "error": "empty_response"
            }

        response = json.loads(data.decode("utf-8"))

        # Normalize result shape like old Perl version
        return {
            "query": response.get("query", query),
            "results": response.get("results", []),
            "error": response.get("error")
        }

    except socket.timeout:
        return {"query": query, "results": [], "error": "timeout"}

    except Exception as e:
        return {"query": query, "results": [], "error": str(e)}
