import * as vscode from "vscode";
import type { DuplicationMode, IssueTypeConfig, TraceLevel } from "./types.js";

const SECTION = "fallow";

const getConfig = (): vscode.WorkspaceConfiguration =>
  vscode.workspace.getConfiguration(SECTION);

export const getLspPath = (): string => getConfig().get<string>("lspPath", "");

export const getAutoDownload = (): boolean =>
  getConfig().get<boolean>("autoDownload", true);

export const getIssueTypes = (): IssueTypeConfig =>
  getConfig().get<IssueTypeConfig>("issueTypes", {
    "unused-files": true,
    "unused-exports": true,
    "unused-types": true,
    "unused-dependencies": true,
    "unused-dev-dependencies": true,
    "unused-enum-members": true,
    "unused-class-members": true,
    "unresolved-imports": true,
    "unlisted-dependencies": true,
    "duplicate-exports": true,
  });

export const getDuplicationThreshold = (): number =>
  getConfig().get<number>("duplication.threshold", 5);

export const getDuplicationMode = (): DuplicationMode =>
  getConfig().get<DuplicationMode>("duplication.mode", "mild");

export const getProduction = (): boolean =>
  getConfig().get<boolean>("production", false);

export const getTraceLevel = (): TraceLevel =>
  getConfig().get<TraceLevel>("trace.server", "off");

export const onConfigChange = (
  callback: (e: vscode.ConfigurationChangeEvent) => void
): vscode.Disposable =>
  vscode.workspace.onDidChangeConfiguration((e) => {
    if (e.affectsConfiguration(SECTION)) {
      callback(e);
    }
  });
