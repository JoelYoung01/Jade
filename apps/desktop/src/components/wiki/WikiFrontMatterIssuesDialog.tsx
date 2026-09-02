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
import type { WikiIndexIssue } from "@/lib/types";

export type WikiFrontMatterFileGroup = {
  key: string;
  root_id: string;
  rel_path: string;
  absolute_path: string;
  issues: WikiIndexIssue[];
  repairable: boolean;
  repair_label: string;
};

export function groupWikiIndexIssues(issues: WikiIndexIssue[]): WikiFrontMatterFileGroup[] {
  const groups = new Map<string, WikiFrontMatterFileGroup>();
  for (const issue of issues) {
    const key = `${issue.root_id}:${issue.rel_path}`;
    const existing = groups.get(key);
    if (existing) {
      existing.issues.push(issue);
      existing.repairable = existing.repairable && issue.repairable;
      continue;
    }
    groups.set(key, {
      key,
      root_id: issue.root_id,
      rel_path: issue.rel_path,
      absolute_path: issue.absolute_path,
      issues: [issue],
      repairable: issue.repairable,
      repair_label: issue.repair_label ?? "Fix",
    });
  }
  return [...groups.values()].map((group) => ({
    ...group,
    repairable: group.issues.every((issue) => issue.repairable),
    repair_label:
      group.issues.find((issue) => issue.repair_label)?.repair_label ?? "Fix front matter",
  }));
}

type WikiFrontMatterIssuesDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  issues: WikiIndexIssue[];
  busy: boolean;
  repairingKey: string | null;
  onRepairFile: (group: WikiFrontMatterFileGroup) => void;
  onRepairAll: () => void;
};

export function WikiFrontMatterIssuesDialog({
  open,
  onOpenChange,
  issues,
  busy,
  repairingKey,
  onRepairFile,
  onRepairAll,
}: WikiFrontMatterIssuesDialogProps): React.JSX.Element {
  const groups = React.useMemo(() => groupWikiIndexIssues(issues), [issues]);
  const repairableCount = groups.filter((group) => group.repairable).length;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[min(92vw,36rem)] gap-3">
        <DialogHeader>
          <DialogTitle>Wiki front matter issues</DialogTitle>
          <DialogDescription>
            Jade indexed the rest of your wiki. These files have YAML headers that
            aren&apos;t in the shape Jade expects. Fixes rewrite only the header and
            keep the article body.
          </DialogDescription>
        </DialogHeader>

        {groups.length === 0 ? (
          <p className="text-sm text-muted-foreground">No remaining issues.</p>
        ) : (
          <div className="max-h-80 space-y-2 overflow-y-auto rounded-md border border-border/60 p-2">
            {groups.map((group) => {
              const repairing = repairingKey === group.key;
              return (
                <div
                  key={group.key}
                  className="rounded-md border border-border/50 bg-background/60 px-2.5 py-2"
                >
                  <div className="flex items-start gap-2">
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-sm font-medium" title={group.absolute_path}>
                        {group.rel_path}
                      </div>
                      <ul className="mt-1 space-y-1">
                        {group.issues.map((issue, index) => (
                          <li
                            key={`${group.key}-${index}`}
                            className="text-xs leading-snug text-muted-foreground"
                          >
                            {issue.message}
                            {issue.line != null ? (
                              <span className="text-muted-foreground/80">
                                {" "}
                                (line {issue.line}
                                {issue.column != null ? `, column ${issue.column}` : ""})
                              </span>
                            ) : null}
                          </li>
                        ))}
                      </ul>
                    </div>
                    {group.repairable ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        className="h-7 shrink-0"
                        disabled={busy}
                        onClick={() => onRepairFile(group)}
                      >
                        {repairing ? "Fixing…" : group.repair_label}
                      </Button>
                    ) : (
                      <p className="shrink-0 pt-0.5 text-[11px] text-muted-foreground">
                        Can&apos;t auto-fix
                      </p>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={busy}>
            Dismiss
          </Button>
          {repairableCount > 0 ? (
            <Button onClick={onRepairAll} disabled={busy}>
              {busy && repairingKey === "*"
                ? "Fixing…"
                : `Fix all (${repairableCount})`}
            </Button>
          ) : null}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
