// src/index.ts
import path from "path";
import dotenv from "dotenv";

//  Load .env file from the project root (one directory up from /src)
dotenv.config({ path: path.resolve(__dirname, "../.env") });

import { startSocketServer } from "./wsServer";
import { EphemeralTunnelManager } from "./tunnel";
import { log } from "./utils/logger";

async function main() {
  const socketPath = process.argv[2];
  if (!socketPath) {
    log.err("Missing unix socket path arg");
    process.exit(1);
  }

  const port = parseInt(process.env.TUNNEL_PORT || "30823", 10);
  const tunnelMode = (process.env.TUNNEL_MODE || "instant").toLowerCase();
  const useAccount =
    process.env.USE_ACCOUNT_TUNNELS === "1" || process.env.USE_ACCOUNT_TUNNELS === "true";
  const domain = process.env.TUNNEL_DOMAIN;
  const subdomain = process.env.TUNNEL_SUBDOMAIN;
  const permanent =
    process.env.TUNNEL_PERMANENT === "1" || process.env.TUNNEL_PERMANENT === "true";

log.info("🔧 Loaded env configuration:");
log.info(JSON.stringify({
  TUNNEL_MODE: tunnelMode,
  USE_ACCOUNT_TUNNELS: useAccount,
  TUNNEL_DOMAIN: domain,
  TUNNEL_SUBDOMAIN: subdomain,
  TUNNEL_PORT: port,
  TUNNEL_PERMANENT: permanent,
}, null, 2));

  log.info(` Starting socket server on port ${port}`);
  await startSocketServer(socketPath, port);

  const manager = new EphemeralTunnelManager();

  // Decide which tunnel creation strategy to use
  let tunnelUrl: string | null = null;

  if (tunnelMode === "instant") {
    log.info(" Instant mode: Creating quick tunnel...");
    const t = await manager.create(port);
    tunnelUrl = t.url;
  } else if (tunnelMode === "normal") {
    log.info(` Normal mode: account=${useAccount}, domain=${domain}, sub=${subdomain}`);
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

  if (tunnelUrl) {
    log.ok(` Accessible at: ${tunnelUrl}`);
    if (permanent) log.info(" Permanent tunnel: will not auto-cleanup on exit.");
  }

  //  Cleanup logic
  if (!permanent) {
    process.on("SIGINT", async () => {
      log.warn("Cleaning up tunnels...");
      await manager.cleanupAll();
      process.exit(0);
    });
    process.on("SIGTERM", async () => {
      log.warn(" Cleaning up tunnels...");
      await manager.cleanupAll();
      process.exit(0);
    });
  } else {
    log.info(" Permanent mode: skipping cleanup on exit.");
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
