# Data Types

Every value has a type. GitHook has six basic types:

## String

```ghook
let name = "Alice"
```

## Number

All numbers are floating point:

```ghook
let count = 42
let pi = 3.14
```

## Boolean

```ghook
let is_valid = true
let is_empty = false
```

## Null

Represents absence of a value:

```ghook
let result = numbers.find(x => x > 100)  # returns null if not found
```

## Array

```ghook
let numbers = [1, 2, 3, 4, 5]
let mixed = ["text", 42, true]
```

## Object

Objects have properties:

```ghook
let branch = git.branch.name
let email = git.author.email
```
