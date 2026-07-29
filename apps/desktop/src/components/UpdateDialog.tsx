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
import type { UpdatePrompt, UpdateRoute } from "@/lib/updater";
import { cn } from "@/lib/utils";

type UpdateDialogProps = {
  prompt: UpdatePrompt | null;
  onInstall: () => void;
  onRunRoute: (route: UpdateRoute) => void;
  onDismiss: () => void;
};

export function UpdateDialog({
  prompt,
  onInstall,
  onRunRoute,
  onDismiss,
}: UpdateDialogProps): React.JSX.Element {
  const open = prompt !== null && prompt.kind !== "checking";
  const linuxChooser =
    prompt?.kind === "available" && prompt.routes.length > 0;

  const title = (() => {
    switch (prompt?.kind) {
      case "available":
        return "Update available";
      case "upToDate":
        return "You're up to date";
      case "error":
        return "Update check failed";
      case "installing":
        return "Installing update";
      default:
        return "Updates";
    }
  })();

  const description = (() => {
    switch (prompt?.kind) {
      case "available": {
        if (linuxChooser) {
          return `Jade ${prompt.version} is available. Choose how to update based on how you installed the app.`;
        }
        const notes = prompt.notes?.trim();
        return notes
          ? `Jade ${prompt.version} is ready to install.\n\n${notes}`
          : `Jade ${prompt.version} is ready to install.`;
      }
      case "upToDate":
        return "You already have the latest version of Jade.";
      case "error":
        return prompt.message;
      case "installing":
        return `Downloading and installing Jade ${prompt.version}. The app will restart when finished.`;
      default:
        return null;
    }
  })();

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next && prompt?.kind !== "installing") onDismiss();
      }}
    >
      <DialogContent className={linuxChooser ? "sm:max-w-lg" : undefined}>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description ? (
            <DialogDescription className="whitespace-pre-wrap">{description}</DialogDescription>
          ) : null}
        </DialogHeader>

        {linuxChooser && prompt.kind === "available" ? (
          <div className="grid gap-2">
            {prompt.routes.map((route) => (
              <UpdateRouteCard
                key={route.id}
                route={route}
                onSelect={() => onRunRoute(route)}
              />
            ))}
          </div>
        ) : null}

        <DialogFooter>
          {prompt?.kind === "available" && !linuxChooser ? (
            <>
              <Button variant="ghost" onClick={onDismiss}>
                Later
              </Button>
              {prompt.canSelfInstall ? (
                <Button onClick={onInstall}>Update now</Button>
              ) : (
                <Button onClick={onDismiss}>OK</Button>
              )}
            </>
          ) : prompt?.kind === "available" && linuxChooser ? (
            <Button variant="ghost" onClick={onDismiss}>
              Later
            </Button>
          ) : prompt?.kind === "installing" ? null : (
            <Button onClick={onDismiss}>OK</Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function UpdateRouteCard({
  route,
  onSelect,
}: {
  route: UpdateRoute;
  onSelect: () => void;
}): React.JSX.Element {
  return (
    <div
      className={cn(
        "rounded-md border border-border/70 bg-background/40 p-3",
        route.recommended && "border-primary/50",
        !route.enabled && "opacity-60",
      )}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 space-y-1">
          <div className="flex flex-wrap items-center gap-2">
            <p className="text-sm font-medium text-foreground">{route.title}</p>
            {route.recommendedLabel ? (
              <span className="rounded bg-primary/15 px-1.5 py-0.5 text-[10px] font-medium tracking-wide text-primary uppercase">
                {route.recommendedLabel}
              </span>
            ) : null}
          </div>
          <p className="text-xs text-muted-foreground">{route.description}</p>
          {!route.enabled && route.disabledReason ? (
            <p className="text-xs text-destructive/90">{route.disabledReason}</p>
          ) : null}
        </div>
        <Button
          size="sm"
          variant={route.recommended ? "default" : "secondary"}
          disabled={!route.enabled}
          onClick={onSelect}
        >
          {route.actionLabel}
        </Button>
      </div>
    </div>
  );
}
