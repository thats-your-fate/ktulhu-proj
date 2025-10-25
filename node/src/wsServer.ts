import WebSocket, { WebSocketServer } from "ws";
import http from "http";
import fs from "fs";
import net from "net";
import path from "path";
import { randomUUID } from "crypto";
import { Kafka, Producer } from "kafkajs";
import { Worker } from "worker_threads";
import { log } from "./utils/logger";

const kafka = new Kafka({
  clientId: "ws-proxy",
  brokers: ["localhost:9092"],
});

let producer: Producer | null = null;
const deviceClients = new Map<string, Set<WebSocket>>();

async function ensureKafka(): Promise<void> {
  if (!producer) producer = kafka.producer();
  try {
    await producer.connect();
    log.ok("🪣 Connected to Kafka broker");
  } catch (err: any) {
    log.err(`Kafka connection failed: ${err.message}`);
  }
}

async function waitForSocket(unixPath: string, maxAttempts = 30): Promise<void> {
  let attempt = 0;
  const absPath = path.resolve(unixPath);
  while (attempt < maxAttempts) {
    if (fs.existsSync(absPath)) {
      log.ok(` Found worker socket: ${absPath}`);
      return;
    }
    attempt++;
    log.warn(`⏳ Waiting for ${absPath} (attempt ${attempt}/${maxAttempts})...`);
    await new Promise((res) => setTimeout(res, 10_000));
  }
  throw new Error(`Timeout waiting for worker socket: ${absPath}`);
}

/** 🔄 Start Kafka bridge in a background worker thread */
function startKafkaBridge() {
  const bridgePath = path.resolve(__dirname, "kafkaBridge.js");
  const worker = new Worker(bridgePath);

  worker.on("message", ({ topic, payload }) => {
    try {
      const { message: msg, ts, response } = payload;
      const device = msg?.device_hash || payload.device_hash;
      if (!device) return;

      const summary = {
        type: "chat_summary",
        data: {
          chat_id: msg?.chat_id || msg?.session_id,
          session_id: msg?.session_id,
          device_hash: device,
          preview: msg?.text?.slice(0, 80) || response?.slice(0, 80) || "",
          ts,
          source: topic,
        },
      };  

      const json = JSON.stringify(summary);
      const clients = deviceClients.get(device);
      if (!clients) return;

      for (const ws of clients) {
        if (ws.readyState === WebSocket.OPEN) ws.send(json);
      }
    } catch (err) {
      log.err(`Kafka bridge relay error: `);
    }
  });

  worker.on("error", (err) => {
    log.err("Kafka bridge worker error: " + err.message);
  });

  log.ok("🪣 Kafka bridge worker started");
}

export async function startSocketServer(unixPath: string, port: number) {
  await ensureKafka();
  await waitForSocket(unixPath);

  const server = http.createServer((_, res) => {
    res.writeHead(200, { "Content-Type": "text/plain" });
    res.end(`BA LoRA proxy alive on ${port}\n`);
  });

  const wss = new WebSocketServer({ server });

  // Start Kafka bridge in its own worker thread
  startKafkaBridge();

  wss.on("connection", (ws: WebSocket) => {
    const clientId = randomUUID();
    (ws as any).clientId = clientId;

    log.ok(` WebSocket client connected (port ${port}) → clientId=${clientId}`);

    ws.on("message", async (msg: WebSocket.RawData) => {
      const raw = msg.toString();

      let data: any;
      try {
        data = JSON.parse(raw);
      } catch {
        log.err("❌ Invalid JSON from WS client");
        return;
      }

      // ✅ Device registration
      if (data.type === "register" && data.device_hash) {
        const device = data.device_hash;
        const set = deviceClients.get(device) || new Set<WebSocket>();
        set.add(ws);
        deviceClients.set(device, set);
        (ws as any).deviceHash = device;
        log.ok(`📱 Registered device hash: ${device}`);
        return;
      }

      // 🔄 Normal inference message handling
      log.info(` From WS client (${clientId}) → worker [${unixPath}]: ${raw}`);

      const sock = net.createConnection(unixPath);
      sock.setKeepAlive(true, 5000);
      sock.write(raw + "\n");

      // Emit user message to Kafka
// Emit user message to Kafka
// 🪣 Emit user message to Kafka safely
try {
  if (!producer) {
    log.err("⚠️ Kafka producer not initialized");
  } else {
    // 🧠 Extract useful fields
    const userText =
      data.text ||
      data.message?.text ||
      data.prompt ||
      "";

    // 🧩 Simple local summary fallback (3–4 words)
    const makeSummary = (text: string) => {
      const clean = text.replace(/[^a-zA-Z0-9\s]/g, "").trim();
      const words = clean.split(/\s+/).slice(0, 4);
      return words.length ? words.join(" ") : "General request";
    };

    const summaryValue =
      data.summary ||
      data.message?.summary ||
      makeSummary(userText);

    const kafkaMessage = {
      client_id: clientId,
      message: data,
      summary: summaryValue,
      ts: Date.now(),
      device_hash: (ws as any).deviceHash || null,
    };

    await producer.send({
      topic: "user_messages",
      messages: [
        {
          key: clientId,
          value: JSON.stringify(kafkaMessage),
        },
      ],
    });

    log.ok(`🪣 Kafka → user_messages emitted (summary="${summaryValue}")`);
  }
} catch (err: any) {
  log.err(`Kafka emit failed (user_messages): ${err.message}`);
}



      let buffer = "";
      let fullResponse = "";

      sock.on("data", (chunk) => {
        buffer += chunk.toString();
        const parts = buffer.split("\n");
        buffer = parts.pop()!;

        for (const line of parts) {
          const trimmed = line.trim();
          if (!trimmed) continue;

          let msgOut: string;
          if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
            msgOut = trimmed;
          } else {
            msgOut = JSON.stringify({ token: trimmed });
          }

          try {
            ws.send(msgOut);
            fullResponse += trimmed + " ";
            if (trimmed.includes('"done"') || trimmed === "[DONE]") sock.end();
          } catch (err: any) {
            log.err(`Error sending WS chunk: ${err.message}`);
          }
        }
      });

      sock.on("end", async () => {
        if (fullResponse.trim().length > 0 && producer) {
          try {
            await producer.send({
              topic: "assistant_responses",
              messages: [
                {
                  key: clientId,
                  value: JSON.stringify({
                    client_id: clientId,
                    response: fullResponse.trim(),
                    ts: Date.now(),
                    device_hash: (ws as any).deviceHash || null,
                  }),
                },
              ],
            });
            log.ok("🪣 Kafka → assistant_responses emitted");
          } catch (err: any) {
            log.err(`Kafka emit failed (assistant_responses): ${err.message}`);
          }
        }
      });

      sock.on("error", (err) => {
        log.err(` Worker socket error (${unixPath}): ${err.message}`);
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ error: err.message }));
          ws.close();
        }
      });

      ws.on("close", () => {
        const device = (ws as any).deviceHash;
        if (device && deviceClients.has(device)) {
          const set = deviceClients.get(device)!;
          set.delete(ws);
          if (set.size === 0) deviceClients.delete(device);
          log.warn(`WS disconnected → removed from ${device}`);
        }
        sock.destroy();
      });
    });
  });

  server.listen(port, "0.0.0.0", () => {
    log.ok(` Proxy WebSocket listening on port ${port} → ${unixPath}`);
  });

  return { wss, server };
}
