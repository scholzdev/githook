# check-update

Check if a new version of Githook is available.

## Usage

```bash
githook check-update
```

## Description

The `check-update` command checks GitHub releases to see if a newer version of Githook is available. It compares your current version with the latest published release.

## Example Output

If an update is available:
```
New version available: 0.0.2
Current version: 0.0.1
Run 'githook update' to install the latest version.
```

If you're up to date:
```
You are running the latest version (0.0.1)
```

## See Also

- [`update`](./update.md) - Install the latest version
- [Installation](../installation/README.md)