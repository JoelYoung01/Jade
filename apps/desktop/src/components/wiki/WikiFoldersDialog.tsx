import * as React from "react";
import { FolderPlus } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { SyncthingSettings, SyncthingStatus, WikiRoot } from "@/lib/types";
import { cn } from "@/lib/utils";

type WikiFoldersDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  roots: WikiRoot[];
  selectedRootId: string | null;
  busy: boolean;
  syncStatus: SyncthingStatus | null;
  syncthing: SyncthingSettings;
  showSyncSettings: boolean;
  onSelectRoot: (rootId: string | null) => void;
  onAddRoot: () => void;
  onRemoveRoot: (rootId: string) => void;
  onToggleSyncSettings: () => void;
  onSyncthingChange: (next: SyncthingSettings) => void;
  onSaveSyncthing: () => void;
};

export function WikiFoldersDialog({
  open,
  onOpenChange,
  roots,
  selectedRootId,
  busy,
  syncStatus,
  syncthing,
  showSyncSettings,
  onSelectRoot,
  onAddRoot,
  onRemoveRoot,
  onToggleSyncSettings,
  onSyncthingChange,
  onSaveSyncthing,
}: WikiFoldersDialogProps): React.JSX.Element {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[min(92vw,32rem)] gap-3">
        <DialogHeader>
          <DialogTitle>Wiki folders</DialogTitle>
          <DialogDescription>
            Choose which folders to include, or add another local directory.
          </DialogDescription>
        </DialogHeader>

        <Button
          className="w-full justify-start gap-2"
          variant="secondary"
          onClick={onAddRoot}
          disabled={busy}
        >
          <FolderPlus className="size-4" />
          Add folder…
        </Button>

        <div className="max-h-64 space-y-0.5 overflow-y-auto rounded-md border border-border/60 p-1">
          <button
            type="button"
            className={cn(
              "w-full rounded-md px-2.5 py-2 text-left text-sm",
              selectedRootId == null
                ? "bg-accent text-accent-foreground"
                : "hover:bg-accent/50",
            )}
            onClick={() => onSelectRoot(null)}
          >
            All folders
          </button>
          {roots.length === 0 ? (
            <p className="px-2.5 py-3 text-xs text-muted-foreground">
              No folders yet. Add a local directory to start indexing markdown.
            </p>
          ) : (
            roots.map((root) => (
              <div
                key={root.id}
                className={cn(
                  "group flex items-start gap-1 rounded-md",
                  selectedRootId === root.id && "bg-accent text-accent-foreground",
                )}
              >
                <button
                  type="button"
                  className={cn(
                    "min-w-0 flex-1 px-2.5 py-2 text-left",
                    selectedRootId !== root.id && "hover:bg-accent/50 rounded-md",
                  )}
                  onClick={() => onSelectRoot(root.id)}
                  title={root.path}
                >
                  <div className="truncate text-sm font-medium">{root.label}</div>
                  <div className="truncate text-[11px] text-muted-foreground">{root.path}</div>
                </button>
                <button
                  type="button"
                  className="mr-1 mt-2 shrink-0 rounded px-1.5 py-0.5 text-xs text-muted-foreground opacity-0 hover:text-destructive group-hover:opacity-100"
                  onClick={() => onRemoveRoot(root.id)}
                >
                  Remove
                </button>
              </div>
            ))
          )}
        </div>

        {syncStatus?.underSyncthing ? (
          <p className="text-[11px] text-primary">
            Syncthing
            {syncStatus.folder
              ? `: ${syncStatus.folder.label || syncStatus.folder.id}`
              : syncStatus.markerDetected
                ? " (marker detected)"
                : ""}
          </p>
        ) : null}

        <button
          type="button"
          className="text-left text-[11px] text-muted-foreground underline-offset-2 hover:underline"
          onClick={onToggleSyncSettings}
        >
          Syncthing settings
        </button>
        {showSyncSettings ? (
          <div className="space-y-1.5">
            <Input
              value={syncthing.address}
              onChange={(e) =>
                onSyncthingChange({ ...syncthing, address: e.target.value })
              }
              placeholder="http://127.0.0.1:8384"
              className="h-8 text-xs"
            />
            <Input
              value={syncthing.api_key}
              onChange={(e) =>
                onSyncthingChange({ ...syncthing, api_key: e.target.value })
              }
              placeholder="API key (optional)"
              className="h-8 text-xs"
            />
            <Button
              size="sm"
              variant="secondary"
              className="h-7"
              onClick={onSaveSyncthing}
              disabled={busy}
            >
              Save Syncthing settings
            </Button>
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
