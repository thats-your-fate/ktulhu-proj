import dotenv from "dotenv";
import path from "path";

// Load environment variables from the `.env` file
dotenv.config({ path: path.resolve(__dirname, "../../.env") });

export interface EnvConfig {
  socketPath: string | null;
  port: number;
  tunnelMode: "instant" | "normal" | string;
  useAccount: boolean;
  domain?: string;
  subdomain?: string;
  tunnelPort?: number; // Add this if it's not defined yet
  tunnelSubdomain?: string;  // Add this to fix the error
  permanent: boolean;
  publicTunnel?: string;

  // Secondary tunnel
  secondaryPort?: number;
  secondarySubdomain?: string;
}




export function loadEnv(): EnvConfig {
  return {
    socketPath: process.argv[2] || null,

    // Primary tunnel
    port: parseInt(process.env.TUNNEL_PORT || "30823", 10),
    tunnelMode: (process.env.TUNNEL_MODE || "instant").toLowerCase(),
    useAccount:
      process.env.USE_ACCOUNT_TUNNELS === "1" ||
      process.env.USE_ACCOUNT_TUNNELS === "true",
    domain: process.env.TUNNEL_DOMAIN,
    subdomain: process.env.TUNNEL_SUBDOMAIN,
    tunnelSubdomain: process.env.TUNNEL_SUBDOMAIN || "inference",  // Default to "inference"
    permanent:
      process.env.TUNNEL_PERMANENT === "1" ||
      process.env.TUNNEL_PERMANENT === "true",
    publicTunnel: process.env.PUBLIC_TUNNEL,

    // Secondary tunnel
    secondaryPort: process.env.SECONDARY_TUNNEL_PORT
      ? parseInt(process.env.SECONDARY_TUNNEL_PORT, 10)
      : undefined,
    secondarySubdomain: process.env.SECONDARY_TUNNEL_SUBDOMAIN || "persistence", // Default to "persistence"
  };
}


