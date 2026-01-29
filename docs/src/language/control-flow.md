# Control Flow

## if Statements

```ghook
if count > 10 {
    run "echo Large count"
}
```

With `else`:

```ghook
if count > 10 {
    run "echo Large"
} else {
    run "echo Small"
}
```

## foreach Loops

Iterate over arrays:

```ghook
let numbers = [1, 2, 3]

foreach numbers {
    num in
    run "echo ${num}"
}
```

## match Statements

Pattern matching:

```ghook
let severity = "normal"

match git.branch.name {
    "main" -> {
        let severity = "critical"
    }
    "develop" -> {
        let severity = "important"
    }
    _ -> {
        let severity = "normal"
    }
}
```

## block and warn

Prevent commits:

```ghook
block if git.branch.name == "main"
    message "Cannot commit to main"
```

Show warnings:

```ghook
warn if git.stats.files_changed > 20
    message "Large commit"
```
