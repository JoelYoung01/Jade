import { describe, expect, it } from "vitest";

import {
  joinTextFilterTerms,
  matchesTextFilter,
  splitTextFilterTerms,
  textFilterHasTerm,
  toggleTagInTextFilter,
} from "@/lib/taskFilter";
import type { Tag, Task } from "@/lib/types";

function tag(name: string, id = name): Tag {
  return {
    id,
    name,
    created_at: "2026-01-01T00:00:00.000Z",
    updated_at: "2026-01-01T00:00:00.000Z",
  };
}

function task(partial: Partial<Task> & Pick<Task, "id" | "title">): Task {
  return {
    description: null,
    status: "inactive",
    due_at: "2026-07-20T15:00:00.000Z",
    repeat_cron: null,
    created_at: "2026-07-01T00:00:00.000Z",
    updated_at: "2026-07-01T00:00:00.000Z",
    deleted_at: null,
    tags: [],
    ...partial,
  };
}

describe("splitTextFilterTerms", () => {
  it("splits on | and trims empty segments", () => {
    expect(splitTextFilterTerms("foo | bar|| baz ")).toEqual(["foo", "bar", "baz"]);
  });

  it("returns empty for blank queries", () => {
    expect(splitTextFilterTerms("   ")).toEqual([]);
    expect(splitTextFilterTerms("|")).toEqual([]);
  });
});

describe("joinTextFilterTerms", () => {
  it("joins with spaced pipes", () => {
    expect(joinTextFilterTerms(["foo", "bar"])).toBe("foo | bar");
  });
});

describe("matchesTextFilter", () => {
  const sample = task({
    id: "1",
    title: "Ship Portal",
    description: "Finish docs",
    tags: [tag("Project:Portal"), tag("urgent")],
  });

  it("matches any OR term as a substring", () => {
    expect(matchesTextFilter(sample, "docs")).toBe(true);
    expect(matchesTextFilter(sample, "missing | urgent")).toBe(true);
    expect(matchesTextFilter(sample, "missing | nowhere")).toBe(false);
  });

  it("is case-insensitive and treats empty as match-all", () => {
    expect(matchesTextFilter(sample, "PORTAL")).toBe(true);
    expect(matchesTextFilter(sample, "  ")).toBe(true);
  });
});

describe("toggleTagInTextFilter", () => {
  it("appends and removes tag terms", () => {
    expect(toggleTagInTextFilter("", "urgent")).toBe("urgent");
    expect(toggleTagInTextFilter("docs", "urgent")).toBe("docs | urgent");
    expect(toggleTagInTextFilter("docs | urgent", "urgent")).toBe("docs");
    expect(toggleTagInTextFilter("urgent", "Urgent")).toBe("");
  });
});

describe("textFilterHasTerm", () => {
  it("checks exact OR terms", () => {
    expect(textFilterHasTerm("urgent | Project:Portal", "urgent")).toBe(true);
    expect(textFilterHasTerm("urgent", "urge")).toBe(false);
  });
});
