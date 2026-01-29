# Standard Library

Built-in snippets and context variables provided by Githook.

## Overview

The standard library provides:
- **Built-in snippets** - Reusable validation macros
- **Context variables** - Git and file information
- **Helper functions** - Common operations

## Built-in Snippets

Import from standard library:

```javascript
@no_large_files
@conventional_commits
@branch_protection
@no_secrets
```

See individual pages for details:
- [git.ghook](./git.md) - Git operations and context
- [time.ghook](./time.md) - Time-based validations
- [quality.ghook](./quality.md) - Code quality checks
- [security.ghook](./security.md) - Security validations

## Context Variables

### File Context

Available inside `foreach file` loops:

| Variable | Type | Description |
|----------|------|-------------|
| `file` | String | Full file path |
| `content` | String | File content |
| `diff` | String | Staged changes |
| `file_size` | Number | Size in bytes |
| `extension` | String | File extension (.rs) |
| `filename` | String | Name with extension |
| `basename` | String | Name without extension |
| `dirname` | String | Directory path |

### Git Context

Available globally:

| Variable | Type | Description |
|----------|------|-------------|
| `branch_name` | String | Current branch |
| `commit_message` | String | Commit message |
| `modified_lines` | Number | Total lines changed |
| `files_changed` | Number | Number of files |
| `additions` | Number | Lines added |
| `deletions` | Number | Lines deleted |
| `commits_ahead` | Number | Commits ahead of remote |

### Author Context

| Variable | Type | Description |
|----------|------|-------------|
| `author_set` | Boolean | Git user.name set |
| `author_email_set` | Boolean | Git user.email set |
| `author_missing` | Boolean | Author not configured |

## Environment Variables

Access environment with `{env:VAR}`:

```javascript
block_if {env:CI} == "true" message "Running in CI"
block_if {env:GITHUB_ACTIONS} == "true" message "GitHub Actions detected"
```

## Next Steps

Explore each standard library module for detailed documentation and examples.
