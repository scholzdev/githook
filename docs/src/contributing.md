# Contributing

Thank you for your interest in contributing to Githook!

## Ways to Contribute

- 🐛 Report bugs
- 💡 Suggest features
- 📝 Improve documentation
- 💻 Submit code changes
- 🎨 Enhance VSCode extension

## Development Setup

```bash
# Clone repository
git clone https://github.com/scholzdev/githook.git
cd githook

# Build
cargo build --release

# Run tests
cargo test

# Run locally
cargo run -- --help
```

## Project Structure

```
githook/
├── crates/
│   ├── githook/          # Core library
│   ├── githook-cli/      # CLI interface
│   ├── githook-syntax/   # Parser and lexer
│   ├── githook-eval/     # Runtime interpreter
│   ├── githook-git/      # Git integration
│   └── githook-lsp/      # LSP server
├── docs/                 # This documentation
└── vscode-extension/     # VSCode extension
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run tests and linting
6. Submit PR with clear description

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add tests for new features
- Update documentation

## Questions?

Open a [Discussion](https://github.com/scholzdev/githook/discussions) or [Issue](https://github.com/scholzdev/githook/issues).
