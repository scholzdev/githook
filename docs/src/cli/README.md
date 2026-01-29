# CLI Commands

Complete reference for all Githook CLI commands.

## Commands Overview

- **Direct Hook Execution** - Run hooks like `githook pre-commit`, `githook commit-msg`, etc.
- [`githook list`](./list.md) - List installed packages
- [`githook check-update`](./check-update.md) - Check for updates
- [`githook update`](./update.md) - Update to latest version

## Global Options

```bash
githook --help                # Show help
githook --version             # Show version
```

## Quick Reference

```bash
# Run hooks directly
githook pre-commit
githook commit-msg
githook pre-push

# List packages
githook list

# Check for updates
githook check-update

# Update to latest
githook update
githook --check-update
githook --update
```
