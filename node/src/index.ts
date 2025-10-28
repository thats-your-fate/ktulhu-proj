// src/index.ts
import path from "path";
import dotenv from "dotenv";

// 🧩 Load .env file from project root
dotenv.config({ path: path.resolve(__dirname, "../.env") });

import { startSocketServer } from "./sockets/websocket";
import { EphemeralTunnelManager } from "./tunnel";
import { log } from "./utils/logger";

async function main() {
  const socketPath = process.argv[2];
  if (!socketPath) {
    log.err("Missing unix socket path arg");
    process.exit(1);
  }

  // 🔧 Read environment variables
  const port = parseInt(process.env.TUNNEL_PORT || "30823", 10);
  const tunnelMode = (process.env.TUNNEL_MODE || "instant").toLowerCase();
  const useAccount =
    process.env.USE_ACCOUNT_TUNNELS === "1" ||
    process.env.USE_ACCOUNT_TUNNELS === "true";
  const domain = process.env.TUNNEL_DOMAIN;
  const subdomain = process.env.TUNNEL_SUBDOMAIN;
  const permanent =
    process.env.TUNNEL_PERMANENT === "1" ||
    process.env.TUNNEL_PERMANENT === "true";

  log.info("🔧 Loaded env configuration:");
  log.info(
    JSON.stringify(
      {
        TUNNEL_MODE: tunnelMode,
        USE_ACCOUNT_TUNNELS: useAccount,
        TUNNEL_DOMAIN: domain,
        TUNNEL_SUBDOMAIN: subdomain,
        TUNNEL_PORT: port,
        TUNNEL_PERMANENT: permanent,
      },
      null,
      2
    )
  );

  // 🚀 Start WebSocket + Kafka proxy server
  log.info(` Starting socket server on port ${port}`);
  await startSocketServer(socketPath, port);

  // 🌐 Initialize tunnel manager
  const manager = new EphemeralTunnelManager();
  let tunnelUrl: string | null = null;

  // 🧠 Tunnel creation modes
  if (tunnelMode === "instant") {
    log.info(" Instant mode: Creating quick tunnel...");
    const t = await manager.create(port);
    tunnelUrl = t.url;
  } else if (tunnelMode === "normal") {
    log.info(
      ` Normal mode: account=${useAccount}, domain=${domain}, sub=${subdomain}`
    );
    try {
      if (useAccount && domain) {
        const fqdn = subdomain ? `${subdomain}.${domain}` : domain;
        log.info(` Creating account tunnel for ${fqdn}`);
        const t = await manager.create(domain, port);
        tunnelUrl = t.url;
      } else {
        const t = await manager.create(port);
        tunnelUrl = t.url;
      }
    } catch (err) {
      log.err(` Failed to create normal tunnel: ${(err as Error).message}`);
    }
  } else {
    log.warn(` Unknown TUNNEL_MODE="${tunnelMode}", skipping tunnel creation.`);
  }

  // 📡 Report tunnel URL
  if (tunnelUrl) {
    log.ok(` Accessible at: ${tunnelUrl}`);
    if (permanent) log.info(" Permanent tunnel: will not auto-cleanup on exit.");
  }

  // 🧹 Cleanup logic
  if (!permanent) {
    const cleanup = async () => {
      log.warn(" Cleaning up tunnels...");
      await manager.cleanupAll();
      process.exit(0);
    };
    process.on("SIGINT", cleanup);
    process.on("SIGTERM", cleanup);
  } else {
    log.info(" Permanent mode: skipping cleanup on exit.");
  }
}

// 🏁 Entry point
main().catch((err) => {
  log.err(` Fatal error: ${(err as Error).message}`);
  process.exit(1);
});
