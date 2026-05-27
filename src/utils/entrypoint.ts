import type { SessionEntrypointFilter } from "@/types/metadata.types";

export type EntrypointCategory = "cli" | "vscode" | "desktop";

export function normalizeEntrypoint(
  raw: string | null | undefined
): EntrypointCategory | null {
  switch (raw) {
    case "cli":
      return "cli";
    case "claude-vscode":
      return "vscode";
    case "claude-desktop":
      return "desktop";
    default:
      return null;
  }
}

export function matchesEntrypointFilter(
  raw: string | null | undefined,
  filter: SessionEntrypointFilter
): boolean {
  if (filter === "all") {
    return true;
  }
  return normalizeEntrypoint(raw) === filter;
}

export interface EntrypointBadgeMeta {
  i18nKey: string;
  badgeClass: string;
}

export const ENTRYPOINT_BADGE_META: Record<
  EntrypointCategory,
  EntrypointBadgeMeta
> = {
  cli: {
    i18nKey: "session.item.entrypoint.cli",
    badgeClass: "text-emerald-600 bg-emerald-500/10 dark:text-emerald-400",
  },
  vscode: {
    i18nKey: "session.item.entrypoint.vscode",
    badgeClass: "text-blue-600 bg-blue-500/10 dark:text-blue-400",
  },
  desktop: {
    i18nKey: "session.item.entrypoint.desktop",
    badgeClass: "text-purple-600 bg-purple-500/10 dark:text-purple-400",
  },
};

export const ENTRYPOINT_FILTER_LABEL_KEYS: Record<
  SessionEntrypointFilter,
  string
> = {
  all: "session.filter.source.all",
  cli: "session.filter.source.cli",
  vscode: "session.filter.source.vscode",
  desktop: "session.filter.source.desktop",
};

export const ENTRYPOINT_FILTER_OPTIONS: SessionEntrypointFilter[] = [
  "all",
  "cli",
  "vscode",
  "desktop",
];
