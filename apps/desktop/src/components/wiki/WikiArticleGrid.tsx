import type { WikiPage } from "@/lib/types";
import { WikiArticleCard } from "@/components/wiki/WikiArticleCard";

type WikiArticleGridProps = {
  pages: WikiPage[];
  heading: string;
  description: string;
  onSelectPage: (pageId: string) => void;
};

export function WikiArticleGrid({
  pages,
  heading,
  description,
  onSelectPage,
}: WikiArticleGridProps): React.JSX.Element {
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-4">
      <div className="mb-4">
        <h2 className="font-display text-base font-semibold tracking-wide">{heading}</h2>
        <p className="text-xs text-muted-foreground">{description}</p>
      </div>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
        {pages.map((page) => (
          <WikiArticleCard
            key={page.id}
            page={page}
            onClick={() => onSelectPage(page.id)}
          />
        ))}
      </div>
    </div>
  );
}
