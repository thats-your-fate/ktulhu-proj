import socket, json

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect("/tmp/infer_b.sock")
req = {"id": "test1", "text": "Explain free gibbs energy on university level"}
sock.sendall(json.dumps(req).encode())
print(sock.recv(65536).decode())
sock.close()
