# Running Hooks

Learn how to execute, test, and debug your Githook validations.

## Automatic Execution

Once initialized, hooks run automatically during Git operations:

```bash
git commit    # Triggers pre-commit and commit-msg hooks
git push      # Triggers pre-push hook
git rebase    # Triggers pre-rebase hook
```

You don't need to do anything - Githook runs in the background!

## Manual Execution

You can manually run hooks without performing Git operations:

### Run a Specific Hook

```bash
githook pre-commit
```

This executes `.githook/pre-commit.ghook` on your currently staged files.

### Run Different Hook Types

```bash
githook commit-msg    # Validate commit message
githook pre-push      # Run pre-push checks
githook pre-rebase    # Run pre-rebase checks
```

## Testing and Debugging

### Run Manually

Test your hooks without committing:

```bash
# Stage some files
git add .

# Run the hook
githook pre-commit
```

The hook will check your staged files and show any errors or warnings.

## Validate Syntax

To check if your `.ghook` file is valid, simply run the hook:

```bash
githook pre-commit
```

If there are syntax errors, Githook will show detailed error messages with line numbers.

## Skipping Hooks

### Skip All Hooks (Git)

```bash
git commit --no-verify -m "Emergency fix"
# or
git commit -n -m "Skip hooks"
```

### Skip Githook Specifically

```bash
GITHOOK_SKIP=1 git commit -m "Skip Githook"
```

### Skip in CI/CD

Hooks are automatically skipped in most CI environments (when `CI=true`), but you can force them to run:

```bash
GITHOOK_FORCE=1 git commit -m "Run in CI"
```

## Listing Available Hooks

See which hooks are configured:

```bash
# List all .ghook files
ls .githook/

# List installed Git hooks
ls .git/hooks/
```

## Performance

### Check Execution Time

```bash
time githook pre-commit
```

### Optimize Slow Hooks

If your hooks are slow:

1. **Limit file patterns** - Don't check unnecessary files
2. **Use early returns** - Put fast checks first
3. **Cache results** - Githook caches some operations automatically
4. **Parallelize** - Githook runs checks in parallel when possible

Example optimization:

```javascript
# ❌ Slow - checks every file
foreach file in staged_files matching "*" {
  block_if content contains "password" message "Contains password"
}

# ✅ Fast - only checks code files
foreach file in staged_files matching "*.{rs,ts,js,py}" {
  block_if content contains "password" message "Contains password"
}
```

## Output Formats

### Default Format

```
- Running .githook/pre-commit.ghook...
  ✓ Passed checks
✓ Hook passed!
```

Or on failure:

```
- Running .githook/pre-commit.ghook...
  x File large.txt is too large (max 1MB)
✗ Hook blocked!
```

## Exit Codes

Githook uses standard exit codes:

| Code | Meaning |
|------|---------|
| 0 | Success - all checks passed |
| 1 | Failure - one or more checks failed |
| 2 | Error - syntax error or runtime error |

Use in scripts:

```bash
if githook pre-commit; then
    echo "All checks passed!"
else
    echo "Checks failed, aborting deploy"
    exit 1
fi
```

## Running in Different Contexts

### Pre-Commit Hook

Runs automatically before `git commit`:

```bash
git add file.txt
git commit -m "message"  # Triggers pre-commit.ghook
```

What's available:
- `staged_files` - Files in staging area
- `modified_lines` - Lines changed
- File content via `content`

### Commit-Msg Hook

Runs after you write commit message:

```bash
git commit -m "fix bug"  # Triggers commit-msg.ghook
```

What's available:
- `commit_message` - The commit message text
- All other Git context

Example commit-msg.ghook:

```javascript
block_if commit_message not matches "^(feat|fix|docs):" 
  message "Must follow conventional commits"

block_if commit_message matches "WIP" 
  message "Remove WIP before committing"
```

### Pre-Push Hook

Runs before `git push`:

```bash
git push  # Triggers pre-push.ghook
```

What's available:
- `branch_name` - Current branch
- `commits_ahead` - Commits to be pushed
- All staged/committed files

Example pre-push.ghook:

```javascript
# Block pushing to main
block_if branch_name matches "^(main|master)$" 
  message "Cannot push directly to main"

# Ensure tests pass
block_if commits_ahead > 0 message "Run tests before pushing"
```

## IDE Integration

### VSCode

The Githook VSCode extension provides:
- ▶️ Run hooks from command palette
- 🎯 Code lens to run individual rules
- ⚡ Instant feedback while editing

### Command Line from Editor

Run hooks from your editor's terminal:

```bash
:!githook pre-commit    # Vim
M-x shell-command githook pre-commit    # Emacs
```

## Next Steps

- [Troubleshooting](./troubleshooting.md) - Fix common issues
- [Language Guide](../language/README.md) - Learn the full language
- [Examples](../examples/README.md) - Real-world hook examples
