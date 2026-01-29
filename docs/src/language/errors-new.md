# Error Handling

Use `try` and `catch` to handle errors:

```ghook
try {
    let value = some_operation()
} catch error {
    run "echo Error: ${error}"
}
```

The error variable is optional:

```ghook
try {
    # code that might fail
} catch {
    # recovery code
}
```
