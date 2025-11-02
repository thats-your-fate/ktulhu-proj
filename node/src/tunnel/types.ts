import { ChildProcess } from "child_process";

export type TunnelMode = "account" | "quick" | "static";

export interface TunnelInfo {
  id: string;
  url: string | null; // <-- allow null
  port: number;
  mode: "account" | "quick" | "static";
  proc?: ChildProcess | null;
  name?: string | null;
  hostname?: string;
}


