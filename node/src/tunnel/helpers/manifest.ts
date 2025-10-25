import fs from "fs";
import { MANIFEST_PATH } from "../constants";
import { TunnelInfo } from "../types";
import { log } from "../../utils/logger";

export function writeManifest(tunnels: Map<string, TunnelInfo>) {
  const arr = Array.from(tunnels.values()).map((t) => ({
    id: t.id,
    url: t.url,
    port: t.port,
    mode: t.mode,
    name: t.name,
    hostname: t.hostname,
  }));

  try {
    fs.writeFileSync(MANIFEST_PATH, JSON.stringify({ tunnels: arr }, null, 2));
  } catch (e) {
    log.warn("Failed to write manifest: " + (e as Error).message);
  }
}

export function initManifest() {
  try {
    fs.writeFileSync(MANIFEST_PATH, JSON.stringify({ tunnels: [] }, null, 2));
  } catch {}
}
