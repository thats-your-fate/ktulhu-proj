import { spawn } from "child_process";
import { log } from "../../utils/logger";
import { TunnelInfo } from "../types";
import { writeManifest } from "./manifest";

export async function createQuickTunnel(
  tunnels: Map<string, TunnelInfo>,
  port: number,
  timeoutMs = 15000
): Promise<TunnelInfo> {
  const id = Math.random().toString(36).slice(2, 10);
  log.info(`🌐 [${id}] creating quick tunnel for http://localhost:${port}`);

  const proc = spawn("cloudflared", ["tunnel", "--url", `http://localhost:${port}`], {
    stdio: ["ignore", "pipe", "pipe"],
  });

  return new Promise((resolve, reject) => {
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
      const s = data.toString();
      const m = s.match(/https:\/\/[^\s]+trycloudflare\.com/);
      if (m) {
        const url = m[0];
        log.ok(`🌍 [${id}] quick tunnel ready: ${url}`);
        finish({ id, url, port, proc, mode: "quick" });
      } else {
        log.info(`[${id}] ${src}: ${s.trim()}`);
      }
    };

    const timer = setTimeout(() => {
      if (!resolved) {
        log.warn(`[${id}] quick tunnel did not publish URL within ${timeoutMs} ms`);
        finish({ id, url: null, port, proc, mode: "quick" });
      }
    }, timeoutMs);

    proc.stdout!.on("data", (d) => handleLine("stdout", d));
    proc.stderr!.on("data", (d) => handleLine("stderr", d));

    proc.on("error", (err) => {
      if (!resolved) {
        clearTimeout(timer);
        reject(err);
      }
    });

    proc.on("close", (code) => {
      log.warn(`[${id}] cloudflared closed (code ${code})`);
      tunnels.delete(id);
      writeManifest(tunnels);
    });
  });
}
