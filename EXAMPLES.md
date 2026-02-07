# Githook Examples

Every example in this file uses **valid, current syntax** and has been
verified against the Githook parser (v0.0.1, edition 2026-02-07).

> **File extension:** `.ghook`  
> **Script location:** `.githook/<hook-type>.ghook` (e.g. `.githook/pre-commit.ghook`)

---

## Table of Contents

1. [Minimal Pre-Commit Hook](#1-minimal-pre-commit-hook)
2. [Variables and Interpolation](#2-variables-and-interpolation)
3. [Conditionals](#3-conditionals)
4. [Iterating Over Files](#4-iterating-over-files)
5. [Pattern Matching](#5-pattern-matching)
6. [Parallel Commands](#6-parallel-commands)
7. [Groups and Severity](#7-groups-and-severity)
8. [Macros](#8-macros)
9. [Imports](#9-imports)
10. [Packages](#10-packages)
11. [Error Handling](#11-error-handling)
12. [Git Object Model](#12-git-object-model)
13. [File Operations](#13-file-operations)
14. [String and Array Methods](#14-string-and-array-methods)
15. [HTTP Requests](#15-http-requests)
16. [Real-World Pre-Commit Hook](#16-real-world-pre-commit-hook)
17. [Commit Message Hook](#17-commit-message-hook)
18. [Pre-Push Hook](#18-pre-push-hook)

---

## 1. Minimal Pre-Commit Hook

The simplest hook — run a single command and block the commit on failure.

```ghook
# .githook/pre-commit.ghook
run "npm test"
```

Run multiple commands sequentially:

```ghook
run "cargo fmt --check"
run "cargo clippy -- -D warnings"
run "cargo test"
```

---

## 2. Variables and Interpolation

Use `let` to define variables. String interpolation uses `${expr}`.

```ghook
let name = "world"
print "Hello, ${name}!"

let count = 42
let half = count / 2
print "Half of ${count} is ${half}"

let items = [1, 2, 3]
print items.length

let greeting = "Hello"
print greeting.upper
```

Supported value types: strings, numbers, booleans (`true` / `false`), `null`, and arrays.

---

## 3. Conditionals

### Simple if / else

```ghook
let files = git.files.staged
let count = files.length

if count == 0 {
    print "No staged files, skipping checks"
}

if count > 100 {
    warn "Large commit: ${count} files staged"
} else {
    print "Commit size looks good"
}
```

### block if / warn if

These are shorthand conditional checks that block or warn in one line:

```ghook
# Block the commit if the condition is true
block if git.stats.additions > 1000 message "Too many additions"

# Emit a warning (non-blocking) if condition is true
warn if git.branch.name == "main" message "Committing directly to main"
```

### Logical operators

```ghook
if git.branch.is_main and git.stats.additions > 500 {
    block "Large direct commit to main is not allowed"
}

if count == 0 or count > 1000 {
    warn "Suspicious file count"
}
```

### Inline conditionals (ternary)

Use `if ... then ... else` as an expression anywhere a value is expected:

```ghook
let label = if count > 0 then "has files" else "empty"
print if git.branch.is_main then "MAIN" else git.branch.name

# Nestable for multi-way branching
let severity = if errors > 10 then "critical" else if errors > 0 then "warning" else "ok"
```

---

## 4. Iterating Over Files

The `foreach` statement iterates over a collection. The loop variable
is declared *inside* the braces with `var in`.

```ghook
# Iterate all staged files
foreach git.files.staged {
    file in
    print file.name
}
```

### Filtering with `matching`

```ghook
# Only process Rust files
foreach git.files.staged matching "*.rs" {
    file in
    print "Checking ${file.name}"
}
```

### Using closures: filter, map, find, any, all

```ghook
let staged = git.files.staged

# Filter to just TypeScript files
let ts_files = staged.filter(f => f.name.ends_with(".ts"))
print ts_files.length

# Check if any file is larger than 1MB
let has_large = staged.any(f => f.size > 1048576)
if has_large {
    warn "Some staged files are over 1MB"
}

# Check all files have an extension
let all_have_ext = staged.all(f => f.extension != "")
```

---

## 5. Pattern Matching

`match` evaluates a subject against a list of patterns. Patterns can be
literal values, wildcard strings (globs), or the catch-all `_`.

```ghook
match git.branch.name {
    "main" -> {
        block "Direct commits to main are not allowed"
    }
    "develop" -> {
        warn "Committing to develop"
    }
    "feature/*" -> {
        print "Feature branch commit"
    }
    _ -> {
        print "Branch: ${git.branch.name}"
    }
}
```

Match on file extensions:

```ghook
foreach git.files.staged {
    file in
    match file.extension {
        "rs" -> run "cargo clippy"
        "ts" -> run "eslint --quiet ."
        "py" -> run "ruff check ."
        _ -> print "Skipping ${file.name}"
    }
}
```

---

## 6. Parallel Commands

Run multiple commands concurrently using `parallel`:

```ghook
parallel {
    run "cargo test"
    run "cargo clippy -- -D warnings"
    run "cargo fmt --check"
}
```

All commands inside a `parallel` block run on separate threads. If any
command fails, the commit is blocked.

---

## 7. Groups and Severity

Groups organize related checks with a name and severity level.
Severity can be `critical` (default), `warning`, or `info`.

```ghook
group formatting critical {
    run "cargo fmt --check"
}

group linting warning {
    run "cargo clippy -- -D warnings"
}

group docs info {
    print "Remember to update documentation"
}
```

### Disabled groups

```ghook
group slow_tests critical disabled {
    run "cargo test -- --ignored"
}
```

---

## 8. Macros

Define reusable blocks of logic with `macro` and call them with `@`.

### Simple macro

```ghook
macro greet {
    print "Running pre-commit checks..."
}

@greet()
```

### Macro with parameters

```ghook
macro check_format(command, label) {
    print "Checking ${label}..."
    run command
}

@check_format("cargo fmt --check", "Rust formatting")
@check_format("prettier --check .", "Prettier formatting")
```

---

## 9. Imports

Import macros and logic from other `.ghook` files:

```ghook
# Import relative to .githook/ directory
import "helpers.ghook"

# Import with alias
import "shared/lint.ghook" as lint
@lint.check_all()
```

The imported file's macros become available under the alias namespace.

---

## 10. Packages

Use remote packages from a registry (GitHub-based):

```ghook
# Load a package — macros are namespaced
use "@preview/quality"
@quality.check_formatting()

# Alias the namespace
use "@preview/security" as sec
@sec.scan_secrets()
```

---

## 11. Error Handling

Use `try` / `catch` to handle errors gracefully:

```ghook
try {
    run "faulty command"
} catch { error in
    print "Tests failed: ${error}"
    warn "Test suite is broken but allowing commit"
}
```

### Using `block` and `warn` statements

```ghook
# Unconditionally emit a warning
warn "Don't forget to run the full test suite"

# Unconditionally block the commit
block "This repository is read-only"
```

### Allow (informational)

```ghook
allow "binary files"
```

---

## 12. Git Object Model

The `git` object is the main entry point for repository information.

### Branch

```ghook
print git.branch.name
print git.branch.is_main

if git.branch.is_main {
    block "Direct commits to main not allowed"
}
```

### Author

```ghook
print git.author.name
print git.author.email
```

### Remote

```ghook
print git.remote.name
print git.remote.url
```

### Stats (diff statistics)

```ghook
print git.stats.files_changed
print git.stats.additions
print git.stats.deletions
print git.stats.modified_lines
```

### Files

```ghook
# All file collections return arrays of File objects
let staged = git.files.staged
let modified = git.files.modified
let added = git.files.added
let deleted = git.files.deleted
let unstaged = git.files.unstaged
let all = git.files.all

print "Staged: ${staged.length}"
print "Modified: ${modified.length}"
```

### Diff

```ghook
let added_lines = git.diff.added_lines
let removed_lines = git.diff.removed_lines

print "Lines added: ${added_lines.length}"
```

### Merge

```ghook
print git.merge.source
print git.merge.target
```

### Commit

```ghook
if git.commit != null {
    print git.commit.message
    print git.commit.hash
}
```

---

## 13. File Operations

Every element in `git.files.staged` (and similar) is a **File object**
with rich properties and methods.

### File properties

```ghook
foreach git.files.staged { file in
    print file.name
    print file.basename
    print file.extension
    print file.dirname
    print file.content
    print file.diff
    print file.size
}
```

### File methods

```ghook
foreach git.files.all { f in
    if f.exists() {
        print "${f.name} exists"
    }
    if f.is_file() {
        print "${f.name} is a regular file"
    }
    if f.contains(".env") {
        warn "File path contains .env"
    }
    if f.ends_with(".lock") {
        print "Lock file: ${f.name}"
    }
}
```

### Built-in functions

```ghook
# Create a file object from a path
let f = file("src/main.rs")
print f.name

# List directory contents
let entries = dir("src")

# Glob for files
let rust_files = glob("src/**/*.rs")

# Execute a command and capture output (30s timeout)
let result = exec("date")
print result

# Remove a file
let removed = rm("temp.txt")
```

---

## 14. String and Array Methods

### String properties & methods

```ghook
let s = "Hello, World!"

# Properties (no parentheses)
print s.length
print s.upper
print s.lower

# Methods (with parentheses)
print s.trim()
print s.reverse()
print s.replace("World", "Githook")
print s.contains("Hello")
print s.starts_with("Hello")
print s.ends_with("!")
print s.matches("[A-Z][a-z]+")
print s.is_empty()

let parts = s.split(", ")

# Substring extraction (negative indices count from end)
print s.slice(0, 5)     # "Hello"
print s.slice(7, 12)    # "World"
print s.slice(-6, -1)   # "World"
```

### Number methods

```ghook
let n = -3.7

print n.abs()
print n.floor()
print n.ceil()
print n.round()
print n.sqrt()
print n.pow(2)
```

### Array properties & methods

```ghook
let items = [1, 2, 3, 4, 5]

# Methods
print items.length
print items.first()
print items.last()
print items.is_empty()
print items.sum()

# Closure methods
let evens = items.filter(x => x % 2 == 0)
let doubled = items.map(x => x * 2)
let found = items.find(x => x > 3)
let has_big = items.any(x => x > 4)
let all_pos = items.all(x => x > 0)
```

---

## 15. HTTP Requests

Make HTTP requests using the `http` object:

```ghook
let response = http.get("https://api.restful-api.dev/objects")

let items = response.json

# First Object: Google Pixel 6 Pro
let first = items[0]
print "First Device: " + first["name"]
print "Color: " + first["data"]["color"]
print "Capacity: " + first["data"]["capacity"]

# Third Object: Apple iPhone 12 Pro Max
let third = items[2]
print "Third Device: " + third["name"]

# Price of iPhone 11
let iphone11 = items[3]
print iphone11["name"] + " costs $" + iphone11["data"]["price"]

# MacBook Pro Details
let macbook = items[6]
print macbook["name"] + " (" + macbook["data"]["year"] + ") - $" + macbook["data"]["price"]
```
---

## Syntax Quick Reference

| Construct | Syntax |
|---|---|
| Run command | `run "command"` |
| Print value | `print expr` |
| Variable | `let name = expr` |
| If / else | `if expr { ... } else { ... }` |
| Ternary | `if expr then expr else expr` |
| Foreach | `foreach collection matching "pattern" { var in ... }` |
| Match | `match expr { pattern -> statement }` |
| Parallel | `parallel { run "a"  run "b" }` |
| Group | `group name severity { ... }` |
| Macro def | `macro name(params) { ... }` |
| Macro call | `@name(args)` or `@ns.name(args)` |
| Import | `import "path.ghook" as alias` |
| Package | `use "@ns/name" as alias` |
| Try/catch | `try { ... } catch var { ... }` |
| Block | `block "message"` or `block if expr message "msg"` |
| Warn | `warn "message"` or `warn if expr message "msg"` |
| Allow | `allow "description"` |
| Break | `break` |
| Continue | `continue` |
| Comment | `# single-line comment` |
| Interpolation | `"text ${expr} text"` |
| Closure | `param => expr` |
| String slice | `str.slice(start, end)` |
| Comparison | `==  !=  <  <=  >  >=` |
| Logical | `and  or  not` |
| Arithmetic | `+  -  *  /  %` |
| Array | `[1, 2, 3]` |
| Boolean | `true  false` |
| Null | `null` |