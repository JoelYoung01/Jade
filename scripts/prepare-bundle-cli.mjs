/**
 * Build the `jade` CLI into the workspace release target before Linux bundling.
 * No-op on non-Linux hosts (Windows NSIS does not embed the CLI yet).
 *
 * Invoked from apps/desktop via tauri.conf.json `beforeBundleCommand`.
 */
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const platform = process.env.TAURI_ENV_PLATFORM ?? process.platform;
const isLinux = platform === "linux";

if (!isLinux) {
  process.exit(0);
}

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const cliBin = path.join(repoRoot, "target", "release", "jade");

console.log("Building jade-cli (release) for Linux package embedding…");
const result = spawnSync("cargo", ["build", "-p", "jade-cli", "--release"], {
  cwd: repoRoot,
  stdio: "inherit",
  env: process.env,
});

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

if (!existsSync(cliBin)) {
  console.error(`Expected CLI binary missing after build: ${cliBin}`);
  process.exit(1);
}

console.log(`CLI ready: ${cliBin}`);
