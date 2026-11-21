import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const from = resolve(root, "node_modules/vditor/dist");
const to = resolve(root, "public/vditor/dist");

if (!existsSync(from)) {
  console.log("[sync-vditor] vditor dist not found, skipped");
  process.exit(0);
}

rmSync(to, { recursive: true, force: true });
mkdirSync(dirname(to), { recursive: true });
cpSync(from, to, { recursive: true });
console.log(`[sync-vditor] copied to ${to}`);