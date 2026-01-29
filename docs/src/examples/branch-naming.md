# Branch Naming

Enforce branch naming conventions with regex patterns.

## Simple Pattern

```ghook
let branch = git.branch.name

if not branch.starts_with("feature/") and 
   not branch.starts_with("bugfix/") and 
   not branch.starts_with("hotfix/") {
    block "Branch must start with: feature/, bugfix/, or hotfix/"
}
```

## Regex Pattern

```ghook
let branch = git.branch.name
let pattern = "^(dev|release)-([0-9]+)-q([0-9]+)\\.([0-9]+)\\.(.+)$"

if not branch.matches(pattern) {
    block "Branch name invalid. Use: (dev|release)-YYYY-qX.X.X"
}
```

## Multiple Patterns

```ghook
let valid = ["feature/", "bugfix/", "hotfix/", "release/"]
let has_prefix = valid.any(p => git.branch.name.starts_with(p))

let is_special = git.branch.name == "main" or git.branch.name == "develop"

if not has_prefix and not is_special {
    block "Invalid branch name"
}
```
