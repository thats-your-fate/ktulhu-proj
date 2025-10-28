import WebSocket from "ws";
import { ensureKafka } from "./client";
import { log } from "../utils/logger";
import { ChatMessage } from "../types";

export async function emitMessageToKafka(
  msg: ChatMessage,
  topic = "messages"
): Promise<void> {
  try {
    const producer = await ensureKafka();
    await producer.send({
      topic,
      messages: [{ key: msg.chat_id, value: JSON.stringify(msg) }],
    });
    log.ok(`🪣 Kafka → ${topic} (${msg.role}: ${msg.text?.slice(0, 60) ?? msg.summary?.slice(0, 60)}...)`);
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
