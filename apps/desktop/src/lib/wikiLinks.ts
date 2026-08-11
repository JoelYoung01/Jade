import type { WikiPage } from "@/lib/types";

export type WikiLinkMatch = {
  /** Full `[[...]]` span. */
  raw: string;
  /** Link target (left of `|` when present). */
  target: string;
  /** Optional display label (right of `|`). */
  label: string | null;
  index: number;
  length: number;
};

const WIKI_LINK_RE = /\[\[([^\]]+?)\]\]/g;

/** Find `[[target]]` / `[[target|label]]` spans in plain text (document order). */
export function parseWikiLinks(text: string): WikiLinkMatch[] {
  const out: WikiLinkMatch[] = [];
  WIKI_LINK_RE.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = WIKI_LINK_RE.exec(text)) !== null) {
    const inner = match[1]?.trim() ?? "";
    if (!inner) continue;
    const pipe = inner.indexOf("|");
    const target =
      pipe === -1 ? inner.trim() : inner.slice(0, pipe).trim();
    const label =
      pipe === -1 ? null : inner.slice(pipe + 1).trim() || null;
    if (!target) continue;
    out.push({
      raw: match[0],
      target,
      label,
      index: match.index,
      length: match[0].length,
    });
  }
  return out;
}

/**
 * Assign citation numbers by first-seen target order.
 * Returns the number for `target`, allocating the next integer when new.
 */
export function citationNumberFor(
  target: string,
  seen: Map<string, number>,
): number {
  const existing = seen.get(target);
  if (existing != null) return existing;
  const n = seen.size + 1;
  seen.set(target, n);
  return n;
}

function pageStem(relPath: string): string {
  const normalized = relPath.replace(/\\/g, "/");
  const base = normalized.split("/").pop() ?? normalized;
  return base.replace(/\.md$/i, "");
}

/**
 * Resolve a wiki-link target against indexed pages (title, path, or file stem).
 * Prefers exact matches; falls back to case-insensitive.
 */
export function resolveWikiPage(
  pages: readonly WikiPage[],
  target: string,
): WikiPage | null {
  const needle = target.trim();
  if (!needle) return null;

  const exact = pages.find((page) => pageMatchesTarget(page, needle, true));
  if (exact) return exact;

  const lower = needle.toLowerCase();
  return (
    pages.find((page) => pageMatchesTarget(page, lower, false)) ?? null
  );
}

function pageMatchesTarget(
  page: WikiPage,
  needle: string,
  exact: boolean,
): boolean {
  const candidates = [
    page.title_cache,
    page.rel_path,
    pageStem(page.rel_path),
  ].filter((v): v is string => Boolean(v));

  if (exact) {
    return candidates.some((c) => c === needle);
  }
  return candidates.some((c) => c.toLowerCase() === needle);
}

/** Prefer resolved page title for tooltips; otherwise the raw target. */
export function wikiLinkDisplayTitle(
  target: string,
  page: WikiPage | null,
): string {
  return page?.title_cache?.trim() || target;
}

export const WIKI_LINK_HREF_PREFIX = "wiki://";

export function wikiLinkHref(target: string): string {
  return `${WIKI_LINK_HREF_PREFIX}${encodeURIComponent(target)}`;
}

export function parseWikiLinkHref(href: string): string | null {
  if (!href.startsWith(WIKI_LINK_HREF_PREFIX)) return null;
  try {
    return decodeURIComponent(href.slice(WIKI_LINK_HREF_PREFIX.length));
  } catch {
    return null;
  }
}

/**
 * react-markdown's default urlTransform strips unknown protocols (including
 * `wiki://`). Preserve wiki citation hrefs; sanitize everything else.
 */
export function wikiSafeUrlTransform(
  url: string,
  defaultTransform: (value: string) => string = (value) => value,
): string {
  if (url.startsWith(WIKI_LINK_HREF_PREFIX)) return url;
  return defaultTransform(url);
}
