import * as React from "react";
import {
  ArrowLeft,
  Check,
  Copy,
  ExternalLink,
  Info,
  Pencil,
  Save,
} from "lucide-react";

import { MarkdownView } from "@/components/MarkdownView";
import { WikiMetadataPanel } from "@/components/wiki/WikiMetadataPanel";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { openExternalUrl } from "@/lib/openExternal";
import type { WikiBacklink, WikiPage, WikiPageContent } from "@/lib/types";
import { cn } from "@/lib/utils";

type WikiReaderViewProps = {
  content: WikiPageContent;
  pages: WikiPage[];
  backlinks: WikiBacklink[];
  highlightQuery: string;
  editing: boolean;
  draft: string;
  busy: boolean;
  copied: boolean;
  metadataOpen: boolean;
  onBack: () => void;
  onToggleMetadata: () => void;
  onWikiLink: (pageId: string) => void;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onSave: () => void;
  onCopyBody: () => void;
  onDraftChange: (value: string) => void;
  onCloseMetadata: () => void;
};

export function WikiReaderView({
  content,
  pages,
  backlinks,
  highlightQuery,
  editing,
  draft,
  busy,
  copied,
  metadataOpen,
  onBack,
  onToggleMetadata,
  onWikiLink,
  onStartEdit,
  onCancelEdit,
  onSave,
  onCopyBody,
  onDraftChange,
  onCloseMetadata,
}: WikiReaderViewProps): React.JSX.Element {
  const title = content.page.title_cache || content.page.rel_path;
  const sourceUrl =
    content.front_matter?.url?.trim() || content.front_matter?.source?.trim() || null;

  React.useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || editing) return;
      if (metadataOpen) {
        onCloseMetadata();
        return;
      }
      onBack();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [editing, metadataOpen, onBack, onCloseMetadata]);

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <div className="flex items-center gap-2 border-b border-border/60 px-3 py-2">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon" onClick={onBack} aria-label="Back to explorer">
              <ArrowLeft className="size-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">Back to explorer</TooltipContent>
        </Tooltip>
        <h2 className="min-w-0 flex-1 truncate font-display text-sm font-semibold tracking-wide">
          {title}
        </h2>
        <div className="flex shrink-0 items-center gap-1">
          {editing ? (
            <>
              <Button size="sm" variant="ghost" onClick={onCancelEdit} disabled={busy}>
                Cancel
              </Button>
              <Button size="sm" onClick={onSave} disabled={busy}>
                <Save className="size-3.5" />
                Save
              </Button>
            </>
          ) : (
            <>
              {sourceUrl ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => void openExternalUrl(sourceUrl)}
                      aria-label="Open source"
                    >
                      <ExternalLink className="size-3.5" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="bottom" className="max-w-sm break-all">
                    {sourceUrl}
                  </TooltipContent>
                </Tooltip>
              ) : null}
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={onCopyBody}
                    aria-label={copied ? "Copied" : "Copy markdown"}
                  >
                    {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom">Copy markdown body</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    size="sm"
                    variant={metadataOpen ? "default" : "secondary"}
                    onClick={onToggleMetadata}
                    aria-label="Toggle details"
                  >
                    <Info className="size-3.5" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom">Details & metadata</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button size="sm" variant="secondary" onClick={onStartEdit} aria-label="Edit">
                    <Pencil className="size-3.5" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom">Edit</TooltipContent>
              </Tooltip>
            </>
          )}
        </div>
      </div>

      {editing ? (
        <Textarea
          value={draft}
          onChange={(e) => onDraftChange(e.target.value)}
          className="min-h-0 flex-1 resize-none rounded-none border-0 bg-transparent px-4 py-4 font-mono text-sm leading-relaxed focus-visible:ring-0"
          spellCheck={false}
        />
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto">
          <div className="mx-auto max-w-3xl px-4 py-6">
            <MarkdownView
              markdown={content.body}
              pages={pages}
              highlightQuery={highlightQuery}
              onWikiLink={onWikiLink}
              className={cn("wiki-prose")}
            />
          </div>
        </div>
      )}

      <WikiMetadataPanel
        open={metadataOpen}
        onClose={onCloseMetadata}
        content={content}
        backlinks={backlinks}
        onOpenPage={onWikiLink}
      />
    </div>
  );
}
