import type { WikiIndexIssue } from "@/lib/types";

export type WikiFrontMatterFileGroup = {
  key: string;
  root_id: string;
  rel_path: string;
  absolute_path: string;
  issues: WikiIndexIssue[];
  repairable: boolean;
  repair_label: string;
};

export function groupWikiIndexIssues(issues: WikiIndexIssue[]): WikiFrontMatterFileGroup[] {
  const groups = new Map<string, WikiFrontMatterFileGroup>();
  for (const issue of issues) {
    const key = `${issue.root_id}:${issue.rel_path}`;
    const existing = groups.get(key);
    if (existing) {
      existing.issues.push(issue);
      existing.repairable = existing.repairable && issue.repairable;
      continue;
    }
    groups.set(key, {
      key,
      root_id: issue.root_id,
      rel_path: issue.rel_path,
      absolute_path: issue.absolute_path,
      issues: [issue],
      repairable: issue.repairable,
      repair_label: issue.repair_label ?? "Fix",
    });
  }
  return [...groups.values()].map((group) => ({
    ...group,
    repairable: group.issues.every((issue) => issue.repairable),
    repair_label:
      group.issues.find((issue) => issue.repair_label)?.repair_label ?? "Fix front matter",
  }));
}
