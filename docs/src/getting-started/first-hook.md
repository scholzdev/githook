# Your First Hook

Let's create your first `.ghook` file! We'll start with a simple but useful validation: preventing large files from being committed.

## Step 1: Create a Git Repository

If you don't have one already:

```bash
mkdir my-project
cd my-project
git init
```

## Step 2: Create Hook Directory

```bash
mkdir -p .githook
```

This creates the `.githook/` directory where your hook files will live.

## Step 3: Edit pre-commit.ghook

Open `.githook/pre-commit.ghook` in your editor and replace its contents with:

```javascript
# Prevent large files from being committed
foreach file in staged_files matching "*" {
  block_if file_size > 1048576 message "File {file} is too large (max 1MB)"
}
```

Let's break this down:

- `foreach file in staged_files matching "*"` - Loop through all staged files
- `block_if` - Stop the commit if the condition is true
- `file_size > 1048576` - Check if file is larger than 1MB (1,048,576 bytes)
- `message "..."` - Show this error message
- `{file}` - Variable interpolation

## Step 4: Test Your Hook

Create a small file:

```bash
echo "Hello, world!" > small.txt
git add small.txt
git commit -m "Add small file"
```

✅ This should succeed!

Now create a large file:

```bash
dd if=/dev/zero of=large.txt bs=1M count=2
git add large.txt
githook pre-commit
```

❌ This should fail with:
```
  x File large.txt is too large (max 1MB)
✗ Hook blocked!
```

## Step 5: Add More Rules

Let's add more validations to the same file:

```javascript
# Prevent large files
foreach file in staged_files matching "*" {
  block_if file_size > 1048576 message "File {file} is too large (max 1MB)"
}

# Prevent committing TODO comments
foreach file in staged_files matching "*.{js,ts,py,rs}" {
  warn_if content contains "TODO" message "TODO found in {file}"
}

# Block console.log in JavaScript files
foreach file in staged_files matching "*.{js,ts}" {
  block_if content contains "console.log" message "Remove console.log from {file}"
}
```

Notice we used `warn_if` instead of `block_if` for TODOs - this shows a warning but doesn't block the commit.

## Step 6: Test Warning vs. Blocking

Create a file with a TODO:

```bash
echo "# TODO: finish this" > todo.py
git add todo.py
git commit -m "Add todo file"
```

You'll see a ⚠️ warning but the commit succeeds.

Create a file with console.log:

```bash
echo "console.log('debug')" > test.js
git add test.js
git commit -m "Add debug code"
```

This will ❌ block the commit.

## Understanding the Output

When Githook runs, you'll see:

```
Running pre-commit.ghook...
  ✓ small.txt - passed all checks
  ⚠ todo.py - TODO found in todo.py
  ✗ test.js - Remove console.log from test.js
```

- ✓ = passed
- ⚠ = warning (doesn't block)
- ✗ = error (blocks commit)

## What You've Learned

✅ How to initialize Githook  
✅ The basic syntax of `.ghook` files  
✅ The difference between `block_if` and `warn_if`  
✅ How to loop through staged files  
✅ How to check file properties  

## Next Steps

- [Repository Setup](./repository-setup.md) - Configure Githook for your project
- [Running Hooks](./running-hooks.md) - Different ways to execute hooks
- [Language Guide](../language/README.md) - Learn all language features
