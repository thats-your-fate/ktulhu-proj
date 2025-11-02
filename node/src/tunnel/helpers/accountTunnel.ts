import { spawn, execSync } from "child_process";
import fs from "fs";
import path from "path";
import { CLOUDFLARED_HOME } from "../constants";
import { log } from "../../utils/logger";
import type { TunnelInfo } from "../types";

/**
 * Create or reuse an account-bound Cloudflare tunnel.
 * If PUBLIC_TUNNEL is set (from config.json), skip dynamic creation and use that directly.
 */
export async function createAccountTunnel(
  tunnels: Map<string, TunnelInfo>,
  domain: string,
  port: number,
  subdomain?: string
): Promise<TunnelInfo> {
  // ✅ Config override from Rust env injection
  const staticTunnel = process.env.PUBLIC_TUNNEL;
  if (staticTunnel) {
    const id = `static-${Date.now()}`;
    log.info(`🌐 Using static configured tunnel: ${staticTunnel}`);

    const info: TunnelInfo = {
      id,
      url: staticTunnel,
      port,
      mode: "static",
      proc: null as any, // no process to manage
      name: null,
      hostname: staticTunnel.replace(/^https?:\/\//, ""),
    };

    tunnels.set(id, info);
    return info;
  }

  // ✅ No static tunnel → fall back to real account tunnel
  const cert = path.join(CLOUDFLARED_HOME, "cert.pem");
  if (!fs.existsSync(cert)) {
    throw new Error("❌ No ~/.cloudflared/cert.pem found; cannot create account tunnel.");
  }

  // Prefer user-specified subdomain (e.g. "chat1")
  const name = subdomain || `temp-${Math.random().toString(36).slice(2, 8)}`;
  const hostname = `${name}.${domain}`;
  log.info(`🔧 [account] creating tunnel '${name}' → ${hostname}`);

  let credentialsFile: string;

  try {
    const out = execSync(`cloudflared tunnel create ${name}`, { cwd: CLOUDFLARED_HOME }).toString();
    const credMatch = out.match(/Tunnel credentials written to (.+\.json)/);
    if (!credMatch) throw new Error("Failed to parse tunnel credentials path");
    credentialsFile = credMatch[1];
    log.ok(`✅ Created new Cloudflare tunnel: ${name}`);
  } catch (err: any) {
    const msg = err.message || "";
    if (msg.includes("already exists")) {
      log.warn(`⚠️ Tunnel '${name}' already exists — reusing existing credentials.`);
      credentialsFile = path.join(CLOUDFLARED_HOME, `${name}.json`);
    } else {
      throw err;
    }
  }

  // Write Cloudflare config for the tunnel
  const cfgPath = path.join(CLOUDFLARED_HOME, `${name}.yml`);
  const cfg = [
    `tunnel: ${name}`,
    `credentials-file: ${credentialsFile}`,
    "",
    "ingress:",
    `  - hostname: ${hostname}`,
    `    service: http://localhost:${port}`,
    "  - service: http_status:404",
    "",
  ].join("\n");

  fs.writeFileSync(cfgPath, cfg);
  log.info(`📝 Wrote config: ${cfgPath}`);

  // Ensure DNS route exists (CNAME in your Cloudflare zone)
  try {
    execSync(`cloudflared tunnel route dns ${name} ${hostname}`, { cwd: CLOUDFLARED_HOME });
    log.ok(`🌐 Added/verified CNAME for ${hostname}`);
  } catch (dnsErr: any) {
    log.warn(`⚠️ DNS route may already exist: ${dnsErr.message}`);
  }

  // Start tunnel process
  const proc = spawn("cloudflared", ["--config", cfgPath, "tunnel", "run", name], {
    cwd: CLOUDFLARED_HOME,
    stdio: ["ignore", "pipe", "pipe"],
  });

  const info: TunnelInfo = {
    id: name,
    url: `https://${hostname}`,
    port,
    proc,
    mode: "account",
    name,
    hostname,
  };

  tunnels.set(name, info);

  proc.stdout!.on("data", (d) => log.info(`[${name}] ${d.toString().trim()}`));
  proc.stderr!.on("data", (d) => log.warn(`[${name}] ${d.toString().trim()}`));
  proc.on("close", () => {
    log.warn(`[${name}] account tunnel closed`);
    tunnels.delete(name);
  });

  log.ok(`✅ [${name}] account tunnel started: ${info.url}`);
  return info;
}
