import WebSocket from "ws";
import { ensureKafka } from "./client";
import { log } from "../utils/logger";
import { ChatMessage } from "../types";

import { withIdAndTimestamp } from "../utils/withIdAndTimestamp";


/** Safely prepare a message for Kafka */
function safeSerializeMessage(msg: ChatMessage): string {
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


export async function emitStateDelta(delta: any) {
  const producer = await ensureKafka();
   producer.send({
    topic: "conversation_state_delta",
    messages: [{ value: JSON.stringify(delta) }],
  });
}



export async function emitSummaryBroadcast(
  ws: WebSocket,
  summary: string,
  data: any,
  chatId: string,
  deviceClients: Map<string, Set<WebSocket>>
) {
  // Add UUID + timestamp to summary event
  const event: ChatMessage = withIdAndTimestamp({
    role: "summary",
    chat_id: chatId,
    session_id: data.session_id,
    device_hash: (ws as any).deviceHash || null,
    summary,
  });

  await emitMessageToKafka(event);

  const device = (ws as any).deviceHash;
  const payload = JSON.stringify({
    type: "chat_summary",
    data: {
      id: event.id,
      chat_id: chatId,
      summary,
      ts: event.ts,
      source: "inference",
    },
  });

  for (const client of deviceClients.get(device) ?? []) {
    if (client.readyState === WebSocket.OPEN) client.send(payload);
  }
}
