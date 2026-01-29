# ⚠️ DOCUMENTATION UPDATE NEEDED

## Critical Finding: CLI Mismatch

The documentation describes a CLI interface that **doesn't match the actual implementation**.

### What the Docs Say:
- `githook init` - Initialize hooks
- `githook run <hook>` - Run a specific hook
- `githook validate` - Validate syntax
- `githook list-snippets` - List snippets

### What Actually Exists:
- `githook <hook-type>` - Run a hook directly (e.g., `githook pre-commit`)
- `githook list` - List packages
- `githook check-update` - Check for updates
- `githook update` - Update binary

## Required Changes

### 1. Installation Section
- ✅ Binary installation is correct
- ✅ Update mechanism exists (`githook check-update`, `githook update`)

### 2. Getting Started
- ❌ `githook init` doesn't exist - **NEEDS REWORK**
- ❌ Hook setup process different than documented

### 3. Running Hooks
- ❌ `githook run pre-commit` doesn't work
- ✅ `githook pre-commit` DOES work
- ❌ Flags like `--verbose`, `--json` need verification

### 4. CLI Reference
- ❌ All command pages need rewriting
- ❌ init.md - Command doesn't exist
- ❌ run.md - Command doesn't exist  
- ❌ validate.md - Command doesn't exist
- ❌ list-snippets.md - Different command (`githook list`)

## Testing Plan Updates

### Phase 1: Understand Actual CLI (IN PROGRESS)
- [x] Build binary
- [x] Test --help output
- [ ] Test each actual command
- [ ] Document actual behavior

### Phase 2: Update Documentation
- [ ] Rewrite Getting Started based on real usage
- [ ] Fix CLI Reference section
- [ ] Update all examples to use correct commands
- [ ] Fix installation verification steps

### Phase 3: Test Updated Docs
- [ ] Follow Getting Started from scratch
- [ ] Verify all code examples
- [ ] Test all commands shown

## Next Steps

1. **Document actual CLI** - Create accurate reference
2. **Test real workflow** - From install to first hook
3. **Rewrite docs** - Match implementation
4. **Verify language features** - Test .ghook syntax
5. **Re-test everything** - End-to-end validation

## Decision Points

**Option A: Update Docs to Match Code**
- Pros: Fast, works now
- Cons: CLI might be incomplete

**Option B: Implement Missing CLI Commands**
- Pros: Better UX
- Cons: More development work

**Recommendation:** **Option A** for launch, then Option B for v0.2
