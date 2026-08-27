import * as React from "react";
import { X } from "lucide-react";

import { TagLabel } from "@/components/TagLabel";
import { Button } from "@/components/ui/button";
import type { WikiBacklink, WikiPageContent } from "@/lib/types";
import { extractFrontMatterYaml, formatWikiDate } from "@/lib/wikiTopics";
import { cn } from "@/lib/utils";

type WikiMetadataPanelProps = {
  open: boolean;
  onClose: () => void;
  content: WikiPageContent;
  backlinks: WikiBacklink[];
  onOpenPage: (pageId: string) => void;
};

export function WikiMetadataPanel({
  open,
  onClose,
  content,
  backlinks,
  onOpenPage,
}: WikiMetadataPanelProps): React.JSX.Element {
  const fm = content.front_matter;
  const yaml = extractFrontMatterYaml(content.content);
  const sourceUrl = fm?.url?.trim() || fm?.source?.trim() || null;
  const dateAdded = formatWikiDate(fm?.date_added ?? fm?.date ?? content.page.date_added_cache);

  React.useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  return (
    <>
      <button
        type="button"
        aria-label="Close metadata panel"
        className={cn(
          "fixed inset-0 z-40 bg-background/40 transition-opacity",
          open ? "opacity-100" : "pointer-events-none opacity-0",
        )}
        onClick={onClose}
      />
      <aside
        className={cn(
          "fixed inset-y-0 right-0 z-50 flex w-[min(92vw,22rem)] flex-col border-l border-border/60 bg-background shadow-xl transition-transform duration-200",
          open ? "translate-x-0" : "translate-x-full",
        )}
        aria-hidden={!open}
      >
        <div className="flex items-center justify-between border-b border-border/60 px-3 py-2">
          <h3 className="text-sm font-semibold">Details</h3>
          <Button variant="ghost" size="icon" onClick={onClose} aria-label="Close">
            <X className="size-4" />
          </Button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-3 text-sm">
          <dl className="space-y-3">
            {dateAdded ? (
              <div>
                <dt className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                  Added
                </dt>
                <dd className="mt-0.5">{dateAdded}</dd>
              </div>
            ) : null}
            {fm?.author?.trim() ? (
              <div>
                <dt className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                  Author
                </dt>
                <dd className="mt-0.5">{fm.author.trim()}</dd>
              </div>
            ) : null}
            {sourceUrl ? (
              <div>
                <dt className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                  Source
                </dt>
                <dd className="mt-0.5 break-all text-xs text-primary">{sourceUrl}</dd>
              </div>
            ) : null}
            {fm?.tags && fm.tags.length > 0 ? (
              <div>
                <dt className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                  Tags
                </dt>
                <dd className="mt-1 flex flex-wrap gap-1">
                  {fm.tags.map((tag) => (
                    <span
                      key={tag}
                      className="rounded-md bg-secondary px-1.5 py-0.5 text-xs text-secondary-foreground"
                    >
                      <TagLabel name={tag} />
                    </span>
                  ))}
                </dd>
              </div>
            ) : null}
            {fm?.references && fm.references.length > 0 ? (
              <div>
                <dt className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                  References
                </dt>
                <dd className="mt-1 space-y-1">
                  {fm.references.map((ref) => (
                    <p key={ref} className="break-all text-xs text-muted-foreground">
                      {ref}
                    </p>
                  ))}
                </dd>
              </div>
            ) : null}
            <div>
              <dt className="text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                Path
              </dt>
              <dd className="mt-0.5 break-all text-xs text-muted-foreground">
                {content.absolute_path}
              </dd>
            </div>
          </dl>

          {yaml ? (
            <div className="mt-4">
              <p className="mb-1 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                Front matter
              </p>
              <pre className="overflow-x-auto rounded-md border border-border/60 bg-muted/40 p-2 text-[11px] leading-relaxed whitespace-pre-wrap">
                {yaml}
              </pre>
            </div>
          ) : null}

          {backlinks.length > 0 ? (
            <div className="mt-4">
              <p className="mb-1 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
                Backlinks
              </p>
              <div className="flex flex-wrap gap-1.5">
                {backlinks.map((link) => (
                  <button
                    key={`${link.page.id}-${link.target_raw}`}
                    type="button"
                    className="rounded-md bg-secondary px-2 py-0.5 text-xs text-secondary-foreground hover:bg-accent"
                    onClick={() => onOpenPage(link.page.id)}
                  >
                    {link.page.title_cache || link.page.rel_path}
                  </button>
                ))}
              </div>
            </div>
          ) : null}
        </div>
      </aside>
    </>
  );
}
