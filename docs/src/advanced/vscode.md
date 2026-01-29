# VSCode Extension

Githook VSCode extension with syntax highlighting and IntelliSense.

## Features

- 🎨 Syntax highlighting for `.ghook` files
- 💡 IntelliSense and autocompletion
- 🔍 Error diagnostics
- ▶️ Run hooks from command palette
- 🎯 Code lenses for quick actions

## Installation

1. Open VSCode
2. Search for "Githook" in Extensions
3. Click Install

## Usage

### Syntax Highlighting

`.ghook` files automatically get syntax highlighting.

### Run Hooks

Press `Cmd+Shift+P` and search for "Githook: Run Pre-Commit".

### Validate

The extension validates `.ghook` files as you type, showing errors inline.

## Configuration

```json
{
  "githook.lsp.enable": true,
  "githook.lsp.path": "/usr/local/bin/githook-lsp"
}
```

## Development

See the [extension README](../../vscode-extension/README.md) for development setup.
