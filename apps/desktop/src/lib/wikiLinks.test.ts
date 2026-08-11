import { describe, expect, it } from "vitest";

import {
  citationNumberFor,
  parseWikiLinkHref,
  parseWikiLinks,
  resolveWikiPage,
  wikiLinkDisplayTitle,
  wikiLinkHref,
  wikiSafeUrlTransform,
} from "@/lib/wikiLinks";
import type { WikiPage } from "@/lib/types";

function page(partial: Partial<WikiPage> & Pick<WikiPage, "id" | "rel_path">): WikiPage {
  return {
    root_id: "root",
    content_hash: "",
    mtime: "",
    indexed_at: "",
    missing_at: null,
    title_cache: null,
    tags_cache: [],
    created_at: "",
    updated_at: "",
    deleted_at: null,
    ...partial,
  };
}

describe("parseWikiLinks", () => {
  it("parses targets and optional labels in order", () => {
    const text =
      "See [[Atomic Design]] and [[Design is compromise - Steph Ango summary|Ango]].";
    expect(parseWikiLinks(text)).toEqual([
      {
        raw: "[[Atomic Design]]",
        target: "Atomic Design",
        label: null,
        index: 4,
        length: 17,
      },
      {
        raw: "[[Design is compromise - Steph Ango summary|Ango]]",
        target: "Design is compromise - Steph Ango summary",
        label: "Ango",
        index: 26,
        length: 50,
      },
    ]);
  });

  it("skips empty targets", () => {
    expect(parseWikiLinks("[[ ]] and [[|label]]")).toEqual([]);
  });
});

describe("citationNumberFor", () => {
  it("numbers by first-seen target and reuses repeats", () => {
    const seen = new Map<string, number>();
    expect(citationNumberFor("A", seen)).toBe(1);
    expect(citationNumberFor("B", seen)).toBe(2);
    expect(citationNumberFor("A", seen)).toBe(1);
    expect(citationNumberFor("C", seen)).toBe(3);
  });
});

describe("resolveWikiPage", () => {
  const pages = [
    page({ id: "1", rel_path: "notes/atomic.md", title_cache: "Atomic Design" }),
    page({ id: "2", rel_path: "ui/compromise.md", title_cache: null }),
  ];

  it("matches title, path, and stem", () => {
    expect(resolveWikiPage(pages, "Atomic Design")?.id).toBe("1");
    expect(resolveWikiPage(pages, "notes/atomic.md")?.id).toBe("1");
    expect(resolveWikiPage(pages, "atomic")?.id).toBe("1");
    expect(resolveWikiPage(pages, "compromise")?.id).toBe("2");
  });

  it("falls back to case-insensitive match", () => {
    expect(resolveWikiPage(pages, "atomic design")?.id).toBe("1");
  });
});

describe("wiki link href helpers", () => {
  it("round-trips targets with spaces", () => {
    const href = wikiLinkHref("Safe Visual Design Rules");
    expect(href.startsWith("wiki://")).toBe(true);
    expect(parseWikiLinkHref(href)).toBe("Safe Visual Design Rules");
  });

  it("preserves wiki hrefs in urlTransform", () => {
    const href = wikiLinkHref("Atomic Design");
    expect(wikiSafeUrlTransform(href, () => "")).toBe(href);
    expect(wikiSafeUrlTransform("javascript:alert(1)", () => "")).toBe("");
    expect(
      wikiSafeUrlTransform("https://example.com", (u) => u),
    ).toBe("https://example.com");
  });
});

describe("wikiLinkDisplayTitle", () => {
  it("prefers page title_cache", () => {
    expect(
      wikiLinkDisplayTitle(
        "atomic",
        page({ id: "1", rel_path: "a.md", title_cache: "Atomic Design" }),
      ),
    ).toBe("Atomic Design");
    expect(wikiLinkDisplayTitle("atomic", null)).toBe("atomic");
  });
});
