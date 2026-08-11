import type { Root } from "hast";
import { describe, expect, it } from "vitest";

import { rehypeWikiCitations } from "@/lib/rehypeWikiCitations";

describe("rehypeWikiCitations", () => {
  it("replaces wiki links with numbered citations by first appearance", () => {
    const tree: Root = {
      type: "root",
      children: [
        {
          type: "element",
          tagName: "p",
          properties: {},
          children: [
            {
              type: "text",
              value:
                "Claim [[Alpha]]. Again [[Alpha]]. Also [[Beta|B]].",
            },
          ],
        },
        {
          type: "element",
          tagName: "p",
          properties: {},
          children: [{ type: "text", value: "Sources: [[Beta]], [[Gamma]]." }],
        },
      ],
    };

    rehypeWikiCitations()()(tree);

    const p1 = tree.children[0];
    expect(p1?.type).toBe("element");
    if (p1?.type !== "element") return;

    const citations = p1.children.filter(
      (c) => c.type === "element" && c.tagName === "a",
    );
    expect(citations).toHaveLength(3);
    expect(citations[0]).toMatchObject({
      properties: {
        className: ["wiki-citation"],
        "data-wiki-citation": "1",
        "data-wiki-target": "Alpha",
      },
      children: [{ type: "text", value: "[1]" }],
    });
    expect(citations[1]).toMatchObject({
      properties: { "data-wiki-citation": "1", "data-wiki-target": "Alpha" },
      children: [{ type: "text", value: "[1]" }],
    });
    expect(citations[2]).toMatchObject({
      properties: { "data-wiki-citation": "2", "data-wiki-target": "Beta" },
      children: [{ type: "text", value: "[2]" }],
    });

    const p2 = tree.children[1];
    expect(p2?.type).toBe("element");
    if (p2?.type !== "element") return;
    const later = p2.children.filter(
      (c) => c.type === "element" && c.tagName === "a",
    );
    expect(later[0]).toMatchObject({
      properties: { "data-wiki-citation": "2" },
    });
    expect(later[1]).toMatchObject({
      properties: { "data-wiki-citation": "3", "data-wiki-target": "Gamma" },
      children: [{ type: "text", value: "[3]" }],
    });
  });

  it("leaves wiki syntax inside code alone", () => {
    const tree: Root = {
      type: "root",
      children: [
        {
          type: "element",
          tagName: "code",
          properties: {},
          children: [{ type: "text", value: "[[Alpha]]" }],
        },
      ],
    };

    rehypeWikiCitations()()(tree);

    const code = tree.children[0];
    expect(code).toMatchObject({
      type: "element",
      tagName: "code",
      children: [{ type: "text", value: "[[Alpha]]" }],
    });
  });
});
