import * as React from "react";
import type { Update } from "@tauri-apps/plugin-updater";

import {
  checkForAppUpdate,
  downloadAndInstallUpdate,
  isLinuxDesktop,
  supportsInAppUpdater,
  type UpdatePrompt,
} from "@/lib/updater";

type UseAppUpdaterResult = {
  prompt: UpdatePrompt | null;
  checking: boolean;
  supportsUpdater: boolean;
  isLinux: boolean;
  checkManually: () => Promise<void>;
  installPending: () => Promise<void>;
  dismissPrompt: () => void;
};

export function useAppUpdater(): UseAppUpdaterResult {
  const supportsUpdater = supportsInAppUpdater();
  const isLinux = isLinuxDesktop();
  const [prompt, setPrompt] = React.useState<UpdatePrompt | null>(null);
  const [checking, setChecking] = React.useState(false);
  const pendingRef = React.useRef<Update | null>(null);

  const runCheck = React.useCallback(
    async (mode: "startup" | "manual") => {
      if (!supportsUpdater) {
        if (mode === "manual" && isLinux) {
          setPrompt({ kind: "linuxPackageManager" });
        }
        return;
      }

      setChecking(true);
      if (mode === "manual") {
        setPrompt({ kind: "checking" });
      }

      try {
        const update = await checkForAppUpdate();
        if (update) {
          pendingRef.current = update;
          setPrompt({
            kind: "available",
            version: update.version,
            notes: update.body ?? null,
          });
        } else if (mode === "manual") {
          pendingRef.current = null;
          setPrompt({ kind: "upToDate" });
        } else {
          pendingRef.current = null;
        }
      } catch (err) {
        // Startup checks stay silent on network/missing-release errors.
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
    [isLinux, supportsUpdater],
  );

  React.useEffect(() => {
    if (!supportsUpdater) return;
    // Defer so the check does not synchronously setState inside this effect.
    const timer = window.setTimeout(() => {
      void runCheck("startup");
    }, 0);
    return () => {
      window.clearTimeout(timer);
    };
  }, [runCheck, supportsUpdater]);

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

  const dismissPrompt = React.useCallback(() => {
    setPrompt(null);
  }, []);

  return {
    prompt,
    checking,
    supportsUpdater,
    isLinux,
    checkManually,
    installPending,
    dismissPrompt,
  };
}
