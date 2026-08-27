import { describe, expect, it } from "vitest";

import type { WikiPage } from "@/lib/types";
import {
  collectWikiTopics,
  filterPagesByTag,
  sortPagesByDateAdded,
} from "@/lib/wikiTopics";

function page(id: string, tags: string[], dateAdded: string | null): WikiPage {
  return {
    id,
    root_id: "root",
    rel_path: `${id}.md`,
    content_hash: "x",
    mtime: "2026-01-01T00:00:00Z",
    indexed_at: "2026-01-01T00:00:00Z",
    missing_at: null,
    title_cache: id,
    tags_cache: tags,
    date_added_cache: dateAdded,
    summary_cache: null,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    deleted_at: null,
  };
}

describe("wikiTopics", () => {
  it("collects unique tags with counts", () => {
    const topics = collectWikiTopics([
      page("a", ["agents", "rust"], null),
      page("b", ["agents"], null),
    ]);
    expect(topics).toEqual([
      { name: "agents", count: 2 },
      { name: "rust", count: 1 },
    ]);
  });

  it("sorts by date_added descending", () => {
    const sorted = sortPagesByDateAdded([
      page("old", [], "2026-01-01"),
      page("new", [], "2026-08-27"),
    ]);
    expect(sorted.map((p) => p.id)).toEqual(["new", "old"]);
  });

  it("filters pages by tag", () => {
    const filtered = filterPagesByTag(
      [page("a", ["agents"], null), page("b", ["rust"], null)],
      "agents",
    );
    expect(filtered.map((p) => p.id)).toEqual(["a"]);
  });
});
