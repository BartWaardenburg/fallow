import * as vscode from "vscode";
import { startClient, stopClient, restartClient } from "./client.js";
import { onConfigChange } from "./config.js";
import { runAnalysis, runFix } from "./commands.js";
import {
  createStatusBar,
  updateStatusBar,
  setStatusBarAnalyzing,
  setStatusBarError,
  disposeStatusBar,
} from "./statusBar.js";
import { DeadCodeTreeProvider, DuplicatesTreeProvider } from "./treeView.js";
import type { FallowCheckResult, FallowDupesResult } from "./types.js";

let outputChannel: vscode.OutputChannel;
let lastCheckResult: FallowCheckResult | null = null;
let lastDupesResult: FallowDupesResult | null = null;

export const activate = async (
  context: vscode.ExtensionContext
): Promise<void> => {
  outputChannel = vscode.window.createOutputChannel("Fallow");
  context.subscriptions.push(outputChannel);

  const statusBar = createStatusBar();
  context.subscriptions.push(statusBar);

  const deadCodeProvider = new DeadCodeTreeProvider();
  const duplicatesProvider = new DuplicatesTreeProvider();

  context.subscriptions.push(
    vscode.window.registerTreeDataProvider("fallow.deadCode", deadCodeProvider)
  );
  context.subscriptions.push(
    vscode.window.registerTreeDataProvider(
      "fallow.duplicates",
      duplicatesProvider
    )
  );

  const updateViews = (): void => {
    deadCodeProvider.update(lastCheckResult);
    duplicatesProvider.update(lastDupesResult);
    updateStatusBar(lastCheckResult, lastDupesResult);
  };

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.analyze", async () => {
      setStatusBarAnalyzing();
      try {
        const { check, dupes } = await runAnalysis(context);
        lastCheckResult = check;
        lastDupesResult = dupes;
        updateViews();
      } catch {
        setStatusBarError();
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.fix", async () => {
      await runFix(context, false);
      // Re-run analysis after fix
      setStatusBarAnalyzing();
      try {
        const { check, dupes } = await runAnalysis(context);
        lastCheckResult = check;
        lastDupesResult = dupes;
        updateViews();
      } catch {
        setStatusBarError();
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.fixDryRun", async () => {
      await runFix(context, true);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.restart", async () => {
      outputChannel.appendLine("Restarting language server...");
      await restartClient(context, outputChannel);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.showOutput", () => {
      outputChannel.show();
    })
  );

  // No-op command used by Code Lens items (display-only, no action on click)
  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.noop", () => {})
  );

  // Watch for config changes that affect the LSP
  context.subscriptions.push(
    onConfigChange(async (e) => {
      if (
        e.affectsConfiguration("fallow.lspPath") ||
        e.affectsConfiguration("fallow.trace.server")
      ) {
        outputChannel.appendLine("Configuration changed, restarting server...");
        await restartClient(context, outputChannel);
      }
    })
  );

  // Start LSP client
  const client = await startClient(context, outputChannel);
  if (client) {
    context.subscriptions.push({ dispose: () => void stopClient() });
  }

  // Run initial background analysis for tree views and status bar (non-blocking)
  setStatusBarAnalyzing();
  void runAnalysis(context).then(({ check, dupes }) => {
    lastCheckResult = check;
    lastDupesResult = dupes;
    updateViews();
  }).catch(() => {
    setStatusBarError();
  });
};

export const deactivate = async (): Promise<void> => {
  disposeStatusBar();
  await stopClient();
};
