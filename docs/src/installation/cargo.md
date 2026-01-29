# Using Cargo

If you have Rust installed, you can install Githook using Cargo, Rust's package manager.

## Prerequisites

You need Rust 1.75 or later. Install Rust from [rustup.rs](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Install from crates.io

Once Githook is published to crates.io, you can install it with:

```bash
cargo install githook-cli
```

This will download, compile, and install the latest version of Githook.

## Install from Git Repository

To install the latest development version directly from GitHub:

```bash
cargo install --git https://github.com/scholzdev/githook githook-cli
```

### Install a Specific Version

```bash
cargo install --git https://github.com/scholzdev/githook --tag v0.1.0 githook-cli
```

### Install a Specific Branch

```bash
cargo install --git https://github.com/scholzdev/githook --branch main githook-cli
```

## Verify Installation

After installation, verify that Githook is available:

```bash
githook --version
```

You should see output like:
```
githook 0.1.0
```

## Update

To update to the latest version:

```bash
# From crates.io
cargo install githook-cli --force

# From Git
cargo install --git https://github.com/scholzdev/githook githook-cli --force
```

The `--force` flag tells Cargo to overwrite the existing binary.

## Uninstall

To remove Githook installed via Cargo:

```bash
cargo uninstall githook-cli
```

## Benefits of Cargo Installation

✅ Always get the latest features  
✅ Easy to update  
✅ Works on all platforms Rust supports  
✅ Can install development versions  

## Troubleshooting

### Cargo Not Found

If `cargo` is not found, make sure Rust is installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Compilation Errors

If you encounter compilation errors, ensure you have the latest Rust version:

```bash
rustup update
```

### Binary Not in PATH

If `githook` is not found after installation, add Cargo's bin directory to your PATH:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

## Next Steps

Continue to [Verify Installation](./verify.md) to make sure everything works correctly.
