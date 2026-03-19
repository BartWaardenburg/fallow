import * as fs from "node:fs";
import * as https from "node:https";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";

const GITHUB_REPO = "fallow-rs/fallow";
const BINARY_NAME = "fallow-lsp";

interface GithubRelease {
  readonly tag_name: string;
  readonly assets: ReadonlyArray<{
    readonly name: string;
    readonly browser_download_url: string;
  }>;
}

const getPlatformTarget = (): string | null => {
  const arch = os.arch();
  const platform = os.platform();

  if (platform === "darwin" && arch === "arm64") return "darwin-arm64";
  if (platform === "darwin" && arch === "x64") return "darwin-x64";
  if (platform === "linux" && arch === "x64") return "linux-x64-gnu";
  if (platform === "linux" && arch === "arm64") return "linux-arm64-gnu";
  if (platform === "win32" && arch === "x64") return "win32-x64-msvc";

  return null;
};

const httpsGet = (url: string): Promise<string> =>
  new Promise((resolve, reject) => {
    const request = https.get(
      url,
      { headers: { "User-Agent": "fallow-vscode" } },
      (response) => {
        if (
          response.statusCode &&
          response.statusCode >= 300 &&
          response.statusCode < 400 &&
          response.headers.location
        ) {
          httpsGet(response.headers.location).then(resolve, reject);
          return;
        }

        if (response.statusCode && response.statusCode >= 400) {
          reject(new Error(`HTTP ${response.statusCode}`));
          return;
        }

        const chunks: Buffer[] = [];
        response.on("data", (chunk: Buffer) => chunks.push(chunk));
        response.on("end", () => resolve(Buffer.concat(chunks).toString()));
        response.on("error", reject);
      }
    );
    request.on("error", reject);
  });

const httpsDownload = (url: string, dest: string): Promise<void> =>
  new Promise((resolve, reject) => {
    const request = https.get(
      url,
      { headers: { "User-Agent": "fallow-vscode" } },
      (response) => {
        if (
          response.statusCode &&
          response.statusCode >= 300 &&
          response.statusCode < 400 &&
          response.headers.location
        ) {
          httpsDownload(response.headers.location, dest).then(resolve, reject);
          return;
        }

        if (response.statusCode && response.statusCode >= 400) {
          reject(new Error(`HTTP ${response.statusCode}`));
          return;
        }

        const file = fs.createWriteStream(dest);
        response.pipe(file);
        file.on("finish", () => {
          file.close();
          resolve();
        });
        file.on("error", (err) => {
          fs.unlink(dest, () => {});
          reject(err);
        });
      }
    );
    request.on("error", reject);
  });

export const getInstallDir = (context: vscode.ExtensionContext): string => {
  const dir = path.join(context.globalStorageUri.fsPath, "bin");
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
  return dir;
};

export const getInstalledBinaryPath = (
  context: vscode.ExtensionContext
): string | null => {
  const dir = getInstallDir(context);
  const ext = os.platform() === "win32" ? ".exe" : "";
  const binaryPath = path.join(dir, `${BINARY_NAME}${ext}`);
  return fs.existsSync(binaryPath) ? binaryPath : null;
};

export const downloadBinary = async (
  context: vscode.ExtensionContext
): Promise<string | null> => {
  const target = getPlatformTarget();
  if (!target) {
    void vscode.window.showErrorMessage(
      `Fallow: unsupported platform ${os.platform()}-${os.arch()}`
    );
    return null;
  }

  return vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Fallow: Downloading language server...",
      cancellable: false,
    },
    async () => {
      try {
        const releaseJson = await httpsGet(
          `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`
        );
        const release: GithubRelease = JSON.parse(releaseJson);

        const ext = os.platform() === "win32" ? ".exe" : "";
        const assetName = `${BINARY_NAME}-${target}${ext}`;
        const asset = release.assets.find((a) => a.name === assetName);

        if (!asset) {
          // Try with .tar.gz or .zip archive patterns
          const archiveAsset = release.assets.find(
            (a) =>
              a.name.includes(target) && a.name.includes(BINARY_NAME)
          );
          if (!archiveAsset) {
            void vscode.window.showErrorMessage(
              `Fallow: no binary found for ${target} in release ${release.tag_name}`
            );
            return null;
          }
          void vscode.window.showErrorMessage(
            `Fallow: archive downloads not yet supported. Please download ${archiveAsset.name} manually.`
          );
          return null;
        }

        const dir = getInstallDir(context);
        const destPath = path.join(dir, `${BINARY_NAME}${ext}`);

        await httpsDownload(asset.browser_download_url, destPath);

        if (os.platform() !== "win32") {
          fs.chmodSync(destPath, 0o755);
        }

        void vscode.window.showInformationMessage(
          `Fallow: language server ${release.tag_name} installed.`
        );
        return destPath;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        void vscode.window.showErrorMessage(
          `Fallow: failed to download language server: ${message}`
        );
        return null;
      }
    }
  );
};
