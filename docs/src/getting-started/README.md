# Quick Start

Get up and running with Githook in under 5 minutes! This guide will walk you through creating your first Git hook.

## Prerequisites

Make sure you have Githook installed. If not, see the [Installation Guide](../installation/README.md).

## What You'll Learn

In this guide, you'll:
1. ✅ Initialize Githook in a Git repository
2. ✅ Create your first `.ghook` file
3. ✅ Test your hook
4. ✅ Understand the basics of the Githook language

## Quick Steps

1. **[Your First Hook](./first-hook.md)** - Create a simple validation rule
2. **[Repository Setup](./repository-setup.md)** - Initialize Githook in your project
3. **[Running Hooks](./running-hooks.md)** - Execute and test your hooks
4. **[Troubleshooting](./troubleshooting.md)** - Fix common issues

## 30-Second Quick Start

If you just want to try it out right now:

```bash
# Create a test repo
mkdir test-repo && cd test-repo
git init

# Initialize githook
mkdir -p .githook

# Create a simple hook
cat > .githook/pre-commit.ghook << 'EOF'
foreach file in staged_files matching "*" {
  block_if file_size > 1048576 message "File {file} is too large (max 1MB)"
}
EOF

# Test it
echo "test" > test.txt
git add test.txt
git commit -m "test"  # Should succeed

# Create a large file
dd if=/dev/zero of=large.txt bs=1M count=2
git add large.txt
git commit -m "large file"  # Should fail!
```

Ready? Let's start with [Your First Hook](./first-hook.md)!
