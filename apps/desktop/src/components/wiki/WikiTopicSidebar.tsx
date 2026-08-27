import { cn } from "@/lib/utils";
import type { WikiTopic } from "@/lib/wikiTopics";

type WikiTopicSidebarProps = {
  topics: WikiTopic[];
  selectedTag: string | null;
  totalArticles: number;
  onSelectTag: (tag: string | null) => void;
};

export function WikiTopicSidebar({
  topics,
  selectedTag,
  totalArticles,
  onSelectTag,
}: WikiTopicSidebarProps): React.JSX.Element {
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-2">
      <p className="mb-1 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">
        Topics
      </p>
      <ul className="space-y-0.5">
        <li>
          <button
            type="button"
            className={cn(
              "flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left",
              selectedTag == null
                ? "bg-accent text-accent-foreground"
                : "hover:bg-accent/50",
            )}
            onClick={() => onSelectTag(null)}
          >
            <span className="truncate text-sm font-medium">All articles</span>
            <span className="ml-2 shrink-0 text-[11px] text-muted-foreground">
              {totalArticles}
            </span>
          </button>
        </li>
        {topics.length === 0 ? (
          <li className="px-2 py-2 text-xs text-muted-foreground">
            No tags indexed yet.
          </li>
        ) : (
          topics.map((topic) => (
            <li key={topic.name}>
              <button
                type="button"
                className={cn(
                  "flex w-full items-center justify-between rounded-md px-2 py-1.5 text-left",
                  selectedTag === topic.name
                    ? "bg-accent text-accent-foreground"
                    : "hover:bg-accent/50",
                )}
                onClick={() => onSelectTag(topic.name)}
              >
                <span className="truncate text-sm">{topic.name}</span>
                <span className="ml-2 shrink-0 text-[11px] text-muted-foreground">
                  {topic.count}
                </span>
              </button>
            </li>
          ))
        )}
      </ul>
    </div>
  );
}
