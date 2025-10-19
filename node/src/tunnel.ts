// src/tunnel.ts
import { spawn, execSync, ChildProcess } from "child_process";
import fs from "fs";
import os from "os";
import path from "path";
import { log } from "./utils/logger";

type TunnelInfo = {
  id: string;
  url: string | null;
  port: number;
  proc: ChildProcess;
  mode: "quick" | "account";
  name?: string; // set for account-mode
  hostname?: string; // set for account-mode
};

const MANIFEST_PATH = "/tmp/ktulhu_tunnels.json";

export class EphemeralTunnelManager {
  private tunnels = new Map<string, TunnelInfo>();
  private cloudflaredHome = path.join(os.homedir(), ".cloudflared");
  private useAccount = process.env.USE_ACCOUNT_TUNNELS === "1" || process.env.USE_ACCOUNT_TUNNELS === "true";

  constructor() {
    // initialize manifest file
    try { fs.writeFileSync(MANIFEST_PATH, JSON.stringify({ tunnels: [] }, null, 2)); } catch {}
  }

  private persist() {
    const arr = Array.from(this.tunnels.values()).map(t => ({
      id: t.id, url: t.url, port: t.port, mode: t.mode, name: t.name, hostname: t.hostname
    }));
    try { fs.writeFileSync(MANIFEST_PATH, JSON.stringify({ tunnels: arr }, null, 2)); } catch (e) {
      log.warn("Failed to write manifest: " + (e as Error).message);
    }
  }

  /** Create ephemeral quick tunnel (trycloudflare.com) */
async createQuick(port: number, timeoutMs = 15000): Promise<TunnelInfo> {
  const id = Math.random().toString(36).slice(2, 10);
  log.info(`🌐 [${id}] creating quick tunnel for http://localhost:${port}`);

  const proc = spawn("cloudflared", ["tunnel", "--url", `http://localhost:${port}`], {
    stdio: ["ignore", "pipe", "pipe"],
  });

  return new Promise<TunnelInfo>((resolve, reject) => {
    let resolved = false;
    const finish = (info: TunnelInfo) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(timer);
      this.tunnels.set(id, info);
      this.persist();
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

    proc.stdout!.on("data", d => handleLine("stdout", d));
    proc.stderr!.on("data", d => handleLine("stderr", d));

    proc.on("error", err => {
      if (!resolved) {
        resolved = true;
        clearTimeout(timer);
        reject(err);
      }
    });

    proc.on("close", code => {
      log.warn(`[${id}] cloudflared closed (code ${code})`);
      this.tunnels.delete(id);
      this.persist();
    });
  });
}


  /** Optional: create temporary account-bound tunnel (writes creds) */
  async createAccountTemp(domain: string, port: number): Promise<TunnelInfo> {
    // must have cert.pem
    const cert = path.join(this.cloudflaredHome, "cert.pem");
    if (!fs.existsSync(cert)) {
      throw new Error("No ~/.cloudflared/cert.pem found; cannot create account tunnel.");
    }

    const rand = Math.random().toString(36).slice(2, 8);
    const name = `temp-${rand}`;
    const hostname = `${name}.${domain}`;

    log.info(`🔧 [account] creating ${name} -> ${hostname}`);

    // create tunnel (this writes credentials json)
    const out = execSync(`cloudflared tunnel create ${name}`, { cwd: this.cloudflaredHome }).toString();
    const credMatch = out.match(/Tunnel credentials written to (.+\.json)/);
    if (!credMatch) throw new Error("failed to create tunnel credentials");
    const credentialsFile = credMatch[1];

    // create config for this tunnel
    const cfgPath = path.join(this.cloudflaredHome, `${name}.yml`);
    const cfg = [
      `tunnel: ${name}`,
      `credentials-file: ${credentialsFile}`,
      "",
      "ingress:",
      `  - hostname: ${hostname}`,
      `    service: http://localhost:${port}`,
      "  - service: http_status:404",
      ""
    ].join("\n");
    fs.writeFileSync(cfgPath, cfg);

    // add DNS route (will create CNAME in your Cloudflare zone)
    execSync(`cloudflared tunnel route dns ${name} ${hostname}`, { cwd: this.cloudflaredHome });

    // spawn
    const proc = spawn("cloudflared", ["--config", cfgPath, "tunnel", "run", name], { cwd: this.cloudflaredHome, stdio: ["ignore", "pipe", "pipe"] });

    const id = name;
    const info: TunnelInfo = { id, url: `https://${hostname}`, port, proc, mode: "account", name, hostname };
    this.tunnels.set(id, info);
    this.persist();

    proc.stdout!.on("data", (d: Buffer) => log.info(`[${id}] ${d.toString().trim()}`));
    proc.stderr!.on("data", (d: Buffer) => log.warn(`[${id}] ${d.toString().trim()}`));
    proc.on("close", () => {
      log.warn(`[${id}] account tunnel closed`);
      this.tunnels.delete(id);
      this.persist();
    });

    log.ok(`[${id}] account tunnel started: ${info.url}`);
    return info;
  }

  /** Public create - preferring quick, optional account if configured and requested */
  async create(domainOrPort: string | number, maybePort?: number) {
    // support create(port) or create(domain,port) for account-mode
    if (typeof domainOrPort === "number") {
      // quick mode
      return this.createQuick(domainOrPort);
    }

    const domain = domainOrPort;
    const port = maybePort!;
    if (this.useAccount) {
      try {
        return await this.createAccountTemp(domain, port);
      } catch (e) {
        log.warn("Account-based creation failed, falling back to quick: " + (e as Error).message);
        return this.createQuick(port);
      }
    } else {
      // quick by default (no custom hostname)
      return this.createQuick(port);
    }
  }

  async delete(id: string) {
    const t = this.tunnels.get(id);
    if (!t) return;
    log.warn(`[${id}] deleting tunnel`);
    t.proc.kill("SIGTERM");
    if (t.mode === "account" && t.name) {
      try {
        execSync(`cloudflared tunnel delete ${t.name} -f`, { cwd: this.cloudflaredHome });
      } catch (e) {
        log.warn(`[${id}] failed to delete account tunnel: ${(e as Error).message}`);
      }
      const cfg = path.join(this.cloudflaredHome, `${t.name}.yml`);
      const creds = path.join(this.cloudflaredHome, `${t.name}.json`);
      [cfg, creds].forEach(f => { if (fs.existsSync(f)) fs.unlinkSync(f); });
    }
    this.tunnels.delete(id);
    this.persist();
  }

  list() {
    return Array.from(this.tunnels.values()).map(t => ({ id: t.id, url: t.url, port: t.port, mode: t.mode }));
  }

  async cleanupAll() {
    for (const id of Array.from(this.tunnels.keys())) {
      await this.delete(id);
    }
  }
}
