import { createQuickTunnel } from "./helpers/quickTunnel";
import { createAccountTunnel } from "./helpers/accountTunnel";
import { writeManifest, initManifest } from "./helpers/manifest";
import { CLOUDFLARED_HOME } from "./constants";
import { TunnelInfo } from "./types";
import { execSync } from "child_process";
import fs from "fs";
import path from "path";
import { log } from "../utils/logger";

export class EphemeralTunnelManager {
  private tunnels = new Map<string, TunnelInfo>();
  private useAccount =
    process.env.USE_ACCOUNT_TUNNELS === "1" || process.env.USE_ACCOUNT_TUNNELS === "true";

  constructor() {
    initManifest();
  }

  /**
   * Create a Cloudflare tunnel.
   * Supports both quick (trycloudflare.com) and account-based (your domain) tunnels.
   */
  async create(
    domainOrPort: string | number,
    maybePort?: number,
    maybeSubdomain?: string
  ): Promise<TunnelInfo> {
    const envSubdomain = process.env.TUNNEL_SUBDOMAIN;
    const subdomain = maybeSubdomain || envSubdomain;

    if (typeof domainOrPort === "number") {
      // quick mode
      return createQuickTunnel(this.tunnels, domainOrPort);
    }

    const domain = domainOrPort;
    const port = maybePort!;

    if (this.useAccount) {
      try {
        return await createAccountTunnel(this.tunnels, domain, port, subdomain);
      } catch (e) {
        log.warn(
          `⚠️ Account-based tunnel creation failed, falling back to quick mode: ${
            (e as Error).message
          }`
        );
        return createQuickTunnel(this.tunnels, port);
      }
    } else {
      return createQuickTunnel(this.tunnels, port);
    }
  }

  /** Delete a tunnel by ID */
  async delete(id: string) {
    const t = this.tunnels.get(id);
    if (!t) return;

    log.warn(`[${id}] deleting tunnel`);
t.proc?.kill("SIGTERM");

    if (t.mode === "account" && t.name) {
      try {
        execSync(`cloudflared tunnel delete ${t.name} -f`, { cwd: CLOUDFLARED_HOME });
      } catch (e) {
        log.warn(`[${id}] failed to delete account tunnel: ${(e as Error).message}`);
      }

      const cfg = path.join(CLOUDFLARED_HOME, `${t.name}.yml`);
      const creds = path.join(CLOUDFLARED_HOME, `${t.name}.json`);
      [cfg, creds].forEach((f) => fs.existsSync(f) && fs.unlinkSync(f));
    }

    this.tunnels.delete(id);
    writeManifest(this.tunnels);
  }

  list() {
    return Array.from(this.tunnels.values()).map((t) => ({
      id: t.id,
      url: t.url,
      port: t.port,
      mode: t.mode,
    }));
  }

  async cleanupAll() {
    for (const id of Array.from(this.tunnels.keys())) {
      await this.delete(id);
    }
  }
}
