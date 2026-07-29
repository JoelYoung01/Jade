import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

import { isTauriRuntime } from "@/lib/runtime";

export type UpdatePrompt =
  | { kind: "available"; version: string; notes: string | null }
  | { kind: "upToDate" }
  | { kind: "error"; message: string }
  | { kind: "linuxPackageManager" }
  | { kind: "installing"; version: string }
  | { kind: "checking" };

export function supportsInAppUpdater(): boolean {
  return isTauriRuntime() && navigator.userAgent.includes("Windows");
}

export function isLinuxDesktop(): boolean {
  return isTauriRuntime() && /Linux/i.test(navigator.userAgent);
}

export async function checkForAppUpdate(): Promise<Update | null> {
  return check();
}

export async function downloadAndInstallUpdate(update: Update): Promise<void> {
  await update.downloadAndInstall();
  await relaunch();
}
