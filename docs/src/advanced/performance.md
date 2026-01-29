# Performance Optimization

Make your hooks run faster.

## Best Practices

### 1. Limit File Patterns

```javascript
# ❌ Slow - checks every file
foreach file in staged_files matching "*" {
  block_if content matches "complex_regex" message "Found"
}

# ✅ Fast - only relevant files
foreach file in staged_files matching "*.{rs,js,ts}" {
  block_if content matches "complex_regex" message "Found"
}
```

### 2. Use where Clause

```javascript
# Filter before expensive operations
foreach file in staged_files matching "*" where file_size > 1000 {
  block_if content contains "pattern" message "Found"
}
```

### 3. Early Returns

```javascript
foreach file in staged_files matching "*" {
  continue_if file_size < 100
  # Expensive check only for larger files
  block_if content matches "pattern" message "Found"
}
```

### 4. Avoid Complex Regex

```javascript
# ❌ Slow
block_if content matches "^.*password.*$"

# ✅ Fast
block_if content contains "password"
```

## Profiling

```bash
time githook pre-commit
```
