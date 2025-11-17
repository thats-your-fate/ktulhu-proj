import net from "net";
import WebSocket from "ws";
import { log } from "../utils/logger";
import { emitMessageToKafka, emitSummaryBroadcast } from "../kafka/emitters";
import { ChatMessage } from "../types";
import { withIdAndTimestamp } from "../utils/withIdAndTimestamp";

export function handleWorkerStream(
  sock: net.Socket,
  ws: WebSocket,
  data: any,
  chatId: string,
  deviceClients: Map<string, Set<WebSocket>>,
) {
  const uid = data.id;

  let buffer = "";
  let fullResponse = "";
  let hasStarted = false;

  sock.on("data", async (chunk) => {
    buffer += chunk.toString();

    let newlineIndex;
    while ((newlineIndex = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, newlineIndex);
      buffer = buffer.slice(newlineIndex + 1);

      if (!line.trim()) continue;

      let parsed: any;

      try {
        parsed = JSON.parse(line);
      } catch {
        log.warn("⚠️ Incomplete JSON chunk, waiting...");
        buffer = line + "\n" + buffer;
        return;
      }

      // -------------------------
      // 1. SUMMARY
      // -------------------------
      if (parsed.summary) {
        emitSummaryBroadcast(ws, parsed.summary, data, chatId, deviceClients);

        if (ws.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              id: uid,
              type: "summary",
              summary: parsed.summary,
            })
          );
        }
        continue;
      }

      // -------------------------
      // 2. SYSTEM MESSAGE (Kafka + UI)
      // -------------------------
      if (parsed.system) {
        const sysEvent: ChatMessage = withIdAndTimestamp({
          role: "system",
          chat_id: chatId,
          session_id: data.session_id,
          device_hash: (ws as any).deviceHash || null,
          text: parsed.system,
        });

        await emitMessageToKafka(sysEvent);

        if (ws.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              id: uid,
              type: "system",
              system: parsed.system,
            })
          );
        }
        continue;
      }

      // -------------------------
      // 3. STREAM TOKENS
      // -------------------------
      if (parsed.token !== undefined) {
        hasStarted = true;
        fullResponse += parsed.token;

        if (ws.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              id: uid,
              type: "token",
              token: parsed.token,
            })
          );
        }
        continue;
      }

      // -------------------------
      // 4. ERROR
      // -------------------------
      if (parsed.error) {
        log.err(`Worker error: ${parsed.error}`);

        if (ws.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              id: uid,
              type: "error",
              error: parsed.error,
            })
          );
        }
        continue;
      }

      // -------------------------
      // 5. DONE
      // -------------------------
      if (parsed.done) {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(
            JSON.stringify({
              id: uid,
              type: "done",
              final: parsed.final,
              done: true,
            })
          );
        }
        sock.end();
        continue;
      }

      // -------------------------
      // 6. Unknown fallback
      // -------------------------
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(parsed));
      }
    }
  });

  // --------------------------------------------
  // FINAL ASSISTANT MESSAGE (Kafka + history)
  // --------------------------------------------
  sock.on("end", async () => {
    if (!hasStarted) {
      log.warn("⚠️ Worker ended without producing tokens");
      return;
    }

    const cleaned = fullResponse;

    const finalAssistantEvent: ChatMessage = withIdAndTimestamp({
      role: "assistant",
      chat_id: chatId,
      session_id: data.session_id,
      device_hash: (ws as any).deviceHash || null,
      text: cleaned,
    });

    await emitMessageToKafka(finalAssistantEvent);
  });

  sock.on("error", (err) => {
    log.err(`Worker socket error: ${err.message}`);
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ error: err.message }));
      ws.close();
    }
  });
}
