import dotenv from "dotenv";
import path from "path";
import { log } from "./utils/logger";

import { startSocketServer } from "./sockets/websocket";

// 🧩 Load .env early
dotenv.config({ path: path.resolve("/srv/mistral/ktulhuUpgarade/.env") });

async function main() {
  log.info(" Ktulhu Inference Gateway — startup sequence initiated");




  // 🚀 Start WebSocket / Kafka proxy server
  log.info(`🚀 Launching socket server on port 30823`);
  await startSocketServer( "/tmp/infer_b.sock", 30823);
  log.ok(`✅ Socket server active on port 30823`);

  // ✅ Startup complete
  log.ok("✅ Ktulhu Inference Gateway ready for connections.");
}

// 🏁 Entry point
main().catch((err) => {
  log.err(`❌ Fatal startup error: ${(err as Error).message}`);
  process.exit(1);
});
