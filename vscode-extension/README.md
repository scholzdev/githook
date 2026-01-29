# Githook VSCode Extension

Language support for Githook (.ghook files) in Visual Studio Code.

## Features

- ✅ **Syntax Highlighting** - Full syntax highlighting for .ghook files with semantic tokens
- ✅ **Real-time Diagnostics** - Parse errors with precise line/column information
- ✅ **Auto-completion** - Smart completions for keywords, conditions, operators, and macros
- ✅ **Hover Information** - Documentation on hover for keywords and macro definitions
- ✅ **Go to Definition** - Jump to macro definitions (F12) with cross-file support
- ✅ **Find References** - Find all usages of a macro (Shift+F12)
- ✅ **Rename Symbol** - Rename macros everywhere (F2)
- ✅ **Document Symbols** - Outline view for macros and imports (Cmd+Shift+O)
- ✅ **Code Folding** - Fold macro definitions and blocks
- ✅ **Code Lens** - Reference counts above macro definitions
- ✅ **Document Links** - Clickable import paths (Cmd+Click)
- ✅ **Comments** - Single-line (#) and multi-line (/* */) comment support
- ✅ **Arrays** - Inline array syntax for foreach loops: `foreach ext in [".txt", ".rs"] { ... }`

## Installation

### From Source

1. Build the LSP server:
```bash
cd /Users/scholzf/dev/githook
cargo build --release -p githook-lsp
```

2. Install the extension:
```bash
cd vscode-extension
npm install
npm run compile
code --install-extension $(npm run package | grep "DONE" | awk '{print $NF}')
```

### Configuration

The extension will automatically find the `githook-lsp` binary in your PATH or in `target/release/`.

You can configure the LSP server path in VSCode settings:
```json
{
  "githook.lsp.path": "/path/to/githook-lsp"
}
```

## Development

To work on the extension:

```bash
cd vscode-extension
npm install
npm run watch  # Start TypeScript compiler in watch mode
```

Then press F5 in VSCode to launch the Extension Development Host.

## License

MIT
