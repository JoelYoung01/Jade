import * as React from "react";
import type { Update } from "@tauri-apps/plugin-updater";

import {
  buildLinuxRoutes,
  checkForAppUpdate,
  downloadAndInstallUpdate,
  fetchAurPackageInfo,
  getAppVersion,
  getInstallContext,
  isNewerVersion,
  openAurUpdateInKonsole,
  openReleasesPage,
  supportsStartupUpdateCheck,
  usesTauriSelfUpdate,
  type InstallContext,
  type UpdatePrompt,
  type UpdateRoute,
} from "@/lib/updater";

type UseAppUpdaterResult = {
  prompt: UpdatePrompt | null;
  checking: boolean;
  installContext: InstallContext | null;
  checkManually: () => Promise<void>;
  runRoute: (route: UpdateRoute) => Promise<void>;
  installPending: () => Promise<void>;
  dismissPrompt: () => void;
};

export function useAppUpdater(): UseAppUpdaterResult {
  const [prompt, setPrompt] = React.useState<UpdatePrompt | null>(null);
  const [checking, setChecking] = React.useState(false);
  const [installContext, setInstallContext] = React.useState<InstallContext | null>(
    null,
  );
  const pendingRef = React.useRef<Update | null>(null);
  const contextRef = React.useRef<InstallContext | null>(null);

  const ensureContext = React.useCallback(async (): Promise<InstallContext> => {
    if (contextRef.current) return contextRef.current;
    const ctx = await getInstallContext();
    contextRef.current = ctx;
    setInstallContext(ctx);
    return ctx;
  }, []);

  const runCheck = React.useCallback(
    async (mode: "startup" | "manual") => {
      setChecking(true);
      if (mode === "manual") {
        setPrompt({ kind: "checking" });
      }

      try {
        const ctx = await ensureContext();
        if (mode === "startup" && !supportsStartupUpdateCheck(ctx)) {
          return;
        }

        if (usesTauriSelfUpdate(ctx)) {
          const update = await checkForAppUpdate();
          if (update) {
            pendingRef.current = update;
            let routes: UpdateRoute[] = [];
            if (ctx.kind === "appImage") {
              let aurRemoteVersion: string | null = null;
              let aurFetchFailed = false;
              if (ctx.archBased) {
                try {
                  const aur = await fetchAurPackageInfo();
                  aurRemoteVersion = aur?.upstreamVersion ?? null;
                } catch {
                  aurFetchFailed = true;
                }
              }
              routes = buildLinuxRoutes({
                ctx,
                remoteVersion: update.version,
                aurRemoteVersion,
                aurFetchFailed,
              });
            }
            setPrompt({
              kind: "available",
              version: update.version,
              notes: update.body ?? null,
              canSelfInstall: true,
              routes,
            });
          } else if (mode === "manual") {
            pendingRef.current = null;
            setPrompt({ kind: "upToDate" });
          } else {
            pendingRef.current = null;
          }
          return;
        }

        // Linux package installs (AUR / deb / unknown)
        const current = await getAppVersion();
        let aurRemoteVersion: string | null = null;
        let aurFetchFailed = false;
        let remoteVersion: string | null = null;
        let notes: string | null = null;

        if (ctx.kind === "aur" || ctx.archBased) {
          try {
            const aur = await fetchAurPackageInfo();
            if (aur) {
              aurRemoteVersion = aur.upstreamVersion;
              if (isNewerVersion(current, aur.upstreamVersion)) {
                remoteVersion = aur.upstreamVersion;
              }
            }
          } catch {
            aurFetchFailed = true;
          }
        }

        // Also probe GitHub updater JSON for AppImage channel version.
        let githubVersion: string | null = null;
        try {
          const update = await checkForAppUpdate();
          if (update) {
            githubVersion = update.version;
            notes = update.body ?? null;
            // Keep pending only when current install can self-install AppImage.
            if (ctx.kind === "appImage") {
              pendingRef.current = update;
            } else {
              pendingRef.current = null;
            }
          }
        } catch {
          // GitHub probe optional for AUR-primary installs.
        }

        if (!remoteVersion && githubVersion && isNewerVersion(current, githubVersion)) {
          remoteVersion = githubVersion;
        }

        // For AUR installs, only treat as update when AUR package itself is newer
        // (GitHub-only bumps shouldn't enable yay until AUR is published).
        if (ctx.kind === "aur") {
          if (aurRemoteVersion && isNewerVersion(current, aurRemoteVersion)) {
            remoteVersion = aurRemoteVersion;
          } else if (!aurFetchFailed) {
            remoteVersion = null;
          }
        }

        if (remoteVersion) {
          const routes = buildLinuxRoutes({
            ctx,
            remoteVersion,
            aurRemoteVersion,
            aurFetchFailed,
          });
          // For AUR-only updates, disable AppImage self-install action if we
          // don't have a pending Tauri update object — download instead.
          const normalized = routes.map((route) => {
            if (route.action === "installAppImage" && !pendingRef.current) {
              return {
                ...route,
                action: "downloadAppImage" as const,
                actionLabel: "Open downloads",
                description: `Download the AppImage build of Jade ${remoteVersion} from GitHub Releases.`,
              };
            }
            // If AUR package isn't newer, disable that route even if GitHub is.
            if (
              route.id === "aur" &&
              (!aurRemoteVersion || !isNewerVersion(current, aurRemoteVersion))
            ) {
              return {
                ...route,
                enabled: false,
                disabledReason: aurFetchFailed
                  ? route.disabledReason
                  : aurRemoteVersion
                    ? `AUR still publishes ${aurRemoteVersion}; push a PKGBUILD bump first.`
                    : route.disabledReason,
              };
            }
            return route;
          });

          // If every route is disabled and we're on AUR with no AUR bump, up to date.
          const anyEnabled = normalized.some((r) => r.enabled);
          if (!anyEnabled && ctx.kind === "aur" && !aurFetchFailed) {
            if (mode === "manual") setPrompt({ kind: "upToDate" });
            return;
          }

          setPrompt({
            kind: "available",
            version: remoteVersion,
            notes,
            canSelfInstall: Boolean(pendingRef.current),
            routes: normalized,
          });
        } else if (mode === "manual") {
          if (aurFetchFailed && ctx.kind === "aur") {
            setPrompt({
              kind: "error",
              message: "Could not reach the AUR to check for updates.",
            });
          } else {
            setPrompt({ kind: "upToDate" });
          }
        }
      } catch (err) {
        if (mode === "manual") {
          setPrompt({
            kind: "error",
            message: err instanceof Error ? err.message : String(err),
          });
        }
      } finally {
        setChecking(false);
      }
    },
    [ensureContext],
  );

  React.useEffect(() => {
    const timer = window.setTimeout(() => {
      void (async () => {
        try {
          const ctx = await ensureContext();
          if (supportsStartupUpdateCheck(ctx)) {
            await runCheck("startup");
          }
        } catch {
          // Silent on startup.
        }
      })();
    }, 0);
    return () => {
      window.clearTimeout(timer);
    };
  }, [ensureContext, runCheck]);

  const checkManually = React.useCallback(async () => {
    await runCheck("manual");
  }, [runCheck]);

  const installPending = React.useCallback(async () => {
    const update = pendingRef.current;
    if (!update) return;
    setPrompt({ kind: "installing", version: update.version });
    try {
      await downloadAndInstallUpdate(update);
    } catch (err) {
      setPrompt({
        kind: "error",
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }, []);

  const runRoute = React.useCallback(
    async (route: UpdateRoute) => {
      const ctx = await ensureContext();
      try {
        switch (route.action) {
          case "installAppImage":
            await installPending();
            break;
          case "downloadAppImage":
            await openReleasesPage(ctx.releasesUrl);
            setPrompt(null);
            break;
          case "openAurKonsole":
            await openAurUpdateInKonsole();
            setPrompt(null);
            break;
          default:
            break;
        }
      } catch (err) {
        setPrompt({
          kind: "error",
          message: err instanceof Error ? err.message : String(err),
        });
      }
    },
    [ensureContext, installPending],
  );

  const dismissPrompt = React.useCallback(() => {
    setPrompt(null);
  }, []);

  return {
    prompt,
    checking,
    installContext,
    checkManually,
    runRoute,
    installPending,
    dismissPrompt,
  };
}
