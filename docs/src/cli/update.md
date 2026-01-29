# githook update

Update to the latest version.

## Usage

```bash
githook check-update    # Check for updates
githook update          # Download and install
```

## Examples

```bash
# Check if update available
githook check-update

# Update to latest
githook update
```

## What It Does

1. Fetches latest release from GitHub
2. Downloads correct binary for your platform
3. Replaces existing binary
4. Verifies installation

## Manual Update

```bash
curl -L https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-apple-darwin -o githook
chmod +x githook
sudo mv githook /usr/local/bin/
```
