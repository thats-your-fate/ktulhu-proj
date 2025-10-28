import fs from "fs";
import path from "path";
import { log } from "./logger";

export async function waitForSocket(unixPath: string, maxAttempts = 30): Promise<void> {
  const absPath = path.resolve(unixPath);
  for (let i = 0; i < maxAttempts; i++) {
    if (fs.existsSync(absPath)) {
      log.ok(` Found worker socket: ${absPath}`);
      return;
    }
    log.warn(`⏳ Waiting for ${absPath} (${i + 1}/${maxAttempts})...`);
    await new Promise((r) => setTimeout(r, 10_000));
  }
  throw new Error(`Timeout waiting for worker socket: ${absPath}`);
}
