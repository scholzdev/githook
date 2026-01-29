# Git Object

## Branch

```ghook
let branch = git.branch.name
```

## Commit

```ghook
let message = git.commit.message
let hash = git.commit.hash
```

## Author

```ghook
let name = git.author.name
let email = git.author.email
```

## Remote

```ghook
let remote = git.remote.name
let url = git.remote.url
```

## Statistics

```ghook
let files = git.stats.files_changed
let additions = git.stats.additions
let deletions = git.stats.deletions
```

## Files

```ghook
let staged = git.staged_files
let all = git.all_files
```

## Status

```ghook
let is_merge = git.is_merge_commit
let has_conflicts = git.has_conflicts
```
