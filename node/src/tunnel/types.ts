import { ChildProcess } from "child_process";

export type TunnelMode = "quick" | "account";

export interface TunnelInfo {
  id: string;
  url: string | null;
  port: number;
  proc: ChildProcess;
  mode: TunnelMode;
  name?: string;
  hostname?: string;
}
