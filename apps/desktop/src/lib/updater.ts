import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

import { isTauriRuntime } from "@/lib/runtime";

export type InstallKind = "windows" | "appImage" | "aur" | "deb" | "unknown";

export type InstallContext = {
  kind: InstallKind;
  platform: string;
  distroId: string | null;
  archBased: boolean;
  packageName: string | null;
  konsoleAvailable: boolean;
  yayAvailable: boolean;
  appImageEnv: string | null;
  releasesUrl: string;
};

export type AurPackageInfo = {
  name: string;
  version: string;
  upstreamVersion: string;
};

export type UpdateRouteId = "appImage" | "aur";

export type UpdateRoute = {
  id: UpdateRouteId;
  title: string;
  description: string;
  recommended: boolean;
  recommendedLabel: string | null;
  enabled: boolean;
  disabledReason: string | null;
  actionLabel: string;
  action: "installAppImage" | "downloadAppImage" | "openAurKonsole";
};

export type UpdatePrompt =
  | {
      kind: "available";
      version: string;
      notes: string | null;
      /** Present for Windows / AppImage self-update. */
      canSelfInstall: boolean;
      routes: UpdateRoute[];
    }
  | { kind: "upToDate" }
  | { kind: "error"; message: string }
  | { kind: "installing"; version: string }
  | { kind: "checking" };

const MOCK_WINDOWS_CONTEXT: InstallContext = {
  kind: "windows",
  platform: "windows",
  distroId: null,
  archBased: false,
  packageName: null,
  konsoleAvailable: false,
  yayAvailable: false,
  appImageEnv: null,
  releasesUrl: "https://github.com/JoelYoung01/Jade/releases/latest",
};

export async function getInstallContext(): Promise<InstallContext> {
  if (!isTauriRuntime()) return MOCK_WINDOWS_CONTEXT;
  return invoke<InstallContext>("get_install_context_cmd");
}

export async function fetchAurPackageInfo(): Promise<AurPackageInfo | null> {
  if (!isTauriRuntime()) return null;
  return invoke<AurPackageInfo | null>("fetch_aur_package_info_cmd");
}

export async function openAurUpdateInKonsole(): Promise<void> {
  await invoke("open_aur_update_in_konsole_cmd");
}

export async function openReleasesPage(url: string): Promise<void> {
  if (!isTauriRuntime()) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }
  await openUrl(url);
}

export function usesTauriSelfUpdate(ctx: InstallContext): boolean {
  return ctx.kind === "windows" || ctx.kind === "appImage";
}

export function supportsStartupUpdateCheck(ctx: InstallContext): boolean {
  return ctx.kind === "windows" || ctx.kind === "appImage" || ctx.kind === "aur";
}

export async function checkForAppUpdate(): Promise<Update | null> {
  return check();
}

export async function downloadAndInstallUpdate(update: Update): Promise<void> {
  await update.downloadAndInstall();
  await relaunch();
}

export function parseSemver(raw: string): [number, number, number] | null {
  const s = raw.trim().replace(/^v/i, "");
  const [majorRaw, minorRaw, patchRaw = "0"] = s.split(".");
  const major = Number(majorRaw);
  const minor = Number(minorRaw);
  const patchDigits = /^\d+/.exec(String(patchRaw))?.[0];
  const patch = Number(patchDigits ?? Number.NaN);
  if (![major, minor, patch].every((n) => Number.isFinite(n))) return null;
  return [major, minor, patch];
}

export function isNewerVersion(current: string, remote: string): boolean {
  const c = parseSemver(current);
  const r = parseSemver(remote);
  if (!c || !r) {
    const a = current.trim().replace(/^v/i, "");
    const b = remote.trim().replace(/^v/i, "");
    return b !== a && b > a;
  }
  for (let i = 0; i < 3; i += 1) {
    if (r[i]! > c[i]!) return true;
    if (r[i]! < c[i]!) return false;
  }
  return false;
}

export async function getAppVersion(): Promise<string> {
  if (!isTauriRuntime()) return "0.0.0-dev";
  return getVersion();
}

export function buildLinuxRoutes(args: {
  ctx: InstallContext;
  remoteVersion: string;
  aurRemoteVersion: string | null;
  aurFetchFailed: boolean;
}): UpdateRoute[] {
  const { ctx, remoteVersion, aurRemoteVersion, aurFetchFailed } = args;
  const routes: UpdateRoute[] = [];

  const isAppImage = ctx.kind === "appImage";
  const isAur = ctx.kind === "aur";

  routes.push({
    id: "appImage",
    title: "AppImage",
    description: isAppImage
      ? `Install Jade ${remoteVersion} and restart (in-app updater).`
      : `Download the AppImage build of Jade ${remoteVersion} from GitHub Releases.`,
    recommended: isAppImage || (!isAur && !ctx.archBased),
    recommendedLabel: isAppImage
      ? "Recommended (current install)"
      : !isAur
        ? "Recommended"
        : null,
    enabled: true,
    disabledReason: null,
    actionLabel: isAppImage ? "Update and restart" : "Open downloads",
    action: isAppImage ? "installAppImage" : "downloadAppImage",
  });

  if (ctx.archBased || isAur) {
    let disabledReason: string | null = null;
    let enabled = true;

    if (aurFetchFailed) {
      enabled = false;
      disabledReason = "Could not reach the AUR to check the published package.";
    } else if (!aurRemoteVersion) {
      enabled = false;
      disabledReason =
        "jade-desktop-bin is not published on the AUR yet (or was not found).";
    } else if (!ctx.yayAvailable) {
      enabled = false;
      disabledReason = "yay is not installed.";
    } else if (!ctx.konsoleAvailable) {
      enabled = false;
      disabledReason = "Konsole is not installed (needed to run yay interactively).";
    }

    routes.push({
      id: "aur",
      title: "AUR (yay)",
      description: aurRemoteVersion
        ? `Update to AUR package ${aurRemoteVersion} via yay in Konsole.`
        : "Update via yay once jade-desktop-bin is published on the AUR.",
      recommended: isAur,
      recommendedLabel: isAur ? "Recommended (current install)" : null,
      enabled,
      disabledReason,
      actionLabel: "Open Konsole",
      action: "openAurKonsole",
    });
  }

  // Put recommended route first for scanning.
  routes.sort((a, b) => Number(b.recommended) - Number(a.recommended));
  return routes;
}
