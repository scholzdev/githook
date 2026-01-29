# From GitHub Releases

The easiest and fastest way to install Githook is to download a pre-compiled binary from GitHub Releases.

## Step 1: Download

Visit the [Releases page](https://github.com/scholzdev/githook/releases) and download the appropriate binary for your platform:

- **macOS Intel**: `githook-x86_64-apple-darwin`
- **macOS Apple Silicon**: `githook-aarch64-apple-darwin`
- **Linux x86_64**: `githook-x86_64-unknown-linux-gnu`
- **Windows**: `githook-x86_64-pc-windows-msvc.exe`

## Step 2: Install

### macOS / Linux

```bash
# Download (example for macOS Intel)
curl -L https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-apple-darwin -o githook

# Make it executable
chmod +x githook

# Move to a directory in your PATH
sudo mv githook /usr/local/bin/

# Verify installation
githook --version
```

### Windows

1. Download the `.exe` file
2. Place it in a directory (e.g., `C:\Program Files\githook\`)
3. Add that directory to your PATH:
   - Open System Properties → Environment Variables
   - Edit the `Path` variable under System Variables
   - Add the directory containing `githook.exe`
4. Open a new terminal and verify:
   ```cmd
   githook --version
   ```

## Updating

Githook includes a built-in update mechanism:

```bash
# Check for updates
githook check-update

# Download and install the latest version
githook update
```

The updater will:
- Fetch the latest release from GitHub
- Download the correct binary for your platform
- Replace the existing binary safely
- Preserve your configuration and hooks

## Manual Update

You can also manually download a new version and replace the binary:

```bash
# Download new version
curl -L https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-apple-darwin -o githook-new

# Replace old binary
chmod +x githook-new
sudo mv githook-new /usr/local/bin/githook

# Verify new version
githook --version
```

## Troubleshooting

### Permission Denied

If you get "permission denied" when running `githook`:

```bash
chmod +x /usr/local/bin/githook
```

### Command Not Found

If the terminal can't find `githook`:

1. Check that the binary is in a directory listed in your PATH:
   ```bash
   echo $PATH
   ```
2. Add the directory to your PATH:
   ```bash
   echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc  # or ~/.bashrc
   source ~/.zshrc
   ```

### macOS Gatekeeper

On macOS, you might see a security warning. To allow Githook:

```bash
xattr -d com.apple.quarantine /usr/local/bin/githook
```

Or go to System Preferences → Security & Privacy → General, and click "Allow Anyway".

## Next Steps

Now that Githook is installed, continue to [Verify Installation](./verify.md) to make sure everything works correctly.
