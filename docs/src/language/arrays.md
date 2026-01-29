# Arrays

Arrays hold multiple values:

```ghook
let numbers = [1, 2, 3, 4, 5]
```

Arrays can hold mixed types:

```ghook
let mixed = ["text", 42, true]
```

## Array Properties

Get the length:

```ghook
let count = numbers.length
```

## Iterating

Use `foreach` to loop over arrays:

```ghook
foreach numbers {
    num in
    run "echo ${num}"
}
```
