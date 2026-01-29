# Methods

## Array Methods

### filter, map, find, any, all

```ghook
let evens = numbers.filter(x => x % 2 == 0)
let doubled = numbers.map(x => x * 2)
let first = numbers.find(x => x > 100)
let has_large = numbers.any(x => x > 100)
let all_positive = numbers.all(x => x > 0)
```

### join, reverse, sort

```ghook
let text = numbers.join(", ")
let reversed = numbers.reverse()
let sorted = numbers.sort()
```

## String Methods

```ghook
let len = text.length
let has = text.contains("word")
let starts = text.starts_with("prefix")
let ends = text.ends_with("suffix")
let words = text.split(" ")
let clean = text.replace("old", "new")
let upper = text.to_uppercase()
let lower = text.to_lowercase()
```

## Number Methods

```ghook
let positive = num.abs()
let rounded = num.round()
let down = num.floor()
let up = num.ceil()
```
