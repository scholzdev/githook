# Installation

Githook can be installed in several ways, depending on your preference and use case. Choose the method that best fits your workflow.

## Quick Install (Recommended)

The fastest way to install Githook is to download a pre-built binary for your platform:

### macOS

```bash
# Intel
curl -L https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-apple-darwin -o githook
chmod +x githook
sudo mv githook /usr/local/bin/

# Apple Silicon
curl -L https://github.com/scholzdev/githook/releases/latest/download/githook-aarch64-apple-darwin -o githook
chmod +x githook
sudo mv githook /usr/local/bin/
```

### Linux

```bash
curl -L https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-unknown-linux-gnu -o githook
chmod +x githook
sudo mv githook /usr/local/bin/
```

### Windows

Download the latest `.exe` from [GitHub Releases](https://github.com/scholzdev/githook/releases) and add it to your PATH.

## Installation Methods

Choose your preferred installation method:

- **[From GitHub Releases](./releases.md)** - Pre-built binaries (recommended)
- **[Using Cargo](./cargo.md)** - Install from crates.io or Git
- **[From Source](./source.md)** - Build from source code
- **[Verify Installation](./verify.md)** - Check that everything works

## What's Next?

After installation, head to the [Quick Start Guide](../getting-started/README.md) to create your first `.ghook` file!
