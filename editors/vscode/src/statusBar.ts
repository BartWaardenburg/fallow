import * as vscode from "vscode";
import type { FallowCheckResult, FallowDupesResult } from "./types.js";

let statusBarItem: vscode.StatusBarItem | null = null;

export const createStatusBar = (): vscode.StatusBarItem => {
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    50
  );
  statusBarItem.command = "fallow.analyze";
  statusBarItem.tooltip = "Fallow: Click to run analysis";
  statusBarItem.text = "$(search) Fallow";
  statusBarItem.show();
  return statusBarItem;
};

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
