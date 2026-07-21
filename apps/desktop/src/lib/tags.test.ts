import { describe, expect, it } from "vitest";

import { recentTagNames, tagSuggestionPool } from "@/lib/tags";
import type { Tag, Task } from "@/lib/types";

function tag(name: string, id = name): Tag {
  return {
    id,
    name,
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
  };
}

function task(partial: Partial<Task> & Pick<Task, "id" | "title" | "tags">): Task {
  return {
    description: null,
    status: "inactive",
    due_at: "2026-07-20T15:00:00.000Z",
    repeat_cron: null,
    created_at: "2026-07-01T00:00:00.000Z",
    updated_at: "2026-07-01T00:00:00.000Z",
    deleted_at: null,
    ...partial,
  };
}

describe("recentTagNames", () => {
  it("orders by most recently updated task first", () => {
    const names = recentTagNames([
      task({
        id: "1",
        title: "Old",
        updated_at: "2026-07-01T00:00:00.000Z",
        tags: [tag("alpha"), tag("beta")],
      }),
      task({
        id: "2",
        title: "New",
        updated_at: "2026-07-20T00:00:00.000Z",
        tags: [tag("gamma"), tag("alpha")],
      }),
    ]);
    expect(names).toEqual(["gamma", "alpha", "beta"]);
  });
});

describe("tagSuggestionPool", () => {
  it("puts recent names first then remaining tags", () => {
    expect(tagSuggestionPool(["gamma"], [tag("alpha"), tag("gamma"), tag("zeta")])).toEqual([
      "gamma",
      "alpha",
      "zeta",
    ]);
  });
});
