# Custom Snippets

Create and share reusable validation patterns.

## Creating Custom Snippets

```javascript
macro my_validation {
  block_if file_size > 1000 message "Too large"
}
```

## With Parameters

```javascript
macro check_size(max) {
  block_if file_size > {max} message "Exceeds {max} bytes"
}
```

## Organizing Snippets

```
.githook/
└── custom/
    ├── security.ghook
    ├── quality.ghook
    └── style.ghook
```

## Using Custom Snippets

```javascript
import "./custom/security.ghook" as sec

@sec:no_secrets
```

See [Snippets Language Guide](../language/snippets.md) for complete reference.
