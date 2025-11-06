import { spawn, ChildProcess } from "child_process";
import { log } from "../../utils/logger";
import { writeManifest } from "./manifest";
import type { TunnelInfo } from "../types";

/**
 * Create a quick or static Cloudflare tunnel (trycloudflare.com)
 */
export async function createQuickTunnel(
  tunnels: Map<string, TunnelInfo>,
  port: number,
  subdomain?: string,
  timeoutMs = 15000
): Promise<TunnelInfo> {
  // ✅ 1. Check for static tunnel from env
  const staticTunnel = process.env.PUBLIC_TUNNEL;
  if (staticTunnel) {
    const id = `static-${Date.now()}`;
    const hostname = staticTunnel.replace(/^https?:\/\//, "");
    const info: TunnelInfo = {
      id,
      name: subdomain || "static",
      hostname,
      url: staticTunnel,
      port,
      mode: "static",
      proc: undefined, // ✅ no null
      running: true,
    };

    log.info(`🌐 Using static configured tunnel: ${staticTunnel}`);
    tunnels.set(id, info);
    writeManifest(tunnels);
    return info;
  }

  // ✅ 2. Create a new quick tunnel (trycloudflare.com)
  const id = Math.random().toString(36).slice(2, 10);
  const label = subdomain ? `${subdomain} (${id})` : id;
  log.info(`🌐 [${label}] creating quick tunnel for http://localhost:${port}`);

  const proc: ChildProcess = spawn("cloudflared", ["tunnel", "--url", `http://localhost:${port}`], {
    stdio: ["ignore", "pipe", "pipe"],
  });

  return new Promise<TunnelInfo>((resolve, reject) => {
    let resolved = false;

    const finish = (info: TunnelInfo) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(timer);
      tunnels.set(id, info);
      writeManifest(tunnels);
      resolve(info);
    };

    const handleLine = (src: string, data: Buffer) => {
      const line = data.toString().trim();
      const match = line.match(/https:\/\/[^\s]+trycloudflare\.com/);
      if (match) {
        const url = match[0];
        const hostname = url.replace(/^https?:\/\//, "");
        log.ok(`🌍 [${label}] quick tunnel ready: ${url}`);
        finish({
          id,
          name: subdomain || "quick",
          hostname,
          url,
          port,
          proc,
          mode: "quick",
          running: true,
        });
      } else {
        log.info(`[${label}] ${src}: ${line}`);
      }
    };

    const timer = setTimeout(() => {
      if (!resolved) {
        const fallbackUrl = `https://pending-${id}.trycloudflare.com`;
        log.warn(`[${label}] no URL published within ${timeoutMs} ms — using ${fallbackUrl}`);
        finish({
          id,
          name: subdomain || "quick",
          hostname: fallbackUrl.replace(/^https?:\/\//, ""),
          url: fallbackUrl,
          port,
          proc,
          mode: "quick",
          running: false,
        });
      }
    }, timeoutMs);

    proc.stdout?.on("data", (d) => handleLine("stdout", d));
    proc.stderr?.on("data", (d) => handleLine("stderr", d));

    proc.on("error", (err) => {
      if (!resolved) {
        clearTimeout(timer);
        reject(err);
      }
    });

    proc.on("close", (code) => {
      log.warn(`[${label}] cloudflared closed (code ${code})`);
      tunnels.delete(id);
      writeManifest(tunnels);
    });
  });
}
