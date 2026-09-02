import { describe, expect, it } from "vitest";

import { groupWikiIndexIssues } from "@/lib/wikiFrontMatterIssues";
import type { WikiIndexIssue } from "@/lib/types";

function issue(
  relPath: string,
  message: string,
  repairable = true,
): WikiIndexIssue {
  return {
    root_id: "root-1",
    rel_path: relPath,
    absolute_path: `/wiki/${relPath}`,
    kind: "string_as_list",
    field: "references",
    message,
    line: 6,
    column: 13,
    repairable,
    repair_label: repairable ? "Wrap as a list" : null,
  };
}

describe("groupWikiIndexIssues", () => {
  it("groups issues from the same file", () => {
    const groups = groupWikiIndexIssues([
      issue("notes/wigolo.md", "references is a string"),
      issue("notes/wigolo.md", "tags is a string"),
      issue("other.md", "invalid yaml", false),
    ]);
    expect(groups).toHaveLength(2);
    const wigolo = groups.find((group) => group.rel_path === "notes/wigolo.md");
    expect(wigolo?.issues).toHaveLength(2);
    expect(wigolo?.repairable).toBe(true);
    expect(wigolo?.repair_label).toBe("Wrap as a list");
    const other = groups.find((group) => group.rel_path === "other.md");
    expect(other?.repairable).toBe(false);
  });
});
