import type { ElementContent, Root, RootContent } from "hast";
import { visit } from "unist-util-visit";

import {
  citationNumberFor,
  parseWikiLinks,
  wikiLinkHref,
} from "@/lib/wikiLinks";

/**
 * rehype plugin: turn `[[target]]` / `[[target|label]]` text into numbered
 * citation anchors (`[1]`, `[2]`, …) ordered by first appearance in the page.
 */
export function rehypeWikiCitations() {
  return () => (tree: Root) => {
    const seen = new Map<string, number>();

    visit(tree, "text", (node, index, parent) => {
      if (parent == null || typeof index !== "number") return;
      if (parent.type !== "element" && parent.type !== "root") return;
      if (parent.type === "element") {
        const tag = parent.tagName;
        if (tag === "code" || tag === "pre" || tag === "a") return;
      }

      const text = node.value;
      if (!text.includes("[[")) return;

      const matches = parseWikiLinks(text);
      if (matches.length === 0) return;

      const next: ElementContent[] = [];
      let cursor = 0;
      for (const match of matches) {
        if (match.index > cursor) {
          next.push({ type: "text", value: text.slice(cursor, match.index) });
        }
        const number = citationNumberFor(match.target, seen);
        next.push({
          type: "element",
          tagName: "a",
          properties: {
            className: ["wiki-citation"],
            href: wikiLinkHref(match.target),
            "data-wiki-target": match.target,
            "data-wiki-citation": String(number),
          },
          children: [{ type: "text", value: `[${number}]` }],
        });
        cursor = match.index + match.length;
      }
      if (cursor < text.length) {
        next.push({ type: "text", value: text.slice(cursor) });
      }

      const siblings = parent.children as RootContent[];
      siblings.splice(index, 1, ...(next as RootContent[]));
      return index + next.length;
    });
  };
}
