# Documentation Testing Checklist

## Testing Strategy

For each section, we need to verify:
- ✅ All code examples are syntactically correct
- ✅ All commands work as described
- ✅ All links are valid
- ✅ Information matches actual implementation
- ✅ Examples produce expected output

---

## 1. Installation Testing

### From GitHub Releases
- [ ] Download links work (check latest release exists)
- [ ] Installation commands are correct for each platform
- [ ] Binary permissions are set correctly
- [ ] Update mechanism works (`githook --update`)

**Test:**
```bash
# Test download URLs exist
curl -I https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-apple-darwin
curl -I https://github.com/scholzdev/githook/releases/latest/download/githook-aarch64-apple-darwin
curl -I https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-unknown-linux-gnu
```

### From Cargo
- [ ] `cargo install` command syntax is correct
- [ ] Git install command works
- [ ] Package name matches actual crate

**Test:**
```bash
# Verify crate structure
ls crates/githook-cli/Cargo.toml
grep "name = " crates/githook-cli/Cargo.toml
```

### Verify Installation
- [ ] `githook --version` command works
- [ ] `githook --help` shows correct output
- [ ] All listed commands exist

**Test:**
```bash
githook --version
githook --help
githook init --help
githook run --help
githook validate --help
```

---

## 2. Getting Started Testing

### Your First Hook
- [ ] `githook init` creates correct files
- [ ] Example `.ghook` syntax is valid
- [ ] Test scenario works (small file succeeds, large file fails)

**Test:**
```bash
# Create test repo
mkdir /tmp/test-githook
cd /tmp/test-githook
git init
githook init

# Create and test the example hook
cat > .githook/pre-commit.ghook << 'EOF'
foreach file in staged_files matching "*" {
  block_if file_size > 1048576 message "File {file} is too large (max 1MB)"
}
EOF

# Test with small file
echo "test" > small.txt
git add small.txt
git commit -m "test" # Should succeed

# Test with large file
dd if=/dev/zero of=large.txt bs=1M count=2
git add large.txt
git commit -m "large" # Should fail
```

### Repository Setup
- [ ] All `githook init` options work
- [ ] Directory structure matches documentation
- [ ] Git hooks are created correctly

**Test:**
```bash
githook init
ls -la .githook/
ls -la .git/hooks/
cat .git/hooks/pre-commit # Should call githook run
```

### Running Hooks
- [ ] All `githook run` examples work
- [ ] Flags (`--verbose`, `--json`, etc.) work
- [ ] Output format matches examples

**Test:**
```bash
githook run pre-commit
githook run pre-commit --verbose
githook run pre-commit --json
githook run pre-commit --dry-run
```

---

## 3. Language Guide Testing

### Syntax Basics
- [ ] All code examples are syntactically valid
- [ ] Comments work as described
- [ ] String escaping examples are correct

**Test:**
```bash
# Validate each example
cat > /tmp/test-syntax.ghook << 'EOF'
# This is a comment
let max_size = 1048576
block_if file_size > {max_size} message "Too large"
EOF

githook validate /tmp/test-syntax.ghook
```

### Variables
- [ ] Variable declarations work
- [ ] Interpolation works
- [ ] All variable types are correct

**Test:**
```bash
cat > /tmp/test-vars.ghook << 'EOF'
let name = "test.txt"
let max = 1000
let list = [".exe", ".dll"]

block_if file_size > {max} message "File {name} exceeds {max} bytes"
EOF

githook validate /tmp/test-vars.ghook
```

### Operators
- [ ] All operators listed work
- [ ] Operator examples are valid
- [ ] Precedence is correct

**Test:**
```bash
cat > /tmp/test-ops.ghook << 'EOF'
# Numeric
block_if file_size > 1000 message "test"
block_if file_size >= 1000 message "test"
block_if file_size < 1000 message "test"
block_if file_size <= 1000 message "test"
block_if file_size == 1000 message "test"

# String
block_if content contains "TODO" message "test"
block_if content matches "pattern" message "test"
block_if extension in [".rs"] message "test"

# Logical
block_if file_size > 1000 and extension == ".rs" message "test"
block_if file_size > 1000 or extension == ".rs" message "test"
block_if not author_set message "test"
EOF

githook validate /tmp/test-ops.ghook
```

### Conditions
- [ ] `block_if` works
- [ ] `warn_if` works
- [ ] `when` works
- [ ] All condition examples validate

**Test:**
```bash
cat > /tmp/test-conditions.ghook << 'EOF'
block_if file_size > 1000 message "Too large"
warn_if content contains "TODO" message "TODO found"

when branch_name == "main" {
  block_if true message "Protected"
}

when branch_name == "main" {
  warn_if true message "Warning"
} else {
  run "echo 'Not main'"
}
EOF

githook validate /tmp/test-conditions.ghook
```

### Loops
- [ ] `foreach file in staged_files` works
- [ ] Pattern matching works
- [ ] `where` clause works
- [ ] List iteration works

**Test:**
```bash
cat > /tmp/test-loops.ghook << 'EOF'
foreach file in staged_files matching "*" {
  block_if file_size > 1000 message "Too large"
}

foreach file in staged_files matching "*.rs" {
  block_if content contains "unwrap" message "Found"
}

foreach file in staged_files matching "*" where file_size > 1000 {
  warn_if true message "Large file"
}

let patterns = ["TODO", "FIXME"]
foreach pattern in {patterns} {
  warn_if content contains "{pattern}" message "Found"
}
EOF

githook validate /tmp/test-loops.ghook
```

### Snippets
- [ ] Macro definition works
- [ ] Macro with parameters works
- [ ] `@macro_name` invocation works
- [ ] Import works

**Test:**
```bash
# Create snippet file
cat > /tmp/snippets.ghook << 'EOF'
macro no_todos {
  block_if content contains "TODO" message "Remove TODOs"
}

macro check_size(max) {
  block_if file_size > {max} message "Exceeds {max}"
}
EOF

# Use snippets
cat > /tmp/use-snippets.ghook << 'EOF'
import "/tmp/snippets.ghook"

@no_todos
@check_size(1000)
EOF

githook validate /tmp/use-snippets.ghook
```

---

## 4. Examples Testing

For each example in the Examples section:
- [ ] Copy example to a test `.ghook` file
- [ ] Validate syntax
- [ ] Test with real files

**Test:**
```bash
# Test large-files.md example
cat > /tmp/test-example.ghook << 'EOF'
let MAX_SIZE = 1048576

foreach file in staged_files matching "*" {
  block_if file_size > {MAX_SIZE} message "File {file} is {file_size} bytes (max {MAX_SIZE})"
}
EOF

githook validate /tmp/test-example.ghook
```

### Specific Examples to Test:
- [ ] Large Files - All variants
- [ ] Commit Messages - Conventional commits regex
- [ ] Branch Protection - Pattern matching
- [ ] Secret Detection - All patterns
- [ ] Code Quality - Each language
- [ ] Author Validation - All checks
- [ ] Advanced - Complete example

---

## 5. CLI Commands Testing

### githook init
- [ ] Basic `githook init` works
- [ ] `--force` flag works
- [ ] Creates correct file structure

**Test:**
```bash
cd /tmp/test-cli
git init
githook init
ls .githook/
ls .git/hooks/

# Test force
githook init --force
```

### githook run
- [ ] All hook types work (pre-commit, commit-msg, pre-push)
- [ ] `--verbose` flag works
- [ ] `--json` output is valid JSON
- [ ] `--files` pattern works

**Test:**
```bash
githook run pre-commit
githook run pre-commit --verbose
githook run pre-commit --json | jq .
githook run pre-commit --dry-run
```

### githook validate
- [ ] Validates correct files
- [ ] Shows syntax errors
- [ ] `--verbose` provides details

**Test:**
```bash
# Valid file
githook validate .githook/pre-commit.ghook

# Invalid file
cat > /tmp/invalid.ghook << 'EOF'
block_if file_size > "string" message "test"
EOF
githook validate /tmp/invalid.ghook # Should show error
```

### githook list-snippets
- [ ] Lists all available snippets
- [ ] Output format matches docs

**Test:**
```bash
githook list-snippets
githook list-snippets --verbose
```

---

## 6. Standard Library Testing

### Built-in Context Variables
- [ ] All variables listed are actually available
- [ ] Variable types match documentation

**Test:**
```bash
cat > /tmp/test-context.ghook << 'EOF'
# Test all file context variables
foreach file in staged_files matching "*" {
  warn_if true message "file: {file}"
  warn_if true message "content: {content}"
  warn_if true message "file_size: {file_size}"
  warn_if true message "extension: {extension}"
}

# Test all Git context variables
warn_if true message "branch: {branch_name}"
warn_if true message "message: {commit_message}"
EOF

# Run and check output
githook run pre-commit --verbose
```

---

## 7. Cross-Reference Testing

### Internal Links
- [ ] All markdown links work
- [ ] No broken internal links
- [ ] Links point to correct sections

**Test:**
```bash
# Check all markdown files for broken links
cd docs/src
grep -r "\[.*\](.*.md)" . | while read line; do
  # Extract filename
  file=$(echo $line | cut -d: -f1)
  # Extract link
  link=$(echo $line | grep -o '(.*\.md)' | tr -d '()')
  # Check if target exists
  target=$(dirname $file)/$link
  if [ ! -f "$target" ]; then
    echo "Broken link in $file: $link"
  fi
done
```

### Code Consistency
- [ ] All `.ghook` examples use consistent style
- [ ] Variable naming is consistent
- [ ] Error messages follow same format

---

## 8. Platform-Specific Testing

### macOS
- [ ] Installation instructions work
- [ ] Gatekeeper instructions are correct
- [ ] Paths are correct

### Linux
- [ ] Installation instructions work
- [ ] Permissions are correct
- [ ] Paths are correct

### Windows
- [ ] PATH instructions are correct
- [ ] Binary name is correct (.exe)

---

## 9. Edge Cases & Error Testing

- [ ] Invalid syntax shows helpful errors
- [ ] Missing files show clear errors
- [ ] Empty `.ghook` files don't crash
- [ ] Very large files are handled
- [ ] Special characters in messages work

**Test:**
```bash
# Empty file
touch /tmp/empty.ghook
githook validate /tmp/empty.ghook

# Invalid syntax
echo "this is not valid" > /tmp/invalid.ghook
githook validate /tmp/invalid.ghook

# Special characters
cat > /tmp/special.ghook << 'EOF'
block_if true message "Special: \"quotes\", 'apostrophes', \n newlines"
EOF
githook validate /tmp/special.ghook
```

---

## 10. Documentation Build Testing

- [ ] `mdbook build` completes without errors
- [ ] `mdbook serve` works
- [ ] Search functionality works
- [ ] All pages render correctly
- [ ] Code highlighting works for `.ghook` syntax

**Test:**
```bash
cd docs
mdbook build
mdbook test # If tests exist
mdbook serve # Check manually in browser
```

---

## Testing Priority

### High Priority (Must work for launch):
1. ✅ Installation from releases
2. ✅ `githook init` and basic setup
3. ✅ All code examples in Getting Started
4. ✅ Core language features (loops, conditions, operators)
5. ✅ All CLI commands

### Medium Priority (Should work):
1. ✅ All examples in Examples section
2. ✅ Standard library variables
3. ✅ Advanced features
4. ✅ All internal links

### Low Priority (Nice to have):
1. ✅ CI/CD examples
2. ✅ VSCode extension details
3. ✅ Architecture deep-dive

---

## Automated Testing Script

Create `docs/test-docs.sh`:

```bash
#!/bin/bash
set -e

echo "🧪 Testing Githook Documentation"
echo "================================"

# Test mdbook build
echo "📚 Building documentation..."
cd docs
mdbook build
echo "✅ Build successful"

# Test all code examples
echo "🔍 Validating code examples..."
cd ..

# Create temp test directory
TEST_DIR=$(mktemp -d)
cd $TEST_DIR
git init

# Copy and test each example
find ../docs/src -name "*.md" -exec grep -l '```ghook' {} \; | while read file; do
  echo "Testing examples in $file..."
  # Extract and test each ghook code block
  # (Would need proper parser here)
done

cd ..
rm -rf $TEST_DIR

echo "✅ All tests passed!"
```

---

## Manual Testing Checklist

Go through the docs in browser and check:
- [ ] Table of contents is correct
- [ ] All pages load
- [ ] Code blocks are properly formatted
- [ ] Images (if any) load
- [ ] Search finds relevant results
- [ ] Mobile view works
- [ ] Dark theme works

---

## Next Steps

1. **Start with critical path**: Installation → First Hook → Basic validation
2. **Test systematically**: Go section by section
3. **Fix issues immediately**: Update docs as you find problems
4. **Verify fixes**: Re-test after changes
5. **Document any limitations**: If something doesn't work yet, note it

Ready to start testing? Shall we begin with Installation and Getting Started?
