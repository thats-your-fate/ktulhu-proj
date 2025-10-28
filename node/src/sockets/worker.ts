import net from "net";
import WebSocket from "ws";
import { log } from "../utils/logger";
import { emitMessageToKafka, emitSummaryBroadcast } from "../kafka/emitters";

export function handleWorkerStream(
  sock: net.Socket,
  ws: WebSocket,
  data: any,
  chatId: string,
  deviceClients: Map<string, Set<WebSocket>>
) {
  let buffer = "";
  let fullResponse = "";

  sock.on("data", (chunk) => {
    buffer += chunk.toString();
    const parts = buffer.split("\n");
    buffer = parts.pop()!;

    for (const line of parts) {
      const trimmed = line.trim();
      if (!trimmed) continue;

      try {
        const parsed = JSON.parse(trimmed);

        if (parsed.summary) {
          emitSummaryBroadcast(ws, parsed.summary, data, chatId, deviceClients);
          continue;
        }

        if (parsed.token) fullResponse += parsed.token;
        ws.send(JSON.stringify(parsed));

        if (parsed.done) sock.end();
      } catch {
        fullResponse += trimmed;
        ws.send(JSON.stringify({ token: trimmed }));
      }
    }
  });

  sock.on("end", async () => {
    const cleaned = fullResponse.replace(/\s+/g, " ").trim();
    if (cleaned)
      await emitMessageToKafka({
        role: "assistant",
        chat_id: chatId,
        session_id: data.session_id,
        device_hash: (ws as any).deviceHash || null,
        text: cleaned,
        ts: Date.now(),
      });
  });

  sock.on("error", (err) => {
    log.err(` Worker socket error: ${err.message}`);
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ error: err.message }));
      ws.close();
    }
  });
}
