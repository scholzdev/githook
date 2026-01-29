# Branch Protection

Prevent commits to protected branches:

```ghook
block if git.branch.name == "main"
    message "Cannot commit directly to main"
```

Multiple branches:

```ghook
let protected = ["main", "master", "production"]
let is_protected = protected.any(b => b == git.branch.name)

block if is_protected
    message "Protected branch"
```

Different rules per branch:

```ghook
match git.branch.name {
    "main" -> {
        block "No direct commits to main"
    }
    "develop" -> {
        warn "Be careful with develop"
    }
    _ -> {
        run "echo Branch OK"
    }
}
```
