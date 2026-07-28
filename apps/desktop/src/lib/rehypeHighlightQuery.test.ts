import type { Root } from "hast";
import { describe, expect, it } from "vitest";

import { rehypeHighlightQuery } from "@/lib/rehypeHighlightQuery";

describe("rehypeHighlightQuery", () => {
  it("wraps matches in mark elements and marks the first active", () => {
    const tree: Root = {
      type: "root",
      children: [
        {
          type: "element",
          tagName: "p",
          properties: {},
          children: [{ type: "text", value: "Alpha foo beta FOO" }],
        },
      ],
    };

    rehypeHighlightQuery("foo")()(tree);

    const p = tree.children[0];
    expect(p?.type).toBe("element");
    if (p?.type !== "element") return;

    const marks = p.children.filter(
      (c) => c.type === "element" && c.tagName === "mark",
    );
    expect(marks).toHaveLength(2);
    expect(marks[0]).toMatchObject({
      type: "element",
      tagName: "mark",
      properties: { className: ["wiki-search-hit", "wiki-search-hit-active"] },
    });
    expect(marks[1]).toMatchObject({
      type: "element",
      tagName: "mark",
      properties: { className: ["wiki-search-hit"] },
    });
  });

  it("is a no-op for empty query", () => {
    const tree: Root = {
      type: "root",
      children: [{ type: "text", value: "hello" }],
    };
    rehypeHighlightQuery("  ")()(tree);
    expect(tree.children).toHaveLength(1);
    expect(tree.children[0]).toMatchObject({ type: "text", value: "hello" });
  });
});
