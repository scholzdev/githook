# From Source

Building Githook from source gives you the most control and allows you to contribute to the project.

## Prerequisites

You need:
- **Rust 1.75 or later** - Install from [rustup.rs](https://rustup.rs/)
- **Git** - For cloning the repository

## Step 1: Clone the Repository

```bash
git clone https://github.com/scholzdev/githook.git
cd githook
```

## Step 2: Build

### Release Build (Recommended)

Build an optimized binary:

```bash
cargo build --release
```

The compiled binary will be at `target/release/githook`.

### Debug Build (for Development)

For faster compilation during development:

```bash
cargo build
```

The binary will be at `target/debug/githook`.

## Step 3: Install

Copy the binary to a directory in your PATH:

### macOS / Linux

```bash
sudo cp target/release/githook /usr/local/bin/
```

Or install it to your home directory (no sudo required):

```bash
mkdir -p ~/.local/bin
cp target/release/githook ~/.local/bin/
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Windows

```cmd
copy target\release\githook.exe C:\Program Files\githook\
```

Then add `C:\Program Files\githook\` to your PATH.

## Step 4: Verify

```bash
githook --version
```

## Development Build

If you're developing Githook, you can run it directly without installing:

```bash
cargo run -- --help
```

## Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p githook-syntax

# Run with output
cargo test -- --nocapture
```

## Building for Multiple Targets

### Cross-Compilation

Install the target and build:

```bash
# macOS Intel from Apple Silicon
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# Linux from macOS
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

### Using Cross

For easier cross-compilation, use [cross](https://github.com/cross-rs/cross):

```bash
cargo install cross
cross build --release --target x86_64-unknown-linux-gnu
```

## Build Optimization

### Smaller Binary Size

Edit `Cargo.toml` and add:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Enable Link Time Optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols
```

Then build:

```bash
cargo build --release
```

### Faster Compilation (Development)

Use a faster linker:

```bash
# macOS/Linux
cargo install -f cargo-binutils
rustup component add llvm-tools-preview

# Use mold (Linux)
sudo apt install mold
```

Add to `.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

## Update Your Local Build

Pull the latest changes and rebuild:

```bash
git pull
cargo build --release
sudo cp target/release/githook /usr/local/bin/
```

## Uninstall

```bash
sudo rm /usr/local/bin/githook
```

## Next Steps

Now that you've built and installed Githook from source, continue to [Verify Installation](./verify.md).

## Contributing

If you're building from source to contribute, check out our [Contributing Guide](../contributing.md) for development guidelines!
