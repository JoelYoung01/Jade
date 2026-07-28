import type { ElementContent, Root, RootContent } from "hast";
import { visit } from "unist-util-visit";

/**
 * rehype plugin: wrap case-insensitive occurrences of `query` in <mark>.
 * First hit also gets class `wiki-search-hit-active`.
 */
export function rehypeHighlightQuery(query: string) {
  const needle = query.trim();
  const lowerNeedle = needle.toLowerCase();

  return () => (tree: Root) => {
    if (!needle) return;

    let isFirst = true;

    visit(tree, "text", (node, index, parent) => {
      if (parent == null || typeof index !== "number") return;
      if (parent.type !== "element" && parent.type !== "root") return;
      // Don't highlight inside existing marks (shouldn't happen on fresh tree).
      if (parent.type === "element" && parent.tagName === "mark") return;

      const text = node.value;
      const lower = text.toLowerCase();
      if (!lower.includes(lowerNeedle)) return;

      const next: ElementContent[] = [];
      let start = 0;
      while (start < text.length) {
        const idx = lower.indexOf(lowerNeedle, start);
        if (idx === -1) {
          next.push({ type: "text", value: text.slice(start) });
          break;
        }
        if (idx > start) {
          next.push({ type: "text", value: text.slice(start, idx) });
        }
        const className = isFirst
          ? ["wiki-search-hit", "wiki-search-hit-active"]
          : ["wiki-search-hit"];
        isFirst = false;
        next.push({
          type: "element",
          tagName: "mark",
          properties: { className },
          children: [
            { type: "text", value: text.slice(idx, idx + needle.length) },
          ],
        });
        start = idx + needle.length;
      }

      const siblings = parent.children as RootContent[];
      siblings.splice(index, 1, ...(next as RootContent[]));
      // Continue after the nodes we just inserted.
      return index + next.length;
    });
  };
}
