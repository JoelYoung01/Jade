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
import type { UpdatePrompt } from "@/lib/updater";

type UpdateDialogProps = {
  prompt: UpdatePrompt | null;
  onInstall: () => void;
  onDismiss: () => void;
};

export function UpdateDialog({
  prompt,
  onInstall,
  onDismiss,
}: UpdateDialogProps): React.JSX.Element {
  const open = prompt !== null && prompt.kind !== "checking";

  const title = (() => {
    switch (prompt?.kind) {
      case "available":
        return "Update available";
      case "upToDate":
        return "You're up to date";
      case "error":
        return "Update check failed";
      case "linuxPackageManager":
        return "Updates via package manager";
      case "installing":
        return "Installing update";
      default:
        return "Updates";
    }
  })();

  const description = (() => {
    switch (prompt?.kind) {
      case "available": {
        const notes = prompt.notes?.trim();
        return notes
          ? `Jade ${prompt.version} is ready to install.\n\n${notes}`
          : `Jade ${prompt.version} is ready to install.`;
      }
      case "upToDate":
        return "You already have the latest version of Jade.";
      case "error":
        return prompt.message;
      case "linuxPackageManager":
        return "On Arch / EndeavourOS, update Jade with your AUR helper (for example yay -Syu jade-desktop-bin), not from inside the app.";
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
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description ? (
            <DialogDescription className="whitespace-pre-wrap">{description}</DialogDescription>
          ) : null}
        </DialogHeader>
        <DialogFooter>
          {prompt?.kind === "available" ? (
            <>
              <Button variant="ghost" onClick={onDismiss}>
                Later
              </Button>
              <Button onClick={onInstall}>Update now</Button>
            </>
          ) : prompt?.kind === "installing" ? null : (
            <Button onClick={onDismiss}>OK</Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
