import * as React from "react";
import { BookOpen, CheckSquare, Plus } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ShortcutKeys } from "@/components/ui/kbd";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { AppView } from "@/lib/types";
import { cn } from "@/lib/utils";

type AppShellProps = {
  view: AppView;
  onViewChange: (view: AppView) => void;
  onCreateTask: () => void;
  children: React.ReactNode;
};

export function AppShell({
  view,
  onViewChange,
  onCreateTask,
  children,
}: AppShellProps): React.JSX.Element {
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
        ) : (
          <div className="size-9" aria-hidden />
        )}
      </header>

      <main className="flex min-h-0 flex-1 flex-col overflow-hidden">{children}</main>
    </div>
  );
}
