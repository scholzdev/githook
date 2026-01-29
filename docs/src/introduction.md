# Introduction

Welcome to **Githook** - a powerful, expressive domain-specific language (DSL) for creating flexible and reusable Git hooks.

## What is Githook?

Githook lets you write declarative `.ghook` files instead of complex shell scripts for pre-commit, commit-msg, pre-push, and other Git hooks. Define validations and checks in a readable, maintainable format that's easy to understand and share across teams.

## Why Githook?

**Before Githook** (traditional shell scripts):
```bash
#!/bin/bash
for file in $(git diff --cached --name-only); do
  size=$(wc -c < "$file")
  if [ $size -gt 1048576 ]; then
    echo "Error: $file is too large"
    exit 1
  fi
done
```

**With Githook**:
```javascript
foreach file in staged_files matching "*" {
  block_if file_size > 1048576 message "File {file} is too large"
}
```

## Key Benefits

✅ **Declarative** - Express what to validate, not how to validate it  
✅ **Readable** - Anyone can understand your hooks at a glance  
✅ **Reusable** - Share snippets and patterns across projects  
✅ **Type-safe** - No more bash quoting nightmares  
✅ **Fast** - Built in Rust for maximum performance  
✅ **Extensible** - Rich standard library with Git integration  

## Features at a Glance

- 🎯 File pattern matching with glob support
- 📊 Rich Git context (file size, diffs, branches, commits)
- 🔒 Built-in security and quality checks
- 🔄 Reusable snippet system
- ⚡ Fast execution with caching
- 🎨 VSCode extension with syntax highlighting and IntelliSense
- 🔧 Auto-update system

## Quick Example

Here's a complete `.ghook` file that enforces common best practices:

```javascript
# Prevent large files
foreach file in staged_files matching "*" {
  block_if file_size > 5000000 message "File too large: {file}"
}

# Enforce commit message format
block_if commit_message not matches "^(feat|fix|docs|style|refactor|test|chore):" 
  message "Commit message must follow conventional commits format"

# Block direct commits to main
block_if branch_name matches "^(main|master)$" 
  message "Direct commits to main branch are not allowed"

# Warn about TODOs
foreach file in staged_files matching "*.{rs,ts,js,py}" {
  warn_if content contains "TODO" 
    message "TODO found in {file}"
}
```

## Getting Started

Ready to dive in? Head over to the [Installation](./installation/README.md) guide to get Githook installed, then follow the [Quick Start](./getting-started/README.md) to create your first hook!

## Community and Support

- 📖 [GitHub Repository](https://github.com/scholzdev/githook)
- 🐛 [Issue Tracker](https://github.com/scholzdev/githook/issues)
- 💬 [Discussions](https://github.com/scholzdev/githook/discussions)

---

Built with ❤️ by [scholzdev](https://florianscholz.dev)
