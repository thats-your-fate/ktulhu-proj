import os from "os";
import path from "path";

export const MANIFEST_PATH = "/tmp/ktulhu_tunnels.json";
export const CLOUDFLARED_HOME = path.join(os.homedir(), ".cloudflared");
