import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { execSync } from 'child_process';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    State
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let statusBarItem: vscode.StatusBarItem;

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
            // Returns the first match if 'where/which' returns multiple lines
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
 * Updates the bottom tray status bar item based on LSP client state.
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
    const binaryPath = resolveLspBinary(context);

    if (!binaryPath) {
        updateStatusBar(State.Stopped, 'Binary Missing');
        const choice = await vscode.window.showWarningMessage(
            'Vinyl Language Server (vinyl-lsp) was not found in PATH or settings.',
            'Configure Path'
        );
        if (choice === 'Configure Path') {
            vscode.commands.executeCommand('workbench.action.openSettings', 'vinyl.lsp.path');
        }
        return;
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

    client.onDidChangeState((event) => {
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
    // Ensure global storage directory exists
    if (!fs.existsSync(context.globalStorageUri.fsPath)) {
        fs.mkdirSync(context.globalStorageUri.fsPath, { recursive: true });
    }

    // Initialize status bar item
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.command = 'vinyl.showOutputChannel';
    context.subscriptions.push(statusBarItem);

    // Register commands
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

    // Start the server
    await startClient(context);
}

export async function deactivate(): Promise<void> {
    if (client && client.isRunning()) {
        await client.stop();
    }
}
