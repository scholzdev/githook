"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const path = __importStar(require("path"));
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
let client;
function activate(context) {
    console.log('Activating Githook extension');
    const config = vscode.workspace.getConfiguration('githook');
    let serverPath = config.get('lsp.path') || 'githook-lsp';
    if (!path.isAbsolute(serverPath)) {
        const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        if (workspaceRoot) {
            const releasePath = path.join(workspaceRoot, 'target', 'release', 'githook-lsp');
            const debugPath = path.join(workspaceRoot, 'target', 'debug', 'githook-lsp');
            if (require('fs').existsSync(releasePath)) {
                serverPath = releasePath;
            }
            else if (require('fs').existsSync(debugPath)) {
                serverPath = debugPath;
            }
        }
    }
    console.log(`Using LSP server at: ${serverPath}`);
    const serverExecutable = {
        command: serverPath,
        args: [],
    };
    const serverOptions = {
        run: serverExecutable,
        debug: serverExecutable,
    };
    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'githook' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.ghook'),
        },
    };
    client = new node_1.LanguageClient('githook', 'Githook Language Server', serverOptions, clientOptions);
    client.start();
    vscode.window.showInformationMessage('Githook LSP started!');
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}