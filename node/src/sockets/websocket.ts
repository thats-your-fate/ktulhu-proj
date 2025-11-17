import WebSocket, { WebSocketServer } from "ws";
import http from "http";
import net from "net";
import { randomUUID } from "crypto";
import { ensureKafka } from "../kafka/client";
import { emitMessageToKafka } from "../kafka/emitters";
import { waitForSocket } from "../utils/waitForSocket";
import { log } from "../utils/logger";
import { handleWorkerStream } from "./worker";
import { withIdAndTimestamp } from "../utils/withIdAndTimestamp";

export async function startSocketServer(unixPath: string, port: number) {
  await ensureKafka();
  await waitForSocket(unixPath);

  const deviceClients = new Map<string, Set<WebSocket>>();
  const server = http.createServer((_, res) => {
    res.writeHead(200, { "Content-Type": "text/plain" });
    res.end(`Mistral LoRA proxy alive on ${port}\n`);
  });

  const wss = new WebSocketServer({ server });

  wss.on("connection", (ws) => {
    const clientId = randomUUID();
    (ws as any).clientId = clientId;
    log.ok(`🔌 WebSocket connected → clientId=${clientId}`);

    ws.on("message", async (msg) => {
      const raw = msg.toString();
      let data: any;

      try {
        data = JSON.parse(raw);
      } catch {
        log.err("❌ Invalid JSON from WS client");
        return;
      }

      // -----------------------
      // DEVICE REGISTRATION
      // -----------------------
      if (data.type === "register" && data.device_hash) {
        const device = data.device_hash;
        const set = deviceClients.get(device) || new Set<WebSocket>();
        set.add(ws);
        deviceClients.set(device, set);
        (ws as any).deviceHash = device;

        log.ok(`📱 Registered device ${device}`);
        return;
      }

      const device = (ws as any).deviceHash || null;
      const chatId = data.chat_id || data.session_id || clientId;

      // -----------------------
      // USER MESSAGE HANDLING
      // -----------------------
      let userText = "";

      if (typeof data.text === "string") userText = data.text;
      else if (typeof data.prompt === "string") userText = data.prompt;
      else if (typeof data.message === "string") userText = data.message;
      else if (typeof data.text === "object" && data.text !== null)
        userText = data.text.text || data.text.prompt || "";

      // -----------------------
      // EMIT USER MESSAGE TO KAFKA (WITH UUID)
      // -----------------------
      if (userText.trim()) {
        const userEvent = withIdAndTimestamp({
          role: "user",
          chat_id: chatId,
          session_id: data.session_id,
          device_hash: device,
          text: userText.trim(),
        });

        await emitMessageToKafka(userEvent);
      }

      // -----------------------
      // SEND REQUEST TO WORKER
      // -----------------------
      try {
        const sock = net.createConnection(unixPath);
        sock.setKeepAlive(true, 5000);

        console.log("➡️ SENDING TO WORKER:", raw);
        sock.write(raw + "\n");

        handleWorkerStream(sock, ws, data, chatId, deviceClients);
      } catch (err: any) {
        log.err(` Worker socket error: ${err.message}`);
        ws.send(JSON.stringify({ error: err.message }));
      }
    });

    // -----------------------
    // CLEANUP
    // -----------------------
    ws.on("close", () => {
      const device = (ws as any).deviceHash;
      if (device && deviceClients.has(device)) {
        const set = deviceClients.get(device)!;
        set.delete(ws);
        if (set.size === 0) deviceClients.delete(device);
        log.warn(`🧹 Disconnected and removed device ${device}`);
      }
    });
  });

  server.listen(port, "0.0.0.0", () =>
    log.ok(`🚀 Proxy WebSocket listening on port ${port} → ${unixPath}`)
  );

  return { wss, server };
}
