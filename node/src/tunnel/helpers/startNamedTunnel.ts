import https from "https";
import { spawn } from "child_process";
import fs from "fs";
import path from "path";
import net from "net";
import { log } from "../../utils/logger";

/* ────────────────────────────────────────────── *
 * Wait until a backend TCP port becomes available
 * ────────────────────────────────────────────── */
async function waitForPort(port: number, host = "127.0.0.1", timeout = 10000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      await new Promise<void>((resolve, reject) => {
        const socket = net.createConnection({ port, host }, () => {
          socket.destroy();
          resolve();
        });
        socket.on("error", reject);
      });
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 300));
    }
  }
  throw new Error(`Port ${port} did not become ready within ${timeout}ms`);
}

/* ────────────────────────────────────────────── *
 * Lightweight HTTPS health check
 * ────────────────────────────────────────────── */
export async function checkTunnelAlive(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    try {
      https
        .get(url, (res) => resolve(!!res.statusCode && res.statusCode < 500))
        .on("error", () => resolve(false));
    } catch {
      resolve(false);
    }
  });
}

/* ────────────────────────────────────────────── *
 * Read tunnel UUID from YAML (first line)
 * ────────────────────────────────────────────── */
function extractTunnelId(yamlPath: string): string | null {
  try {
    const firstLines = fs.readFileSync(yamlPath, "utf8").split("\n").slice(0, 5);
    for (const line of firstLines) {
      const match = line.match(/^tunnel:\s*([0-9a-f-]+)/i);
      if (match) return match[1];
    }
  } catch {}
  return null;
}

/* ────────────────────────────────────────────── *
 * Start a static, pre-created Cloudflare tunnel safely
 * ────────────────────────────────────────────── */
export async function startNamedTunnel(name = "inference", port = 8080) {
  const home = process.env.CLOUDFLARED_HOME || "/home/yaro/.cloudflared";
  const configPath = path.join(home, `${name}.yml`);
  const credentialsPath = path.join(home, `${name}.json`);

  // --- Validate configuration files
  if (!fs.existsSync(configPath)) throw new Error(`Missing config file: ${configPath}`);
  if (!fs.existsSync(credentialsPath)) throw new Error(`Missing credentials file: ${credentialsPath}`);

  // --- Extract the tunnel UUID
  const tunnelId = extractTunnelId(configPath);
  if (!tunnelId) throw new Error(`Could not extract tunnel UUID from ${configPath}`);

  // --- Build URL
  const domain = process.env.TUNNEL_DOMAIN || "example.com";
  const fullUrl = `https://${name}.${domain}`;

  log.info(`🚀 Launching Cloudflare tunnel '${name}' (UUID ${tunnelId}) → localhost:${port}`);
  log.info(`🌐 Expected public URL: ${fullUrl}`);

  // --- Wait for backend service
  try {
    await waitForPort(port);
  } catch (e: any) {
    log.warn(`⚠️ Backend on port ${port} not ready yet: ${e.message}`);
  }

  // --- Spawn tunnel process (static mode only)
  const tunnelProcess = spawn("cloudflared", [
    "tunnel",
    "--config", configPath,
    "--no-autoupdate",
    "run",
    name,
  ], { env: process.env });

  return new Promise<{ url: string; id: string } | null>((resolve, reject) => {
    let resolved = false;

    tunnelProcess.stdout.on("data", (chunk) => {
      const line = chunk.toString().trim();
      if (!line) return;
      log.info(line);

      // Detect successful start
      if (/Registered tunnel connection|Starting tunnel|Connected to Cloudflare/i.test(line) && !resolved) {
        resolved = true;
        log.ok(`✅ Tunnel '${name}' (UUID ${tunnelId}) is active at ${fullUrl}`);
        resolve({ url: fullUrl, id: tunnelId });
      }
    });

    tunnelProcess.stderr.on("data", (chunk) => {
      const msg = chunk.toString().trim();
      if (/INF|WRN/.test(msg)) log.info(`⚙️ cloudflared: ${msg}`);
      else log.warn(`⚠️ cloudflared: ${msg}`);
    });

    tunnelProcess.on("exit", (code) => {
      if (!resolved) {
        if (code === 0) {
          log.ok(`✅ Tunnel '${name}' exited normally`);
          resolve(null);
        } else {
          log.err(`❌ Tunnel '${name}' exited with code ${code}`);
          reject(new Error(`Tunnel '${name}' exited with code ${code}`));
        }
      } else {
        log.warn(`⚙️ Tunnel '${name}' stopped (exit code ${code})`);
      }
    });

    tunnelProcess.on("error", (err) => {
      log.err(`❌ Failed to start tunnel '${name}': ${err.message}`);
      reject(err);
    });

    // --- Graceful cleanup
    const cleanup = () => {
      if (!tunnelProcess.killed) {
        tunnelProcess.kill("SIGTERM");
        log.info(`🧹 Shutting down tunnel '${name}'`);
      }
    };
    process.on("SIGINT", cleanup);
    process.on("SIGTERM", cleanup);
  });
}
