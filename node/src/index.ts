// src/index.ts
import { startSocketServer } from "./wsServer";
import { EphemeralTunnelManager } from "./tunnel";
import { log } from "./utils/logger";

async function main() {
  const socketPath = process.argv[2];
  const port = 30823; // your local service port
  if (!socketPath) {
    log.err("Missing unix socket path arg");
    process.exit(1);
  }

  await startSocketServer(socketPath, 30823);
  const manager = new EphemeralTunnelManager();

  // Create an ephemeral quick tunnel for port
  const t = await manager.create(port); // quick by default
  log.ok(`Accessible at: ${t.url}`);

  // Example: create an account-bound (ktulhu.com) tunnel if you enable USE_ACCOUNT_TUNNELS=1
  // const t2 = await manager.create("ktulhu.com", 30500);

  process.on("SIGINT", async () => { await manager.cleanupAll(); process.exit(0); });
  process.on("SIGTERM", async () => { await manager.cleanupAll(); process.exit(0); });
}

main().catch(err => { console.error(err); process.exit(1); });
