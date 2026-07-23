import type { Tag, Task } from "@/lib/types";

export type TagParts =
  | { kind: "plain"; name: string }
  | { kind: "keyed"; key: string; value: string };

/** Split `Key:Value` tags on the first colon; plain names stay unchanged. */
export function parseTagParts(name: string): TagParts {
  const colon = name.indexOf(":");
  if (colon <= 0 || colon === name.length - 1) {
    return { kind: "plain", name };
  }
  const key = name.slice(0, colon);
  const value = name.slice(colon + 1);
  if (!key.trim() || !value.trim()) {
    return { kind: "plain", name };
  }
  return { kind: "keyed", key, value };
}

/** Stable 32-bit hash (djb2) for a string. */
export function hashString(input: string): number {
  let hash = 5381;
  for (let i = 0; i < input.length; i++) {
    hash = (hash * 33) ^ input.charCodeAt(i);
  }
  return hash >>> 0;
}

/**
 * Darkened Okabe–Ito categorical colors (skip black / bright yellow).
 * Ordered so adjacent palette slots alternate warm/cool families.
 * Distinct under common color-vision deficiencies; tuned as chip backgrounds.
 */
const TAG_KEY_COLORS = [
  "#8a5a00", // orange
  "#00664a", // bluish green
  "#7a4864", // reddish purple
  "#004a72", // blue
  "#8a3c00", // vermillion
  "#2a6a8a", // sky blue
  "#5a4a00", // olive/gold
] as const;

/**
 * Map a tag key to a muted hex background color (stable across sessions).
 * Picks from a fixed colorblind-safe palette rather than a continuous hue wheel,
 * so nearby keys (e.g. Feature vs Project) stay visually distinct.
 */
export function tagKeyBackground(key: string): string {
  const index = hashString(key.toLowerCase()) % TAG_KEY_COLORS.length;
  return TAG_KEY_COLORS[index]!;
}

/** Tag names ordered by most recent use on a task (by task updated_at, then created_at). */
export function recentTagNames(tasks: Task[], limit = 8): string[] {
  const sorted = [...tasks].sort((a, b) => {
    const aTime = Math.max(Date.parse(a.updated_at), Date.parse(a.created_at));
    const bTime = Math.max(Date.parse(b.updated_at), Date.parse(b.created_at));
    return bTime - aTime;
  });

  const seen = new Set<string>();
  const names: string[] = [];
  for (const task of sorted) {
    for (const tag of task.tags) {
      const key = tag.name.toLowerCase();
      if (seen.has(key)) continue;
      seen.add(key);
      names.push(tag.name);
      if (names.length >= limit) return names;
    }
  }
  return names;
}

/** Prefer recent names, then fall back to the full tag list (alpha) without duplicates. */
export function tagSuggestionPool(recentNames: string[], allTags: Tag[]): string[] {
  const seen = new Set(recentNames.map((n) => n.toLowerCase()));
  const rest = allTags
    .map((t) => t.name)
    .filter((name) => !seen.has(name.toLowerCase()))
    .sort((a, b) => a.localeCompare(b));
  return [...recentNames, ...rest];
}

/**
 * Best existing-tag completion for a draft, from a recent-first pool.
 * Prefers prefix matches; falls back to substring. Skips already-selected tags.
 */
export function bestTagAutocomplete(
  draft: string,
  pool: string[],
  selected: string[] = [],
): string | null {
  const needle = draft.trim().toLowerCase();
  if (!needle) return null;

  const selectedKeys = new Set(selected.map((n) => n.toLowerCase()));
  const candidates = pool.filter((name) => !selectedKeys.has(name.toLowerCase()));

  const prefix = candidates.find((name) => name.toLowerCase().startsWith(needle));
  if (prefix) return prefix;

  return candidates.find((name) => name.toLowerCase().includes(needle)) ?? null;
}
