# list

List installed packages and cached remote packages.

## Usage

```bash
githook list
```

## Description

The `list` command displays:
- Locally installed packages
- Cached remote packages that have been downloaded

This helps you see which packages are available for use in your `.ghook` files.

## Example Output

```
Installed packages:
  - stdlib/git
  - stdlib/quality
  - stdlib/security
  - stdlib/time

Cached remotes:
  - github.com/user/custom-hooks
```

## See Also

- [Standard Library](../stdlib/README.md)
- [Package System](../advanced/packages.md)