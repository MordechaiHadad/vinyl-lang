import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as https from 'https';
import { execSync } from 'child_process';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  State,
  StateChangeEvent
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let statusBarItem: vscode.StatusBarItem;

interface TargetInfo {
  triple: string;
  ext: 'tar.gz' | 'zip';
}

/**
 * Maps the running OS and Architecture to GitHub release target triples.
 */
function getTargetInfo(): TargetInfo | undefined {
  const arch = process.arch;
  const platform = process.platform;

  if (platform === 'linux' && arch === 'x64') {
    return { triple: 'x86_64-unknown-linux-gnu', ext: 'tar.gz' };
  }
  if (platform === 'darwin' && arch === 'arm64') {
    return { triple: 'aarch64-apple-darwin', ext: 'tar.gz' };
  }
  if (platform === 'win32' && arch === 'x64') {
    return { triple: 'x86_64-pc-windows-msvc', ext: 'zip' };
  }
  return undefined;
}

/**
 * Resolves the path to the vinyl-lsp executable across:
 * 1. User settings (vinyl.lsp.path)
 * 2. System $PATH
 * 3. Extension global storage directory
 */
function resolveLspBinary(context: vscode.ExtensionContext): string | undefined {
  const config = vscode.workspace.getConfiguration('vinyl');
  const customPath = config.get<string>('lsp.path');

  // 1. Check custom user setting
  if (customPath && customPath.trim() !== '') {
    const resolvedPath = path.normalize(customPath.trim());
    if (fs.existsSync(resolvedPath)) {
      return resolvedPath;
    }
    vscode.window.showErrorMessage(
      `Vinyl LSP binary not found at configured path: ${resolvedPath}`
    );
    return undefined;
  }

  // 2. Check system $PATH
  try {
    const command = process.platform === 'win32' ? 'where vinyl-lsp' : 'which vinyl-lsp';
    const stdout = execSync(command, { encoding: 'utf8' }).trim();
    if (stdout) {
      return stdout.split(/\r?\n/)[0];
    }
  } catch {
    // Executable not found in PATH; fall through to storage check
  }

  // 3. Check Extension Storage Directory (~/.../globalStorage/vinyl-lsp)
  const binaryName = process.platform === 'win32' ? 'vinyl-lsp.exe' : 'vinyl-lsp';
  const storageBinary = path.join(context.globalStorageUri.fsPath, binaryName);

  if (fs.existsSync(storageBinary)) {
    return storageBinary;
  }

  return undefined;
}

/**
 * Helper to scan full releases array if /releases/latest does not contain the target asset.
 */
function fetchFromReleasesList(
  assetName: string,
  options: https.RequestOptions
): Promise<string | undefined> {
  const listUrl = 'https://api.github.com/repos/MordechaiHadad/vinyl-lang/releases';
  return new Promise((resolve) => {
    https.get(listUrl, options, (res) => {
      let data = '';
      res.on('data', (chunk) => (data += chunk));
      res.on('end', () => {
        try {
          const releases = JSON.parse(data);
          if (Array.isArray(releases)) {
            for (const release of releases) {
              const asset = release.assets?.find((a: { name: string }) => a.name === assetName);
              if (asset?.browser_download_url) {
                resolve(asset.browser_download_url);
                return;
              }
            }
          }
          resolve(undefined);
        } catch {
          resolve(undefined);
        }
      });
    }).on('error', () => resolve(undefined));
  });
}

/**
 * Fetches the download URL for the target binary from GitHub releases.
 * Checks /releases/latest first, falling back to iterating release entries.
 */
function fetchDownloadUrl(assetName: string): Promise<string | undefined> {
  const latestUrl = 'https://api.github.com/repos/MordechaiHadad/vinyl-lang/releases/latest';
  const options = {
    headers: {
      'User-Agent': 'vinyl-vscode-extension'
    }
  };

  return new Promise((resolve) => {
    https.get(latestUrl, options, (res) => {
      let data = '';
      res.on('data', (chunk) => (data += chunk));
      res.on('end', () => {
        try {
          const release = JSON.parse(data);
          const asset = release.assets?.find((a: { name: string }) => a.name === assetName);
          if (asset?.browser_download_url) {
            resolve(asset.browser_download_url);
            return;
          }
        } catch {
          // Fall through to release array scan
        }

        fetchFromReleasesList(assetName, options).then(resolve);
      });
    }).on('error', () => {
      fetchFromReleasesList(assetName, options).then(resolve);
    });
  });
}

/**
 * Downloads a file, following HTTP redirects (e.g. GitHub to S3).
 */
function downloadFile(url: string, destPath: string): Promise<boolean> {
  return new Promise((resolve) => {
    const request = (currentUrl: string) => {
      https.get(currentUrl, { headers: { 'User-Agent': 'vinyl-vscode-extension' } }, (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          request(res.headers.location);
          return;
        }

        if (res.statusCode !== 200) {
          resolve(false);
          return;
        }

        const fileStream = fs.createWriteStream(destPath);
        res.pipe(fileStream);
        fileStream.on('finish', () => {
          fileStream.close();
          resolve(true);
        });
        fileStream.on('error', () => {
          fs.unlink(destPath, () => {});
          resolve(false);
        });
      }).on('error', () => resolve(false));
    };

    request(url);
  });
}

/**
 * Downloads and extracts the prebuilt vinyl-lsp executable from GitHub releases.
 */
async function downloadAndExtractLsp(context: vscode.ExtensionContext): Promise<string | undefined> {
  const target = getTargetInfo();
  if (!target) {
    vscode.window.showErrorMessage(
      `Unsupported platform/architecture for Vinyl LSP: ${process.platform} ${process.arch}`
    );
    return undefined;
  }

  const assetName = `vinyl-lsp-${target.triple}.${target.ext}`;

  return vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: 'Vinyl Language Server',
      cancellable: false
    },
    async (progress) => {
      progress.report({ message: 'Checking GitHub releases for latest entry...' });
      const downloadUrl = await fetchDownloadUrl(assetName);

      if (!downloadUrl) {
        vscode.window.showErrorMessage(
          `Failed to find release asset '${assetName}' on GitHub (MordechaiHadad/vinyl-lang).`
        );
        return undefined;
      }

      const storageDir = context.globalStorageUri.fsPath;
      if (!fs.existsSync(storageDir)) {
        fs.mkdirSync(storageDir, { recursive: true });
      }

      const archivePath = path.join(storageDir, assetName);
      progress.report({ message: `Downloading ${assetName}...` });

      const success = await downloadFile(downloadUrl, archivePath);
      if (!success) {
        vscode.window.showErrorMessage('Failed to download Vinyl Language Server release archive.');
        return undefined;
      }

      progress.report({ message: 'Extracting binary...' });
      try {
        if (target.ext === 'tar.gz') {
          execSync(`tar -xzf "${archivePath}" -C "${storageDir}"`);
        } else if (target.ext === 'zip') {
          if (process.platform === 'win32') {
            execSync(
              `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${storageDir}' -Force"`
            );
          } else {
            execSync(`unzip -o "${archivePath}" -d "${storageDir}"`);
          }
        }

        // Clean up downloaded archive
        if (fs.existsSync(archivePath)) {
          fs.unlinkSync(archivePath);
        }

        const binaryName = process.platform === 'win32' ? 'vinyl-lsp.exe' : 'vinyl-lsp';
        const binaryPath = path.join(storageDir, binaryName);

        // Fallback rename if artifact was packaged without prefix
        if (!fs.existsSync(binaryPath)) {
          const fallbackName = process.platform === 'win32' ? 'lsp.exe' : 'lsp';
          const fallbackPath = path.join(storageDir, fallbackName);
          if (fs.existsSync(fallbackPath)) {
            fs.renameSync(fallbackPath, binaryPath);
          }
        }

        if (fs.existsSync(binaryPath)) {
          if (process.platform !== 'win32') {
            fs.chmodSync(binaryPath, 0o755);
          }
          vscode.window.showInformationMessage('Vinyl Language Server installed successfully!');
          return binaryPath;
        } else {
          vscode.window.showErrorMessage('Failed to locate vinyl-lsp binary inside extracted package.');
          return undefined;
        }
      } catch (err) {
        vscode.window.showErrorMessage(`Failed to extract language server package: ${err}`);
        return undefined;
      }
    }
  );
}

/**
 * Updates status bar tray icon based on LSP client state.
 */
function updateStatusBar(state: State, extraMessage?: string): void {
  if (!statusBarItem) {
    return;
  }

  switch (state) {
    case State.Running:
      statusBarItem.text = '$(check) Vinyl LSP';
      statusBarItem.tooltip = 'Vinyl Language Server is active';
      break;
    case State.Starting:
      statusBarItem.text = '$(sync~spin) Vinyl LSP: Starting';
      statusBarItem.tooltip = 'Vinyl Language Server is starting...';
      break;
    case State.Stopped:
      statusBarItem.text = `$(error) Vinyl LSP: ${extraMessage || 'Stopped'}`;
      statusBarItem.tooltip = 'Click to open Language Server output logs';
      break;
  }
  statusBarItem.show();
}

/**
 * Instantiates and starts the LanguageClient.
 */
async function startClient(context: vscode.ExtensionContext): Promise<void> {
  let binaryPath = resolveLspBinary(context);

  if (!binaryPath) {
    updateStatusBar(State.Stopped, 'Binary Missing');
    const choice = await vscode.window.showWarningMessage(
      'Vinyl Language Server (vinyl-lsp) was not found locally.',
      'Download from GitHub',
      'Configure Path'
    );

    if (choice === 'Download from GitHub') {
      binaryPath = await downloadAndExtractLsp(context);
    } else if (choice === 'Configure Path') {
      vscode.commands.executeCommand('workbench.action.openSettings', 'vinyl.lsp.path');
      return;
    }

    if (!binaryPath) {
      return;
    }
  }

  const serverOptions: ServerOptions = {
    run: { command: binaryPath, args: [] },
    debug: { command: binaryPath, args: [] }
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'vinyl' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.vn')
    },
    outputChannelName: 'Vinyl Language Server'
  };

  client = new LanguageClient(
    'vinylLsp',
    'Vinyl Language Server',
    serverOptions,
    clientOptions
  );

  client.onDidChangeState((event: StateChangeEvent) => {
    updateStatusBar(event.newState);
  });

  try {
    updateStatusBar(State.Starting);
    await client.start();
  } catch (error) {
    vscode.window.showErrorMessage(`Failed to start Vinyl Language Server: ${error}`);
    updateStatusBar(State.Stopped, 'Failed to Start');
  }
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  if (!fs.existsSync(context.globalStorageUri.fsPath)) {
    fs.mkdirSync(context.globalStorageUri.fsPath, { recursive: true });
  }

  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  statusBarItem.command = 'vinyl.showOutputChannel';
  context.subscriptions.push(statusBarItem);

  context.subscriptions.push(
    vscode.commands.registerCommand('vinyl.restartServer', async () => {
      if (client) {
        if (client.isRunning()) {
          await client.stop();
        }
        client = undefined;
      }
      await startClient(context);
      vscode.window.showInformationMessage('Vinyl Language Server restarted.');
    }),

    vscode.commands.registerCommand('vinyl.showOutputChannel', () => {
      if (client) {
        client.outputChannel.show();
      } else {
        vscode.window.showWarningMessage('Vinyl Language Server client is not initialized.');
      }
    })
  );

  await startClient(context);
}

export async function deactivate(): Promise<void> {
  if (client && client.isRunning()) {
    await client.stop();
  }
}
