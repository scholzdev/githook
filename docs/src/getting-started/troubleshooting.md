# Troubleshooting

Having issues with Githook? This guide covers common problems and their solutions.

## Installation Issues

### Command Not Found

**Problem:** `githook: command not found`

**Solutions:**

1. **Check if installed:**
   ```bash
   which githook
   ```

2. **Add to PATH:**
   ```bash
   echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc
   source ~/.zshrc
   ```

3. **Verify installation location:**
   ```bash
   ls -l /usr/local/bin/githook
   ```

### Permission Denied (macOS/Linux)

**Problem:** `permission denied: githook`

**Solution:**

```bash
chmod +x /usr/local/bin/githook
```

### macOS Gatekeeper Warning

**Problem:** "githook cannot be opened because the developer cannot be verified"

**Solution:**

```bash
xattr -d com.apple.quarantine /usr/local/bin/githook
```

Or manually allow in System Preferences → Security & Privacy.

## Hooks Not Running

### Hook Doesn't Execute

**Problem:** Git commits succeed even though hooks should block them

**Diagnosis:**

1. **Check if hooks are installed:**
   ```bash
   ls -la .git/hooks/
   ```
   You should see `pre-commit`, `commit-msg`, etc.

2. **Check hook permissions:**
   ```bash
   ls -l .git/hooks/pre-commit
   ```
   Should show executable permissions (`-rwxr-xr-x`).

**Solutions:**

```bash
# Reinstall hooks
mkdir -p .githook

# Make hooks executable
chmod +x .git/hooks/pre-commit
chmod +x .git/hooks/commit-msg
```

### Hooks Run But Don't Block

**Problem:** Hooks execute but commits still go through

**Diagnosis:**

Check if you're using `warn_if` instead of `block_if`:

```javascript
# ❌ This only warns
warn_if file_size > 1000000 message "Large file"

# ✅ This blocks
block_if file_size > 1000000 message "Large file"
```

### Wrong Hook Triggered

**Problem:** Rules run at the wrong time

**Solution:**

Make sure your `.ghook` file name matches the hook type:

```
.githook/
├── pre-commit.ghook     ← Runs before commit
├── commit-msg.ghook     ← Runs after commit message
└── pre-push.ghook       ← Runs before push
```

## Syntax Errors

### Parse Error

**Problem:**
```
Error: Parse error at line 5
```

**Solution:**

Validate your syntax:

```bash
githook pre-commit .githook/pre-commit.ghook --verbose
```

Common syntax errors:

```javascript
# ❌ Missing quotes
block_if file matches test.txt

# ✅ Correct
block_if file matches "test.txt"

# ❌ Wrong operator
block_if file_size = 1000

# ✅ Correct
block_if file_size == 1000

# ❌ Missing message
block_if file_size > 1000

# ✅ Correct
block_if file_size > 1000 message "Too large"
```

### Invalid Operator

**Problem:**
```
Error: Invalid operator: 'equals'
```

**Solution:**

Use the correct operator syntax:

```javascript
# ❌ Wrong
block_if content equals "test"
block_if size greater_than 1000

# ✅ Correct
block_if content == "test"
block_if content contains "test"
block_if size > 1000
```

See [Operators](../language/operators.md) for full list.

## Runtime Errors

### Variable Not Found

**Problem:**
```
Error: Variable 'branch' not found
```

**Solution:**

Use the correct variable name:

```javascript
# ❌ Wrong
block_if branch == "main"

# ✅ Correct
block_if branch_name == "main"
```

See available variables in context:
- [Git context](../stdlib/git.md)
- [Time context](../stdlib/time.md)

### Property Not Available

**Problem:**
```
Error: Property 'content' not available in this context
```

**Solution:**

Some properties are only available in loops:

```javascript
# ❌ Won't work - content needs a file
block_if content contains "TODO"

# ✅ Correct - content available in foreach
foreach file in staged_files matching "*" {
  block_if content contains "TODO" message "TODO in {file}"
}
```

### File Not Found

**Problem:**
```
Error: File 'test.txt' not found
```

**Diagnosis:**

The file might not be staged:

```bash
git status  # Check which files are staged
```

**Solution:**

```bash
git add test.txt  # Stage the file
```

## Performance Issues

### Hooks Are Slow

**Problem:** Hooks take more than a few seconds to run

**Diagnosis:**

Run with timing:

```bash
time githook pre-commit
```

**Solutions:**

1. **Limit file patterns:**
   ```javascript
   # ❌ Checks all files (slow)
   foreach file in staged_files matching "*" {
     block_if content matches "password" message "Found password"
   }
   
   # ✅ Only checks source files (fast)
   foreach file in staged_files matching "*.{rs,js,ts,py}" {
     block_if content matches "password" message "Found password"
   }
   ```

2. **Use early returns:**
   ```javascript
   # Check file extension before reading content
   foreach file in staged_files matching "*" {
     continue_if extension not in [".rs", ".js", ".ts"]
     block_if content contains "password" message "Found password"
   }
   ```

3. **Avoid expensive operations:**
   ```javascript
   # ❌ Slow - complex regex on every file
   foreach file in staged_files matching "*" {
     block_if content matches "^.*password.*$" message "Found password"
   }
   
   # ✅ Fast - simple contains check
   foreach file in staged_files matching "*" {
     block_if content contains "password" message "Found password"
   }
   ```

## Git Integration Issues

### Hooks Skipped in CI

**Problem:** Hooks don't run in CI/CD

**Explanation:** This is intentional - Githook skips hooks when `CI=true`.

**Solution:**

Force hooks to run in CI:

```yaml
# GitHub Actions
- name: Run hooks
  run: GITHOOK_FORCE=1 githook pre-commit
```

### Worktree Issues

**Problem:** Hooks don't work in Git worktrees

**Solution:**

Initialize Githook in each worktree:

```bash
cd worktree-directory
mkdir -p .githook
```

### Submodule Issues

**Problem:** Hooks don't run in submodules

**Solution:**

Initialize Githook in each submodule:

```bash
git submodule foreach 'mkdir -p .githook'
```

## Update Issues

### Update Fails

**Problem:**
```
Error: Failed to download update
```

**Solutions:**

1. **Check internet connection**
2. **Check GitHub is accessible:**
   ```bash
   curl -I https://github.com
   ```
3. **Manual update:**
   ```bash
   curl -L https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-apple-darwin -o githook
   chmod +x githook
   sudo mv githook /usr/local/bin/
   ```

### Wrong Version After Update

**Problem:** `githook --version` shows old version

**Solution:**

Clear shell hash cache:

```bash
hash -r    # bash/zsh
rehash     # zsh
```

## Getting Help

### Enable Verbose Mode

```bash
githook pre-commit
```

### Enable Debug Mode

```bash
GITHOOK_DEBUG=1 githook pre-commit
```

### Check Version

```bash
githook --version
```

### Validate Syntax

```bash
githook pre-commit --verbose
```

### List All Snippets

```bash
githook list
```

## Common Error Messages

### "No .ghook files found"

**Cause:** `.githook/` directory doesn't exist or is empty

**Solution:**
```bash
mkdir -p .githook
```

### "Hook failed with exit code 1"

**Cause:** A validation rule blocked the operation

**Solution:** Read the error message and fix the issue, or skip the hook:
```bash
git commit --no-verify
```

### "Syntax error: unexpected token"

**Cause:** Invalid `.ghook` syntax

**Solution:**
```bash
githook pre-commit .githook/pre-commit.ghook --verbose
```

## Still Having Issues?

If you're still stuck:

1. **Check the docs:** Browse [Language Reference](../language/README.md) and [Examples](../examples/README.md)
2. **Search issues:** Check [GitHub Issues](https://github.com/scholzdev/githook/issues)
3. **Ask for help:** Open a [new issue](https://github.com/scholzdev/githook/issues/new) or [discussion](https://github.com/scholzdev/githook/discussions)

When reporting issues, include:
- Githook version (`githook --version`)
- Operating system
- The `.ghook` file content
- Full error message
- Output of `githook pre-commit --verbose`

## Next Steps

- [Language Guide](../language/README.md) - Learn the full syntax
- [Examples](../examples/README.md) - See working examples
- [CLI Reference](../cli/README.md) - All available commands
