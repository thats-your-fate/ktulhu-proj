import dotenv from "dotenv";
import path from "path";
import { log } from "./utils/logger";
import { loadEnv } from "./config/env";
import { startSocketServer } from "./sockets/websocket";

// 🧩 Load .env early
dotenv.config({ path: path.resolve("/srv/mistral/ktulhuUpgarade/.env") });

async function main() {
  log.info("🧠 Ktulhu Inference Gateway — startup sequence initiated");

  // ⚙️ Load configuration
  const env = loadEnv();
  log.info("🔧 Environment configuration:");
  log.info(JSON.stringify(env, null, 2));

  // 🧩 Ensure socket path exists
  if (!env.socketPath) {
    log.err("❌ Missing unix socket path argument — aborting startup.");
    process.exit(1);
  }

  // 🚀 Start WebSocket / Kafka proxy server
  log.info(`🚀 Launching socket server on port ${env.port}...`);
  await startSocketServer(env.socketPath, env.port);
  log.ok(`✅ Socket server active on port ${env.port}`);

  // ✅ Startup complete
  log.ok("✅ Ktulhu Inference Gateway ready for connections.");
}

// 🏁 Entry point
main().catch((err) => {
  log.err(`❌ Fatal startup error: ${(err as Error).message}`);
  process.exit(1);
});
