import * as React from "react";
import { getVersion } from "@tauri-apps/api/app";
import { BookOpen, CheckSquare, MoreVertical, Plus, RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ShortcutKeys } from "@/components/ui/kbd";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { isTauriRuntime } from "@/lib/runtime";
import type { AppView } from "@/lib/types";
import { cn } from "@/lib/utils";

type AppShellProps = {
  view: AppView;
  onViewChange: (view: AppView) => void;
  onCreateTask: () => void;
  onCheckForUpdates: () => void;
  updateChecking?: boolean;
  children: React.ReactNode;
};

export function AppShell({
  view,
  onViewChange,
  onCreateTask,
  onCheckForUpdates,
  updateChecking = false,
  children,
}: AppShellProps): React.JSX.Element {
  const [version, setVersion] = React.useState<string | null>(() =>
    isTauriRuntime() ? null : "0.1.0-dev",
  );

  React.useEffect(() => {
    if (!isTauriRuntime()) return;
    let cancelled = false;
    void getVersion()
      .then((next) => {
        if (!cancelled) setVersion(next);
      })
      .catch(() => {
        if (!cancelled) setVersion(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex items-center justify-between border-b border-border/60 px-3 py-2">
        <div className="flex items-center gap-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className={cn(
                  view === "tasks" && "bg-accent text-accent-foreground",
                )}
                aria-label="Tasks"
                aria-current={view === "tasks" ? "page" : undefined}
                onClick={() => onViewChange("tasks")}
              >
                <CheckSquare />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">Tasks</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className={cn(
                  view === "wiki" && "bg-accent text-accent-foreground",
                )}
                aria-label="Wiki"
                aria-current={view === "wiki" ? "page" : undefined}
                onClick={() => onViewChange("wiki")}
              >
                <BookOpen />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">Wiki</TooltipContent>
          </Tooltip>
        </div>

        <div className="pointer-events-none absolute left-1/2 flex -translate-x-1/2 items-center gap-2">
          <img
            src="/jade-logo.svg"
            alt=""
            width={20}
            height={20}
            className="size-5 rounded-[5px]"
            draggable={false}
          />
          <span className="font-display text-sm font-semibold tracking-[0.2em] text-primary uppercase">
            Jade
          </span>
        </div>

        <div className="flex items-center gap-1">
          {view === "tasks" ? (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="ghost" size="icon" onClick={onCreateTask} aria-label="New task">
                  <Plus />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="flex items-center gap-1.5">
                <span>New task</span>
                <ShortcutKeys keys={["Ctrl", "N"]} className="text-background" />
              </TooltipContent>
            </Tooltip>
          ) : null}

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" aria-label="App menu">
                <MoreVertical />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {version ? <DropdownMenuLabel>Version {version}</DropdownMenuLabel> : null}
              <DropdownMenuSeparator />
              <DropdownMenuItem
                disabled={updateChecking}
                onSelect={() => {
                  void onCheckForUpdates();
                }}
              >
                <RefreshCw className={cn("mr-2 size-4", updateChecking && "animate-spin")} />
                Check for updates
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </header>

      <main className="flex min-h-0 flex-1 flex-col overflow-hidden">{children}</main>
    </div>
  );
}
