import * as React from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  apiGeneratePeerSyncToken,
  apiGetPeerSyncStatus,
  apiPairPeer,
  apiSetPeerSyncSettings,
  apiSyncNow,
} from "@/lib/api";
import type { PeerSyncSettings, PeerSyncStatus } from "@/lib/types";

type PeerSyncDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

function emptySettings(): PeerSyncSettings {
  return { enabled: false, bind: "0.0.0.0:7421", token: "" };
}

export function PeerSyncDialog({
  open,
  onOpenChange,
}: PeerSyncDialogProps): React.JSX.Element {
  const [status, setStatus] = React.useState<PeerSyncStatus | null>(null);
  const [settings, setSettings] = React.useState<PeerSyncSettings>(emptySettings);
  const [pairUrl, setPairUrl] = React.useState("");
  const [pairToken, setPairToken] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [message, setMessage] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  const refresh = React.useCallback(async () => {
    const next = await apiGetPeerSyncStatus();
    setStatus(next);
    setSettings(next.settings);
  }, []);

  React.useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setMessage(null);
    setError(null);
    void refresh().catch((e: unknown) => {
      if (!cancelled) setError(e instanceof Error ? e.message : String(e));
    });
    return () => {
      cancelled = true;
    };
  }, [open, refresh]);

  async function handleSave(): Promise<void> {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const next = await apiSetPeerSyncSettings(settings);
      setStatus(next);
      setSettings(next.settings);
      setMessage(
        next.settings.enabled
          ? next.listening
            ? "Peer sync enabled — listening while Jade is open."
            : "Peer sync enabled."
          : "Peer sync disabled.",
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleGenerateToken(): Promise<void> {
    const token = await apiGeneratePeerSyncToken();
    setSettings((prev) => ({ ...prev, token }));
  }

  async function handlePair(): Promise<void> {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const peer = await apiPairPeer(pairUrl.trim(), pairToken.trim());
      setPairUrl("");
      setPairToken("");
      await refresh();
      setMessage(`Paired with ${peer.peer_device_id}`);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleSyncNow(): Promise<void> {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const report = await apiSyncNow();
      await refresh();
      const ok = report.peers.filter((p) => !p.error).length;
      const failed = report.peers.filter((p) => p.error).length;
      setMessage(
        report.peers.length === 0
          ? "No peers configured."
          : `Synced ${ok} peer(s)${failed ? `, ${failed} failed` : ""}.`,
      );
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Peer sync</DialogTitle>
          <DialogDescription>
            LAN or Tailscale task sync between devices. Not a cloud server — peers dial each
            other with a shared token. Wiki stays local (use Syncthing for files).
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4 py-1">
          <div className="text-muted-foreground space-y-1 text-xs">
            <p>
              Device id:{" "}
              <span className="text-foreground font-mono">
                {status?.device.device_id ?? "…"}
              </span>
            </p>
            <p>
              Listener:{" "}
              <span className="text-foreground">
                {status?.listening ? "running" : "stopped"}
              </span>
            </p>
          </div>

          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={settings.enabled}
              onChange={(e) =>
                setSettings((prev) => ({ ...prev, enabled: e.target.checked }))
              }
            />
            Enable sync while Jade is open
          </label>

          <div className="grid gap-2">
            <Label htmlFor="peer-sync-bind">Bind address</Label>
            <Input
              id="peer-sync-bind"
              value={settings.bind}
              onChange={(e) =>
                setSettings((prev) => ({ ...prev, bind: e.target.value }))
              }
              placeholder="0.0.0.0:7421"
            />
          </div>

          <div className="grid gap-2">
            <Label htmlFor="peer-sync-token">Shared token</Label>
            <div className="flex gap-2">
              <Input
                id="peer-sync-token"
                value={settings.token}
                onChange={(e) =>
                  setSettings((prev) => ({ ...prev, token: e.target.value }))
                }
                className="font-mono text-xs"
              />
              <Button
                type="button"
                variant="outline"
                onClick={() => void handleGenerateToken()}
              >
                New
              </Button>
            </div>
          </div>

          <DialogFooter className="sm:justify-start">
            <Button type="button" disabled={busy} onClick={() => void handleSave()}>
              Save
            </Button>
            <Button
              type="button"
              variant="outline"
              disabled={busy}
              onClick={() => void handleSyncNow()}
            >
              Sync now
            </Button>
          </DialogFooter>

          <div className="grid gap-2 border-t border-border/60 pt-4">
            <p className="text-sm font-medium">Pair a peer</p>
            <Input
              value={pairUrl}
              onChange={(e) => setPairUrl(e.target.value)}
              placeholder="http://192.168.1.10:7421"
            />
            <Input
              value={pairToken}
              onChange={(e) => setPairToken(e.target.value)}
              placeholder="Peer token"
              className="font-mono text-xs"
            />
            <Button
              type="button"
              variant="secondary"
              disabled={busy || !pairUrl.trim() || !pairToken.trim()}
              onClick={() => void handlePair()}
            >
              Pair
            </Button>
          </div>

          <div className="grid gap-2 border-t border-border/60 pt-4">
            <p className="text-sm font-medium">Peers</p>
            {status?.peers.length ? (
              <ul className="space-y-2 text-xs">
                {status.peers.map((peer) => (
                  <li key={peer.peer_device_id} className="rounded-md border border-border/60 p-2">
                    <div className="font-mono">{peer.peer_device_id}</div>
                    <div className="text-muted-foreground">{peer.base_url}</div>
                    <div className="text-muted-foreground">
                      Last sync:{" "}
                      {peer.last_sync_at
                        ? new Date(peer.last_sync_at).toLocaleString()
                        : "never"}
                    </div>
                    {peer.last_error ? (
                      <div className="text-destructive mt-1">{peer.last_error}</div>
                    ) : null}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-muted-foreground text-xs">No peers paired yet.</p>
            )}
          </div>

          {message ? <p className="text-xs text-primary">{message}</p> : null}
          {error ? <p className="text-destructive text-xs">{error}</p> : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}
