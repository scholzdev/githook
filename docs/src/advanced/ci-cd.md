# CI/CD Integration

Run Githook in continuous integration pipelines.

## GitHub Actions

```yaml
name: Validate
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Githook
        run: |
          curl -L https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-unknown-linux-gnu -o githook
          chmod +x githook
          sudo mv githook /usr/local/bin/
      
      - name: Run validations
        run: githook pre-commit
```

## GitLab CI

```yaml
validate:
  stage: test
  script:
    - curl -L https://github.com/scholzdev/githook/releases/latest/download/githook-x86_64-unknown-linux-gnu -o githook
    - chmod +x githook
    - ./githook pre-commit
```

## Force Hooks in CI

```yaml
- name: Run hooks
  run: GITHOOK_FORCE=1 githook pre-commit
```
