import * as child_process from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";
import { getLspPath, getProduction, getDuplicationMode, getDuplicationThreshold } from "./config.js";
import { getInstalledBinaryPath } from "./download.js";
import type {
  FallowCheckResult,
  FallowDupesResult,
  FallowFixResult,
} from "./types.js";

const findCliBinary = (context: vscode.ExtensionContext): string | null => {
  const lspPath = getLspPath();
  if (lspPath) {
    const dir = path.dirname(lspPath);
    const ext = os.platform() === "win32" ? ".exe" : "";
    const cliPath = path.join(dir, `fallow${ext}`);
    if (fs.existsSync(cliPath)) {
      return cliPath;
    }
  }

  const ext = os.platform() === "win32" ? ".exe" : "";
  const pathDirs = (process.env["PATH"] ?? "").split(path.delimiter);
  for (const dir of pathDirs) {
    const candidate = path.join(dir, `fallow${ext}`);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  const installed = getInstalledBinaryPath(context);
  if (installed) {
    const dir = path.dirname(installed);
    const cliPath = path.join(dir, `fallow${ext}`);
    if (fs.existsSync(cliPath)) {
      return cliPath;
    }
  }

  return null;
};

const execFallow = (
  context: vscode.ExtensionContext,
  args: ReadonlyArray<string>,
  cwd: string
): Promise<string> =>
  new Promise((resolve, reject) => {
    const binary = findCliBinary(context);
    if (!binary) {
      reject(new Error("fallow CLI binary not found in PATH."));
      return;
    }

    // Using execFile (not exec) to avoid shell injection
    child_process.execFile(
      binary,
      [...args],
      { cwd, maxBuffer: 50 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (error) {
          // Exit code 1 means issues found (expected), only reject on real errors
          const exitCode = (error as unknown as { status: number }).status;
          if (exitCode !== 1) {
            reject(new Error(stderr || error.message));
            return;
          }
        }
        resolve(stdout);
      }
    );
  });

const getWorkspaceRoot = (): string | null => {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    return null;
  }
  return folders[0].uri.fsPath;
};

export const runAnalysis = async (
  context: vscode.ExtensionContext
): Promise<{
  check: FallowCheckResult | null;
  dupes: FallowDupesResult | null;
}> => {
  const root = getWorkspaceRoot();
  if (!root) {
    void vscode.window.showWarningMessage("Fallow: no workspace folder open.");
    return { check: null, dupes: null };
  }

  let check: FallowCheckResult | null = null;
  let dupes: FallowDupesResult | null = null;

  try {
    const checkArgs = ["check", "--format", "json", "--quiet"];
    if (getProduction()) {
      checkArgs.push("--production");
    }

    const dupesArgs = ["dupes", "--format", "json", "--quiet"];
    dupesArgs.push("--mode", getDuplicationMode());
    dupesArgs.push("--threshold", String(getDuplicationThreshold()));

    const [checkOutput, dupesOutput] = await Promise.all([
      execFallow(context, checkArgs, root),
      execFallow(context, dupesArgs, root),
    ]);

    try {
      check = JSON.parse(checkOutput) as FallowCheckResult;
    } catch {
      // Check output may be empty or non-JSON on error
    }

    try {
      dupes = JSON.parse(dupesOutput) as FallowDupesResult;
    } catch {
      // Dupes output may be empty or non-JSON on error
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(`Fallow analysis failed: ${message}`);
  }

  return { check, dupes };
};

export const runFix = async (
  context: vscode.ExtensionContext,
  dryRun: boolean
): Promise<FallowFixResult | null> => {
  const root = getWorkspaceRoot();
  if (!root) {
    void vscode.window.showWarningMessage("Fallow: no workspace folder open.");
    return null;
  }

  const args = dryRun
    ? ["fix", "--dry-run", "--format", "json", "--quiet"]
    : ["fix", "--yes", "--format", "json", "--quiet"];

  if (getProduction()) {
    args.push("--production");
  }

  if (!dryRun) {
    const confirm = await vscode.window.showWarningMessage(
      "Fallow: This will remove unused exports and dependencies. Continue?",
      "Yes",
      "No"
    );
    if (confirm !== "Yes") {
      return null;
    }
  }

  try {
    const output = await execFallow(context, args, root);
    const result = JSON.parse(output) as FallowFixResult;

    if (dryRun) {
      const fixCount = result.fixes.length;
      void vscode.window.showInformationMessage(
        `Fallow: ${fixCount} fix${fixCount === 1 ? "" : "es"} available. Run "Fallow: Auto-Fix" to apply.`
      );
    } else {
      const fixCount = result.fixes.length;
      void vscode.window.showInformationMessage(
        `Fallow: applied ${fixCount} fix${fixCount === 1 ? "" : "es"}.`
      );
    }

    return result;
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(`Fallow fix failed: ${message}`);
    return null;
  }
};
