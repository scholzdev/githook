# Repository Setup

Learn how to properly set up Githook in your Git repository and configure it for your team.

## Basic Setup

### Create Hook Directory

In your Git repository:

```bash
mkdir -p .githook
```

This creates the `.githook/` directory where you'll place your hook files:

```
.githook/
├── pre-commit.ghook      # Runs before commit
├── commit-msg.ghook      # Validates commit messages
└── pre-push.ghook        # Runs before push
```

### Install Git Hooks

Git doesn't know about `.ghook` files automatically. We need to install hooks that call Githook.

### Quick Setup (Recommended)

Download and run the setup script:

```bash
# Download setup script
curl -sSL https://raw.githubusercontent.com/scholzdev/githook/main/setup-hooks.sh -o setup-hooks.sh

# Run it
bash setup-hooks.sh
```

This will automatically install hooks for all your `.ghook` files.

### Manual Setup

If you prefer to install hooks manually:

```bash
# Create pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/sh
githook pre-commit "$@"
EOF

chmod +x .git/hooks/pre-commit
```

Do the same for other hooks you need:
- `commit-msg` - Validate commit messages
- `pre-push` - Run before pushing
- `post-commit` - Run after committing

### Why is this needed?

Git only runs scripts in `.git/hooks/`. These scripts act as a "bridge" between Git and Githook, calling your `.ghook` files automatically.

## Directory Structure

A typical Githook setup looks like:

```
my-project/
├── .git/
│   └── hooks/
│       ├── pre-commit       # Shell script
│       ├── commit-msg       # Shell script
│       └── pre-push        # Shell script
├── .githook/
│   ├── pre-commit.ghook    # Your validation rules
│   ├── commit-msg.ghook    # Commit message rules
│   ├── pre-push.ghook      # Pre-push validations
│   └── custom/
│       ├── security.ghook   # Custom snippets
│       └── quality.ghook    # Reusable rules
└── .gitignore              # Add .githook if needed
```

## Configuration Options

### Custom Hook Directory

By default, Githook looks for `.ghook` files in `.githook/`. You can change this:

```bash
# Use a different directory
githook pre-commit ./hooks
```

### Specific Hook Files

You can have multiple hook files:

```
.githook/
├── pre-commit-files.ghook     # File validations
├── pre-commit-quality.ghook   # Code quality
└── pre-commit-security.ghook  # Security checks
```

Githook will run all `.ghook` files that match the hook name.

## Sharing with Your Team

### Commit .githook Directory

Add `.githook/` to version control so your team uses the same rules:

```bash
git add .githook/
git commit -m "Add Githook validations"
```

### Don't Commit .git/hooks/

The `.git/hooks/` directory should NOT be committed. Each developer needs to run:

```bash
mkdir -p .githook
```

This installs the Git hooks locally.

### Add Setup Instructions

Add to your README:

```markdown
## Development Setup

1. Install Githook: https://githook.dev/installation
2. Initialize hooks: `mkdir -p .githook`
3. Done! Hooks will now run automatically.
```

## CI/CD Integration

Run Githook in your CI/CD pipeline:

```yaml
# GitHub Actions
- name: Validate with Githook
  run: |
    githook pre-commit
    githook pre-commit
```

See [CI/CD Integration](../advanced/ci-cd.md) for more details.

## Multiple Repositories

If you have multiple projects, you can:

### Share Common Rules

Create a shared snippets repository:

```bash
# In your snippets repo
.githook-snippets/
├── security.ghook
├── quality.ghook
└── conventions.ghook
```

Import them in your projects:

```javascript
import "../githook-snippets/security.ghook"
import "../githook-snippets/quality.ghook"

# Project-specific rules
foreach file in staged_files matching "*.rs" {
  block_if content contains "panic!" message "Don't use panic! in production"
}
```

### Use Template Repository

Create a template repository with your standard Githook setup, then use it for new projects:

```bash
git clone https://github.com/yourorg/project-template
cd project-template
mkdir -p .githook
```

## Advanced Configuration

### Skip Hooks Temporarily

Sometimes you need to bypass hooks:

```bash
# Skip all hooks for one commit
git commit --no-verify -m "Emergency fix"

# Or use Githook's skip flag
GITHOOK_SKIP=1 git commit -m "Skip validations"
```

### Run Specific Hook Manually

```bash
# Run pre-commit checks without committing
githook pre-commit

# Run on specific files
githook pre-commit
```

### Verbose Output

See detailed execution information:

```bash
githook pre-commit
```

## Hook Types

Githook supports all standard Git hooks:

| Hook | When it Runs | Common Use Cases |
|------|--------------|------------------|
| `pre-commit` | Before commit | File size, syntax, linting |
| `commit-msg` | Before commit message is saved | Enforce format, check length |
| `pre-push` | Before push | Run tests, check branch |
| `pre-rebase` | Before rebase | Prevent rebasing certain branches |
| `post-commit` | After commit | Notifications, automation |

Create any hook file:

```bash
# Create pre-push hook
cat > .githook/pre-push.ghook << 'EOF'
block_if branch_name matches "^main$" message "Cannot push to main directly"
EOF

# Reinstall hooks
mkdir -p .githook
```

## Best Practices

✅ **DO:**
- Commit `.githook/` directory to version control
- Keep rules simple and fast
- Provide clear error messages
- Document your rules
- Use `warn_if` for suggestions, `block_if` for requirements

❌ **DON'T:**
- Commit `.git/hooks/` directory
- Make rules too strict (frustrates developers)
- Run expensive operations (keep hooks fast)
- Hide important validations in complex logic

## Updating Hooks

When you update `.ghook` files, there's nothing to install - they're loaded at runtime!

```bash
# Edit your hook
vim .githook/pre-commit.ghook

# Test it
git add -A
git commit -m "test"

# It automatically uses the new rules!
```

## Next Steps

- [Running Hooks](./running-hooks.md) - Learn how to execute and test hooks
- [Language Guide](../language/README.md) - Master the Githook language
- [Examples](../examples/README.md) - See real-world examples
