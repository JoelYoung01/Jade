import type { Task } from "@/lib/types";

/**
 * OR delimiter for the board text filter.
 * Pipe is rare in titles/descriptions/tags (keyed tags use `:`), so it
 * won't collide with typical filter text.
 */
export const TEXT_FILTER_OR = "|";

/** Split a text filter into OR terms (trimmed; empty segments dropped). */
export function splitTextFilterTerms(query: string): string[] {
  return query
    .split(TEXT_FILTER_OR)
    .map((term) => term.trim())
    .filter((term) => term.length > 0);
}

/** Join OR terms with spaced pipes for readable filter input. */
export function joinTextFilterTerms(terms: string[]): string {
  return terms.join(` ${TEXT_FILTER_OR} `);
}

function taskHaystack(task: Task): string {
  return [task.title, task.description ?? "", ...task.tags.map((t) => t.name)]
    .join(" ")
    .toLowerCase();
}

/** True when the task matches any OR term as a case-insensitive substring. */
export function matchesTextFilter(task: Task, textQuery: string): boolean {
  const terms = splitTextFilterTerms(textQuery);
  if (terms.length === 0) return true;

  const haystack = taskHaystack(task);
  return terms.some((term) => haystack.includes(term.toLowerCase()));
}

/** Whether `tagName` is already an exact (case-insensitive) OR term. */
export function textFilterHasTerm(query: string, tagName: string): boolean {
  const needle = tagName.trim().toLowerCase();
  if (!needle) return false;
  return splitTextFilterTerms(query).some((term) => term.toLowerCase() === needle);
}

/**
 * Toggle a tag name as an exact OR term in the text filter.
 * Clicking an active term removes it; otherwise it is appended.
 */
export function toggleTagInTextFilter(query: string, tagName: string): string {
  const name = tagName.trim();
  if (!name) return query;

  const terms = splitTextFilterTerms(query);
  const needle = name.toLowerCase();
  const index = terms.findIndex((term) => term.toLowerCase() === needle);
  if (index >= 0) {
    terms.splice(index, 1);
  } else {
    terms.push(name);
  }
  return joinTextFilterTerms(terms);
}
