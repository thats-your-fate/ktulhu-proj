import WebSocket, { WebSocketServer } from "ws";
import http from "http";
import fs from "fs";
import net from "net";
import path from "path";
import { log } from "./utils/logger";

async function waitForSocket(unixPath: string, maxAttempts = 30): Promise<void> {
  let attempt = 0;
  const absPath = path.resolve(unixPath);
  while (attempt < maxAttempts) {
    if (fs.existsSync(absPath)) {
      log.ok(`✅ Found worker socket: ${absPath}`);
      return;
    }
    attempt++;
    log.warn(`⏳ Waiting for ${absPath} (attempt ${attempt}/${maxAttempts})...`);
    await new Promise(res => setTimeout(res, 10_000));
  }
  throw new Error(`Timeout waiting for worker socket: ${absPath}`);
}

export async function startSocketServer(unixPath: string, port: number) {
  await waitForSocket(unixPath);

  const server = http.createServer((_, res) => {
    res.writeHead(200, { "Content-Type": "text/plain" });
    res.end(`BA LoRA proxy alive on ${port}\n`);
  });

  const wss = new WebSocketServer({ server });

  wss.on("connection", (ws: WebSocket) => {
    log.ok(`🌐 WebSocket client connected (port ${port})`);

    ws.on("message", (msg: WebSocket.RawData) => {
      const data = msg.toString();
      log.info(`↩️ From WS client → worker [${unixPath}]: ${data}`);
      const sock = net.createConnection(unixPath);
      sock.setKeepAlive(true, 5000);
      sock.write(data + "\n");

      let buffer = "";
      sock.on("data", chunk => {
        buffer += chunk.toString();
        let parts = buffer.split("\n");
        buffer = parts.pop()!;
        for (const line of parts) {
          if (!line.trim()) continue;
          try {
            ws.send(line);
            if (line.includes('"done"')) sock.end();
          } catch (err) {
            log.err(`Error sending WS chunk: ${(err as Error).message}`);
          }
        }
      });

      sock.on("error", err => {
        log.err(`❌ Worker socket error (${unixPath}): ${err.message}`);
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ error: err.message }));
          ws.close();
        }
      });

      ws.on("close", () => {
        log.warn(`WS client disconnected from port ${port}`);
        sock.destroy();
      });
    });
  });

  server.listen(port, "0.0.0.0", () =>
    log.ok(`🧩 Proxy WebSocket listening on port ${port} → ${unixPath}`)
  );

  return { wss, server };
}
