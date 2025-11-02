import WebSocket from "ws";
import { ensureKafka } from "./client";
import { log } from "../utils/logger";
import { ChatMessage } from "../types";

/** Safely prepare a message for Kafka */
function safeSerializeMessage(msg: ChatMessage): string {
  // just remove null bytes; let JSON.stringify handle escaping
  const safeMsg = { ...msg };
  if (typeof safeMsg.text === "string") {
    safeMsg.text = safeMsg.text.replace(/\u0000/g, "");
  }
  if (typeof safeMsg.summary === "string") {
    safeMsg.summary = safeMsg.summary.replace(/\u0000/g, "");
  }
  return JSON.stringify(safeMsg);
}

export async function emitMessageToKafka(
  msg: ChatMessage,
  topic = "messages"
): Promise<void> {
  try {
    const producer = await ensureKafka();
    const serialized = safeSerializeMessage(msg);

    await producer.send({
      topic,
      messages: [{ key: msg.chat_id, value: serialized }],
    });

    log.ok(
      `🪣 Kafka → ${topic} (${msg.role}: ${
        msg.text?.slice(0, 60) ?? msg.summary?.slice(0, 60)
      }...)`
    );
  } catch (err: any) {
    log.err(`Kafka emit failed (${msg.role}): ${err.message}`);
  }
}

export async function emitSummaryBroadcast(
  ws: WebSocket,
  summary: string,
  data: any,
  chatId: string,
  deviceClients: Map<string, Set<WebSocket>>
) {
  const event: ChatMessage = {
    role: "summary",
    chat_id: chatId,
    session_id: data.session_id,
    device_hash: (ws as any).deviceHash || null,
    summary,
    ts: Date.now(),
  };

  await emitMessageToKafka(event);

  const device = (ws as any).deviceHash;
  const payload = JSON.stringify({
    type: "chat_summary",
    data: { chat_id: chatId, summary, ts: Date.now(), source: "inference" },
  });

  for (const client of deviceClients.get(device) ?? []) {
    if (client.readyState === WebSocket.OPEN) client.send(payload);
  }
}
