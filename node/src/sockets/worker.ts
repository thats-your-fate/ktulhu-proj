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

    let newlineIndex;
    while ((newlineIndex = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, newlineIndex);   // ❗ NO TRIM
      buffer = buffer.slice(newlineIndex + 1);

      if (!line.trim()) continue; // skip only pure whitespace

      let parsed: any;

      try {
        parsed = JSON.parse(line);
      } catch {
        log.warn("⚠️ Incomplete JSON chunk, waiting for more");
        // ❗ put line back and wait for next call
        buffer = line + "\n" + buffer;
        return;
      }

      // SUMMARY
      if (parsed.summary) {
        emitSummaryBroadcast(ws, parsed.summary, data, chatId, deviceClients);
        continue;
      }

      // STREAM TOKENS
if (parsed.token !== undefined) {
 // console.log("RAW TOKEN:", JSON.stringify(parsed.token));

  hasStarted = true;

  const decoded = parsed.token


  fullResponse += decoded;
}

      // FORWARD TO BROWSER
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(parsed));
      }

      // DONE
      if (parsed.done) {
        sock.end();
      }
    }
  });

  sock.on("end", async () => {
    if (!hasStarted) {
      log.warn("⚠️ Worker ended without producing tokens");
      return;
    }

    const cleaned = fullResponse; // ❗ DO NOT TRIM

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
