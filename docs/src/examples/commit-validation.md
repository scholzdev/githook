# Commit Validation

Check message length:

```ghook
let message = git.commit.message

block if message.length < 10
    message "Commit message too short"
```

Check message format:

```ghook
let valid = ["feat:", "fix:", "docs:", "chore:"]
let has_prefix = valid.any(p => message.starts_with(p))

block if not has_prefix
    message "Commit must start with: feat, fix, docs, or chore"
```

Check commit size:

```ghook
let total = git.stats.additions + git.stats.deletions

warn if total > 500
    message "Large commit"

block if total > 1000
    message "Commit too large"
```

Check author email:

```ghook
block if not git.author.email.ends_with("@company.com")
    message "Use company email"
```
