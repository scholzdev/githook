import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    Executable,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    console.log('Activating Githook extension');

    // Get LSP binary path from config or use default
    const config = vscode.workspace.getConfiguration('githook');
    let serverPath = config.get<string>('lsp.path') || 'githook-lsp';

    // If relative path, resolve from workspace
    if (!path.isAbsolute(serverPath)) {
        const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        if (workspaceRoot) {
            // Try to find in target/release first, then target/debug
            const releasePath = path.join(workspaceRoot, 'target', 'release', 'githook-lsp');
            const debugPath = path.join(workspaceRoot, 'target', 'debug', 'githook-lsp');
            
            if (require('fs').existsSync(releasePath)) {
                serverPath = releasePath;
            } else if (require('fs').existsSync(debugPath)) {
                serverPath = debugPath;
            }
        }
    }

    console.log(`Using LSP server at: ${serverPath}`);

    // Define the server executable
    const serverExecutable: Executable = {
        command: serverPath,
        args: [],
    };

    const serverOptions: ServerOptions = {
        run: serverExecutable,
        debug: serverExecutable,
    };

    // Options for the language client
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'githook' }],
        synchronize: {
            // Notify the server about file changes to .ghook files
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.ghook'),
        },
    };

    // Create the language client
    client = new LanguageClient(
        'githook',
        'Githook Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client (this also starts the server)
    client.start();

    vscode.window.showInformationMessage('Githook LSP started!');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
