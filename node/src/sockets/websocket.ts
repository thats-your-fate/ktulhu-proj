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

  // -----------------------
  // PARSE WEBSOCKET JSON
  // -----------------------
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

  // -----------------------
  // CONTEXT
  // -----------------------
  const device = (ws as any).deviceHash || null;
  const chatId = data.chat_id || data.session_id || clientId;

  // -----------------------
  // NORMALIZE USER TEXT
  // Supports: text, prompt, message, nested JSON in text
  // -----------------------
function extractUserText(input: any): string {
  if (!input) return "";

  // CASE 1: looks like a JSON string → try to decode it
  if (typeof input === "string") {
    const trimmed = input.trim();

    if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
      try {
        const obj = JSON.parse(trimmed);
        return extractUserText(obj);
      } catch {
        // not valid JSON, return raw string
        return trimmed;
      }
    }

    // plain text string
    return trimmed;
  }

  // CASE 2: object → try text / prompt / message keys
  if (typeof input === "object") {
    if (typeof input.text === "string") return extractUserText(input.text);
    if (typeof input.prompt === "string") return extractUserText(input.prompt);
    if (typeof input.message === "string") return extractUserText(input.message);

    return "";
  }

  return String(input);
}


  const userText = extractUserText(data.text) ||
                   extractUserText(data.prompt) ||
                   extractUserText(data.message) ||
                   "";

  // -----------------------
  // SEND USER MESSAGE TO KAFKA
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

  //const chatId = data.chat_id || data.session_id || clientId;



  // Build worker payload including memory:
  const workerPayload = {
    id: data.id,
    chat_id: chatId,
    role: "user",
    text: userText.trim(),
    model: data.model || "mistral-7b-lora",
    ts: Date.now(),
    session_id: data.session_id,
    device_hash: device
  };


  const workerRaw = JSON.stringify(workerPayload);

  // -----------------------
  // SEND TO MISTRAL WORKER
  // -----------------------
  try {
    const sock = net.createConnection(unixPath);
    sock.setKeepAlive(true, 5000);

    console.log("➡️ SENDING TO WORKER:", workerPayload);
    sock.write(workerRaw + "\n");

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
