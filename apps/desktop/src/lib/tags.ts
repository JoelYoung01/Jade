import type { Tag, Task } from "@/lib/types";

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
