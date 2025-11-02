// src/index.ts

import dotenv from "dotenv";
import https from "https";
import { spawn } from "child_process";
import fs from "fs";
import path from "path";
// 🧩 Load .env file from project root
dotenv.config({ path: path.resolve(__dirname, "../.env") });

import { startSocketServer } from "./sockets/websocket";
import { EphemeralTunnelManager } from "./tunnel";
import { log } from "./utils/logger";

/**
 * Check if the given tunnel URL is responding (alive).
 */
async function checkTunnelAlive(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    try {
      https
        .get(url, (res) => {
          resolve(res.statusCode !== undefined && res.statusCode < 500);
        })
        .on("error", () => resolve(false));
    } catch {
      resolve(false);
    }
  });
}

/**
 * Start a named Cloudflare tunnel process if not already running.
 */

export function startNamedTunnel(name = "inference") {
  const home = process.env.CLOUDFLARED_HOME || "/home/yaro/.cloudflared";
  const cfgPath = path.join(home, "config.yml");
  const credPath = path.join(home, `${name}.json`);

  // Sanity checks before spawning
  if (!fs.existsSync(cfgPath)) {
    log.err(`❌ Missing Cloudflare config at ${cfgPath}`);
    return null;
  }
  if (!fs.existsSync(credPath)) {
    log.err(`❌ Missing credentials file at ${credPath}`);
    log.info(
      `💡 If you've symlinked the credentials, verify it's readable and named '${name}.json'`
    );
    return null;
  }

  log.info(`🚀 Starting named Cloudflare tunnel '${name}'...`);
  log.info(`🗂  Using config: ${cfgPath}`);
  log.info(`🔐 Using credentials: ${credPath}`);

  const proc = spawn(
    "cloudflared",
    ["--config", cfgPath, "tunnel", "run", name],
    { stdio: ["ignore", "pipe", "pipe"] }
  );

  // Live log the process output
  proc.stdout.on("data", (d) => {
    const line = d.toString().trim();
    log.info(`[cloudflared] ${line}`);
    if (line.includes("Route") || line.includes("Connection established")) {
      log.ok(`✅ Cloudflare tunnel '${name}' initialized`);
    }
  });

  proc.stderr.on("data", (d) =>
    log.warn(`[cloudflared] ${d.toString().trim()}`)
  );

  proc.on("close", (code) =>
    log.warn(`⚠️ cloudflared exited with code ${code}`)
  );

  return proc;
}


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
  log.info(`🚀 Starting socket server on port ${port}`);
  await startSocketServer(socketPath, port);

  // 🌐 Initialize tunnel manager
  const manager = new EphemeralTunnelManager();
  let tunnelUrl: string | null = null;

  // 🧩 Check if a static tunnel is provided via env (from Rust config)
  if (process.env.PUBLIC_TUNNEL) {
    const staticUrl = process.env.PUBLIC_TUNNEL;
    log.info(`🌐 Using static tunnel from config: ${staticUrl}`);

    const alive = await checkTunnelAlive(staticUrl);
    if (!alive) {
      log.warn(`⚠️ Configured tunnel appears offline — starting 'cloudflared tunnel run inference'...`);
      startNamedTunnel("inference");
      log.info("⏳ Waiting 5s for Cloudflare tunnel to initialize...");
      await new Promise((r) => setTimeout(r, 5000));
    } else {
      log.ok(`✅ Verified ${staticUrl} is responding`);
    }

    tunnelUrl = staticUrl;
  } else {
    // 🧠 Tunnel creation modes
    if (tunnelMode === "instant") {
      log.info("⚡ Instant mode: Creating quick tunnel...");
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
        log.err(`❌ Failed to create normal tunnel: ${(err as Error).message}`);
      }
    } else {
      log.warn(`⚠️ Unknown TUNNEL_MODE="${tunnelMode}", skipping tunnel creation.`);
    }
  }

  // 📡 Report tunnel URL
  if (tunnelUrl) {
    log.ok(`✅ Accessible at: ${tunnelUrl}`);
    if (permanent) log.info("🧱 Permanent tunnel: will not auto-cleanup on exit.");
  }

  // 🧹 Cleanup logic
  if (!permanent) {
    const cleanup = async () => {
      log.warn("🧹 Cleaning up tunnels...");
      await manager.cleanupAll();
      process.exit(0);
    };
    process.on("SIGINT", cleanup);
    process.on("SIGTERM", cleanup);
  } else {
    log.info("ℹ️ Permanent mode: skipping cleanup on exit.");
  }
}

// 🏁 Entry point
main().catch((err) => {
  log.err(`❌ Fatal error: ${(err as Error).message}`);
  process.exit(1);
});
