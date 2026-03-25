import * as vscode from "vscode";
import { startClient, stopClient, restartClient } from "./client.js";
import { onConfigChange } from "./config.js";
import { runAnalysis, runFix } from "./commands.js";
import {
  createStatusBar,
  updateStatusBar,
  updateStatusBarFromLsp,
  setStatusBarAnalyzing,
  setStatusBarError,
  disposeStatusBar,
} from "./statusBar.js";
import type { AnalysisCompleteParams } from "./statusBar.js";
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

  // Use createTreeView to get visibility events — defer CLI analysis until the
  // tree view is first shown, avoiding a double analysis on activation (the LSP
  // runs its own analysis for diagnostics).
  let cliAnalysisRan = false;

  const triggerCliAnalysis = async (): Promise<void> => {
    setStatusBarAnalyzing();
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: "Fallow: Analyzing...",
        cancellable: false,
      },
      async () => {
        try {
          const { check, dupes } = await runAnalysis(context);
          lastCheckResult = check;
          lastDupesResult = dupes;
          updateViews();

          const issueCount = check
            ? check.unused_files.length +
              check.unused_exports.length +
              check.unused_types.length +
              check.unused_dependencies.length +
              check.unused_dev_dependencies.length +
              check.unused_enum_members.length +
              check.unused_class_members.length +
              check.unresolved_imports.length +
              check.unlisted_dependencies.length +
              check.duplicate_exports.length +
              (check.type_only_dependencies?.length ?? 0) +
              (check.circular_dependencies?.length ?? 0)
            : 0;

          if (issueCount > 0) {
            void vscode.window.showInformationMessage(
              `Fallow: found ${issueCount} issue${issueCount === 1 ? "" : "s"}. Open the Fallow sidebar to explore.`,
              "Open Sidebar"
            ).then((choice) => {
              if (choice === "Open Sidebar") {
                void vscode.commands.executeCommand("fallow.deadCode.focus");
              }
            });
          } else {
            void vscode.window.showInformationMessage(
              "Fallow: no issues found."
            );
          }
        } catch {
          setStatusBarError();
        }
      }
    );
  };

  const deadCodeView = vscode.window.createTreeView("fallow.deadCode", {
    treeDataProvider: deadCodeProvider,
  });
  deadCodeProvider.setView(deadCodeView);
  const duplicatesView = vscode.window.createTreeView("fallow.duplicates", {
    treeDataProvider: duplicatesProvider,
  });
  context.subscriptions.push(deadCodeView, duplicatesView);

  const onViewVisible = (): void => {
    if (cliAnalysisRan) {
      return;
    }
    cliAnalysisRan = true;
    void triggerCliAnalysis();
  };

  context.subscriptions.push(
    deadCodeView.onDidChangeVisibility((e) => {
      if (e.visible) {
        onViewVisible();
      }
    })
  );
  context.subscriptions.push(
    duplicatesView.onDidChangeVisibility((e) => {
      if (e.visible) {
        onViewVisible();
      }
    })
  );

  const updateViews = (): void => {
    deadCodeProvider.update(lastCheckResult);
    duplicatesProvider.update(lastDupesResult);
    updateStatusBar(lastCheckResult, lastDupesResult);
  };

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.analyze", async () => {
      cliAnalysisRan = true;
      await triggerCliAnalysis();
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.fix", async () => {
      // Save dirty editors first so the fix works on up-to-date content
      await vscode.workspace.saveAll(false);
      await runFix(context, false);
      // Restart LSP to force fresh analysis — the fix modified files on disk
      // bypassing VS Code's editor, so did_save never fires for those files
      await restartClient(context, outputChannel);
      // Re-run CLI analysis for tree views
      cliAnalysisRan = true;
      await triggerCliAnalysis();
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

  // Open the Fallow sidebar (used by walkthrough completion event)
  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.openSidebar", () => {
      void vscode.commands.executeCommand("fallow.deadCode.focus");
    })
  );

  // Open Fallow settings (used by walkthrough completion event)
  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.openSettings", () => {
      void vscode.commands.executeCommand(
        "workbench.action.openSettings",
        "fallow"
      );
    })
  );

  // Fallback command for Code Lens items with 0 references (display-only)
  context.subscriptions.push(
    vscode.commands.registerCommand("fallow.noop", () => {})
  );

  // Watch for config changes
  context.subscriptions.push(
    onConfigChange(async (e) => {
      const needsRestart =
        e.affectsConfiguration("fallow.lspPath") ||
        e.affectsConfiguration("fallow.trace.server") ||
        e.affectsConfiguration("fallow.issueTypes");

      const needsReanalysis =
        e.affectsConfiguration("fallow.production") ||
        e.affectsConfiguration("fallow.duplication") ||
        e.affectsConfiguration("fallow.issueTypes");

      if (needsRestart) {
        outputChannel.appendLine("Configuration changed, restarting server...");
        await restartClient(context, outputChannel);
      }

      if (needsReanalysis) {
        // Re-run CLI analysis for tree views and status bar
        // (sequenced after LSP restart if both apply)
        void triggerCliAnalysis();
      }
    })
  );

  // Start LSP client
  const client = await startClient(context, outputChannel);
  if (client) {
    context.subscriptions.push({ dispose: () => void stopClient() });

    // Handle custom LSP notification: update status bar from LSP data
    // so the extension shows results immediately without waiting for CLI
    const notificationDisposable = client.onNotification(
      "fallow/analysisComplete",
      (params: AnalysisCompleteParams) => {
        updateStatusBarFromLsp(params);
        void vscode.commands.executeCommand(
          "setContext",
          "fallow.hasAnalyzed",
          true
        );
      }
    );
    context.subscriptions.push(notificationDisposable);
  }

  // Show walkthrough on first install
  const walkthroughShown = context.globalState.get<boolean>(
    "fallow.walkthroughShown"
  );
  if (!walkthroughShown) {
    void context.globalState.update("fallow.walkthroughShown", true);
    void vscode.commands.executeCommand(
      "workbench.action.openWalkthrough",
      "fallow-rs.fallow-vscode#fallow.gettingStarted",
      false
    );
  }
};

export const deactivate = async (): Promise<void> => {
  disposeStatusBar();
  await stopClient();
};
