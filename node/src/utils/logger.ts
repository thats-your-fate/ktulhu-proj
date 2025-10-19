import chalk from "chalk";

export const log = {
  info: (msg: string) => console.log(chalk.cyan("ℹ️  " + msg)),
  ok: (msg: string) => console.log(chalk.green("✅ " + msg)),
  warn: (msg: string) => console.log(chalk.yellow("⚠️  " + msg)),
  err: (msg: string) => console.log(chalk.red("❌ " + msg)),
};
