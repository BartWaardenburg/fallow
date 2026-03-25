import * as vscode from "vscode";
import type { FallowCheckResult, FallowDupesResult } from "./types.js";

/** Summary stats pushed by the LSP server via fallow/analysisComplete. */
export interface AnalysisCompleteParams {
  totalIssues: number;
  unusedFiles: number;
  unusedExports: number;
  unusedTypes: number;
  unusedDependencies: number;
  unusedDevDependencies: number;
  unusedEnumMembers: number;
  unusedClassMembers: number;
  unresolvedImports: number;
  unlistedDependencies: number;
  duplicateExports: number;
  typeOnlyDependencies: number;
  circularDependencies: number;
  duplicationPercentage: number;
  cloneGroups: number;
}

let statusBarItem: vscode.StatusBarItem | null = null;

export const createStatusBar = (): vscode.StatusBarItem => {
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    50
  );
  statusBarItem.command = "fallow.analyze";
  statusBarItem.text = "$(search) Fallow";
  statusBarItem.show();
  return statusBarItem;
};

/** Update the status bar from CLI-driven analysis results. */
export const updateStatusBar = (
  checkResult: FallowCheckResult | null,
  dupesResult: FallowDupesResult | null
): void => {
  if (!statusBarItem) {
    return;
  }

  const parts: string[] = [];

  if (checkResult) {
    const issueCount =
      checkResult.unused_files.length +
      checkResult.unused_exports.length +
      checkResult.unused_types.length +
      checkResult.unused_dependencies.length +
      checkResult.unused_dev_dependencies.length +
      checkResult.unused_enum_members.length +
      checkResult.unused_class_members.length +
      checkResult.unresolved_imports.length +
      checkResult.unlisted_dependencies.length +
      checkResult.duplicate_exports.length +
      (checkResult.type_only_dependencies?.length ?? 0) +
      (checkResult.circular_dependencies?.length ?? 0);

    parts.push(`${issueCount} issues`);
  }

  if (dupesResult) {
    const pct = dupesResult.stats.duplication_percentage.toFixed(1);
    parts.push(`${pct}% duplication`);
  }

  applyStatusBarText(parts);
};

/** Update the status bar from LSP notification data. */
export const updateStatusBarFromLsp = (params: AnalysisCompleteParams): void => {
  if (!statusBarItem) {
    return;
  }

  const dupPct = Number.isFinite(params.duplicationPercentage)
    ? params.duplicationPercentage
    : 0;

  const parts: string[] = [];
  parts.push(`${params.totalIssues} issues`);
  parts.push(`${dupPct.toFixed(1)}% duplication`);

  // Color-code by severity
  const hasErrors = params.unresolvedImports > 0;
  const hasWarnings = params.totalIssues > 0;

  if (hasErrors) {
    statusBarItem.backgroundColor = new vscode.ThemeColor(
      "statusBarItem.errorBackground"
    );
  } else if (hasWarnings) {
    statusBarItem.backgroundColor = new vscode.ThemeColor(
      "statusBarItem.warningBackground"
    );
  } else {
    statusBarItem.backgroundColor = undefined;
  }

  // Build rich markdown tooltip with breakdown
  const lines: string[] = ["**Fallow** — Analysis Results\n"];

  if (params.unresolvedImports > 0) {
    lines.push(`$(error) ${params.unresolvedImports} unresolved imports`);
  }
  if (params.unusedFiles > 0) {
    lines.push(`$(warning) ${params.unusedFiles} unused files`);
  }
  if (params.unusedExports > 0) {
    lines.push(`$(warning) ${params.unusedExports} unused exports`);
  }
  if (params.unusedTypes > 0) {
    lines.push(`$(info) ${params.unusedTypes} unused types`);
  }
  if (params.unusedDependencies > 0) {
    lines.push(`$(warning) ${params.unusedDependencies} unused dependencies`);
  }
  if (params.unusedDevDependencies > 0) {
    lines.push(`$(warning) ${params.unusedDevDependencies} unused dev dependencies`);
  }
  if (params.unusedEnumMembers > 0) {
    lines.push(`$(info) ${params.unusedEnumMembers} unused enum members`);
  }
  if (params.unusedClassMembers > 0) {
    lines.push(`$(info) ${params.unusedClassMembers} unused class members`);
  }
  if (params.unlistedDependencies > 0) {
    lines.push(`$(warning) ${params.unlistedDependencies} unlisted dependencies`);
  }
  if (params.duplicateExports > 0) {
    lines.push(`$(warning) ${params.duplicateExports} duplicate exports`);
  }
  if (params.typeOnlyDependencies > 0) {
    lines.push(`$(info) ${params.typeOnlyDependencies} type-only dependencies`);
  }
  if (params.circularDependencies > 0) {
    lines.push(`$(warning) ${params.circularDependencies} circular dependencies`);
  }
  if (params.cloneGroups > 0) {
    lines.push(`$(copy) ${params.cloneGroups} clone groups (${dupPct.toFixed(1)}% duplication)`);
  }

  if (params.totalIssues === 0 && params.cloneGroups === 0) {
    lines.push("$(check) No issues found");
  }

  lines.push("\n---\n");
  lines.push("[$(play) Run Analysis](command:fallow.analyze) · [$(wrench) Auto-Fix](command:fallow.fix) · [$(output) Output](command:fallow.showOutput)");

  const tooltip = new vscode.MarkdownString(lines.join("\n\n"));
  tooltip.isTrusted = true;
  statusBarItem.tooltip = tooltip;

  applyStatusBarText(parts);
};

const applyStatusBarText = (parts: string[]): void => {
  if (!statusBarItem) {
    return;
  }
  if (parts.length > 0) {
    statusBarItem.text = `$(search) Fallow: ${parts.join(" | ")}`;
  } else {
    statusBarItem.text = "$(search) Fallow";
  }
};

export const setStatusBarAnalyzing = (): void => {
  if (statusBarItem) {
    statusBarItem.text = "$(loading~spin) Fallow: Analyzing...";
  }
};

export const setStatusBarError = (): void => {
  if (statusBarItem) {
    statusBarItem.text = "$(error) Fallow: Error";
  }
};

export const disposeStatusBar = (): void => {
  if (statusBarItem) {
    statusBarItem.dispose();
    statusBarItem = null;
  }
};
