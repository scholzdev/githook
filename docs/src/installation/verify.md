# Verify Installation

After installing Githook, let's make sure everything is working correctly.

## Check Version

First, verify that Githook is installed and accessible:

```bash
./target/release/githook --version
# or if installed globally:
githook --version
```

You should see output like:
```
githook 0.0.1
```

If you get `command not found`, see the troubleshooting section below.

## Check Help

View the available commands:

```bash
githook --help
```

You should see a list of available options and commands:
```
Git hook language and executor

Usage: githook [OPTIONS] [HOOK_TYPE] [HOOK_ARGS]...

Commands:
  list          List installed packages
  check-update  Check for updates
  update        Update to latest version
  help          Print help message

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Test in a Repository

Create a test repository to verify Githook works:

```bash
# Create a test directory
mkdir githook-test
cd githook-test

# Initialize git
git init

# Create hook directory
mkdir -p .githook

# Create a simple hook
cat > .githook/pre-commit.ghook << 'EOF'
foreach file in staged_files matching "*" {
  block_if file_size > 1048576 message "File {file} is too large (max 1MB)"
}
EOF
```

## Verify Hook Files

Check that the `.githook` directory was created:

```bash
ls -la .githook/
```

You should see:
```
.githook/
├── pre-commit.ghook
└── commit-msg.ghook
```

## Test a Hook

Let's test the pre-commit hook:

```bash
# Create a test file
echo "console.log('test')" > test.js

# Stage it
git add test.js

# Run the hook
githook pre-commit
```

You should see:
```
- Running .githook/pre-commit.ghook...
  ✓ Passed checks
✓ Hook passed!
```

This means Githook is working!

## Check Syntax

You can check if your `.ghook` files have valid syntax by running them:

```bash
githook pre-commit
```

If there are syntax errors, you'll see detailed error messages with line numbers.

## List Packages

Check available packages:

```bash
githook list
```

You should see a list of installed packages and cached remote packages.

## Common Issues

### Command Not Found

If `githook` is not found:

1. **Check PATH**: Make sure the installation directory is in your PATH:
   ```bash
   echo $PATH
   ```

2. **macOS/Linux**: Add to PATH:
   ```bash
   echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc
   source ~/.zshrc
   ```

3. **Windows**: Add to PATH through System Settings

### Permission Denied (macOS/Linux)

If you get "permission denied":

```bash
chmod +x /usr/local/bin/githook
```

### macOS Gatekeeper Warning

On macOS, you might see a security warning:

```bash
xattr -d com.apple.quarantine /usr/local/bin/githook
```

### Git Hooks Not Running

If Git hooks don't execute:

1. Check hook permissions:
   ```bash
   ls -l .git/hooks/pre-commit
   ```
   It should be executable (`-rwxr-xr-x`).

2. Recreate the Git hook:
   ```bash
   cat > .git/hooks/pre-commit << 'EOF'
#!/bin/sh
githook pre-commit "$@"
EOF
   chmod +x .git/hooks/pre-commit
   ```

## System Requirements

Githook requires:
- **Git 2.0+** (any recent version)
- **macOS 10.15+**, **Linux (glibc 2.31+)**, or **Windows 10+**
- No other dependencies needed!

## What's Next?

✅ Installation verified! Head to the [Quick Start Guide](../getting-started/README.md) to create your first custom hook.
