import { startNamedTunnel } from "./helpers/startNamedTunnel"; // Import startNamedTunnel directly
import { log } from "../utils/logger";
import { EnvConfig } from "../config/env";

export async function setupTunnels(env: EnvConfig) {
  const { subdomain, domain, port, secondaryPort, secondarySubdomain } = env;

  // --- Start Primary Tunnel ---
  const primarySubdomain = subdomain || "inference"; // Default to "inference" if subdomain is not provided
  log.info(`🌐 Starting primary tunnel '${primarySubdomain}.${domain}' on port ${port}`);

  let primaryTunnelUrl = "";
  try {
    const primaryTunnel = await startNamedTunnel(primarySubdomain, port);
    primaryTunnelUrl = primaryTunnel?.url || ""; // Now accessing .url safely
    log.ok(`🧠 Primary tunnel online: ${primaryTunnelUrl}`);
  } catch (err) {
    log.err(`❌ Failed to start primary tunnel: ${(err as Error).message}`);
    process.exit(1);
  }

  // --- Start Secondary Tunnel (if applicable) ---
  if (secondaryPort && domain) {
    const secondaryTunnelSubdomain = secondarySubdomain || process.env.SECONDARY_TUNNEL_SUBDOMAIN || "persistence";
    log.info(`🌐 Starting secondary tunnel '${secondaryTunnelSubdomain}.${domain}' on port ${secondaryPort}`);

    try {
      const secondaryTunnel = await startNamedTunnel(secondaryTunnelSubdomain, secondaryPort);
      log.ok(`📦 Secondary tunnel online: ${secondaryTunnel?.url || ""}`);
    } catch (e) {
      log.err(`❌ Failed to start secondary tunnel: ${(e as Error).message}`);
    }
  }

  // Return the URL of the primary tunnel
  return { tunnelUrl: primaryTunnelUrl };
}
