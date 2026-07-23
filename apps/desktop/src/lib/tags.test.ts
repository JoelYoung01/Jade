import { describe, expect, it } from "vitest";

import {
  bestTagAutocomplete,
  hashString,
  parseTagParts,
  recentTagNames,
  tagKeyBackground,
  tagSuggestionPool,
} from "@/lib/tags";
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

describe("parseTagParts", () => {
  it("splits keyed tags on the first colon", () => {
    expect(parseTagParts("Project:Portal")).toEqual({
      kind: "keyed",
      key: "Project",
      value: "Portal",
    });
    expect(parseTagParts("Env:prod:west")).toEqual({
      kind: "keyed",
      key: "Env",
      value: "prod:west",
    });
  });

  it("treats plain names and malformed keyed tags as plain", () => {
    expect(parseTagParts("urgent")).toEqual({ kind: "plain", name: "urgent" });
    expect(parseTagParts(":Portal")).toEqual({ kind: "plain", name: ":Portal" });
    expect(parseTagParts("Project:")).toEqual({ kind: "plain", name: "Project:" });
    expect(parseTagParts(" : ")).toEqual({ kind: "plain", name: " : " });
  });
});

describe("tagKeyBackground", () => {
  it("returns a stable hex color for a key", () => {
    const a = tagKeyBackground("Project");
    const b = tagKeyBackground("Project");
    expect(a).toMatch(/^#[0-9a-f]{6}$/);
    expect(a).toBe(b);
  });

  it("is case-insensitive and differs across keys", () => {
    expect(tagKeyBackground("Project")).toBe(tagKeyBackground("project"));
    expect(tagKeyBackground("Project")).not.toBe(tagKeyBackground("Env"));
    // Continuous-hue hashing put these near each other; palette indexing should separate them.
    expect(tagKeyBackground("Feature")).not.toBe(tagKeyBackground("Project"));
  });

  it("hashString is deterministic", () => {
    expect(hashString("Project")).toBe(hashString("Project"));
  });
});

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

describe("bestTagAutocomplete", () => {
  const pool = ["work", "workout", "personal", "alpha"];

  it("returns null for empty draft", () => {
    expect(bestTagAutocomplete("", pool)).toBeNull();
    expect(bestTagAutocomplete("   ", pool)).toBeNull();
  });

  it("prefers the most recent prefix match", () => {
    expect(bestTagAutocomplete("wo", pool)).toBe("work");
  });

  it("skips already-selected tags", () => {
    expect(bestTagAutocomplete("wo", pool, ["work"])).toBe("workout");
  });

  it("falls back to substring match when no prefix", () => {
    expect(bestTagAutocomplete("pha", pool)).toBe("alpha");
  });

  it("is case-insensitive", () => {
    expect(bestTagAutocomplete("PER", pool)).toBe("personal");
  });
});
