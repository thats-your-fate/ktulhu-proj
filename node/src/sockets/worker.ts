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
  let hasStarted = false;

  sock.on("data", (chunk) => {
    buffer += chunk.toString();

    // process only full lines
    let newlineIndex;
    while ((newlineIndex = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, newlineIndex).trim();
      buffer = buffer.slice(newlineIndex + 1); // remove processed part

      if (!line) continue;

      let parsed: any = null;

      try {
        parsed = JSON.parse(line);

        // SUMMARY
        if (parsed.summary) {
          emitSummaryBroadcast(ws, parsed.summary, data, chatId, deviceClients);
          continue;
        }

        // STREAM TOKENS
        if (parsed.token) {
          hasStarted = true;
          fullResponse += parsed.token;
        }

        // FORWARD TO BROWSER
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify(parsed));
        }

        // DONE
        if (parsed.done) {
          sock.end();
        }
      } catch {
        // ❌ DO NOT stream broken JSON → skip it entirely
        log.warn("⚠️ Skipping incomplete JSON chunk (TCP split)");
        continue;
      }
    }
  });

  sock.on("end", async () => {
    if (!hasStarted) {
      log.warn("⚠️ Worker ended without producing tokens (probably TCP fragment)");
      return;
    }

    const cleaned = fullResponse.replace(/\s+/g, " ").trim();
    if (!cleaned) return;

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
