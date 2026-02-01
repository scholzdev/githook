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

    const config = vscode.workspace.getConfiguration('githook');
    let serverPath = config.get<string>('lsp.path') || 'githook-lsp';

    if (!path.isAbsolute(serverPath)) {
        const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        if (workspaceRoot) {
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

    const serverExecutable: Executable = {
        command: serverPath,
        args: [],
    };

    const serverOptions: ServerOptions = {
        run: serverExecutable,
        debug: serverExecutable,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'githook' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.ghook'),
        },
    };

    client = new LanguageClient(
        'githook',
        'Githook Language Server',
        serverOptions,
        clientOptions
    );

    client.start();

    vscode.window.showInformationMessage('Githook LSP started!');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
