import type { WikiPage } from "@/lib/types";

export type WikiTopic = {
  name: string;
  count: number;
};

/** Unique tags across pages, sorted alphabetically. */
export function collectWikiTopics(pages: readonly WikiPage[]): WikiTopic[] {
  const counts = new Map<string, number>();
  for (const page of pages) {
    for (const tag of page.tags_cache) {
      const name = tag.trim();
      if (!name) continue;
      counts.set(name, (counts.get(name) ?? 0) + 1);
    }
  }
  return [...counts.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

function pageSortKey(page: WikiPage): string {
  return page.date_added_cache ?? page.created_at;
}

export function pageSortDate(page: WikiPage): string {
  return pageSortKey(page);
}

export function pageDisplayTitle(page: WikiPage): string {
  return page.title_cache ?? page.rel_path;
}

/** Recently added first (uses cached `date_added`, then index `created_at`). */
export function sortPagesByDateAdded(pages: readonly WikiPage[]): WikiPage[] {
  return [...pages].sort((a, b) => {
    const byDate = pageSortKey(b).localeCompare(pageSortKey(a));
    if (byDate !== 0) return byDate;
    return (a.title_cache ?? a.rel_path).localeCompare(b.title_cache ?? b.rel_path);
  });
}

export function filterPagesByTag(
  pages: readonly WikiPage[],
  tag: string | null,
): WikiPage[] {
  const sorted = sortPagesByDateAdded(pages);
  if (tag == null) return sorted;
  return sorted.filter((page) => page.tags_cache.includes(tag));
}

export function formatWikiDate(value: string | null | undefined): string | null {
  if (!value?.trim()) return null;
  const raw = value.trim();
  if (/^\d{4}-\d{2}-\d{2}$/.test(raw)) {
    const date = new Date(`${raw}T12:00:00`);
    if (!Number.isNaN(date.getTime())) {
      return date.toLocaleDateString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
      });
    }
  }
  const parsed = new Date(raw);
  if (!Number.isNaN(parsed.getTime())) {
    return parsed.toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }
  return raw;
}

/** YAML block from full markdown file content, if present. */
export function extractFrontMatterYaml(content: string): string | null {
  const normalized = content.replace(/^\uFEFF/, "");
  if (!normalized.startsWith("---")) return null;
  const afterOpen = normalized.slice(3).replace(/^\n/, "");
  const close = afterOpen.indexOf("\n---");
  if (close < 0) return null;
  return afterOpen.slice(0, close).trim();
}
