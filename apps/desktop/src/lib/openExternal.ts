import { openUrl } from "@tauri-apps/plugin-opener";

import { isTauriRuntime } from "@/lib/runtime";

/** Open a URL in the system browser (or a new tab outside Tauri). */
export async function openExternalUrl(url: string): Promise<void> {
  if (!isTauriRuntime()) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }
  await openUrl(url);
}
