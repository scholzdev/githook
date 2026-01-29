# Githook Documentation

This directory contains the complete documentation for Githook, built with [mdBook](https://rust-lang.github.io/mdBook/).

## Building the Documentation

### Prerequisites

Install mdBook:

```bash
cargo install mdbook
```

### Build

```bash
cd docs
mdbook build
```

The built documentation will be in `docs/book/`.

### Serve Locally

```bash
cd docs
mdbook serve --open
```

Then visit http://localhost:3000

### Watch Mode

mdBook automatically rebuilds when you edit files:

```bash
mdbook serve
```

## Structure

```
docs/
├── book.toml                # Configuration
├── src/
│   ├── SUMMARY.md          # Table of contents
│   ├── introduction.md     # Home page
│   ├── installation/       # Installation guides
│   ├── getting-started/    # Quick start tutorials
│   ├── language/           # Language reference
│   ├── examples/           # Real-world examples
│   ├── stdlib/             # Standard library docs
│   ├── cli/                # CLI command reference
│   ├── advanced/           # Advanced topics
│   ├── contributing.md     # Contributing guide
│   └── architecture.md     # Architecture overview
└── book/                   # Built documentation (gitignored)
```

## Contributing

To contribute to the documentation:

1. Edit the relevant `.md` files in `docs/src/`
2. Run `mdbook serve` to preview changes
3. Submit a pull request

## Deployment

The documentation is automatically built and deployed to GitHub Pages on every push to `main`.

## License

MIT © Florian Scholz
