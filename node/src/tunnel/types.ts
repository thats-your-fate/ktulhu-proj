import type { ChildProcess } from "child_process";

export interface TunnelInfo {
  id: string;                     // unique ID (tunnel name or random)
  name: string;                   // subdomain or descriptive label
  hostname: string;               // full hostname (e.g. inference.ktulhu.com)
  url: string;                    // public URL (https://...)
  port: number;                   // local port being tunneled
  proc?: ChildProcess;            // optional child process (cloudflared)
  mode: "quick" | "account" | "static"; // ✅ added 'static'
  running?: boolean;              // if tunnel is active
}
