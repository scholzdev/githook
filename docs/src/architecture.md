# Architecture

Technical overview of Githook's architecture.

## Components

### githook-syntax
Lexer and parser for `.ghook` files. Produces an AST.

### githook-eval
Runtime interpreter that executes the AST. Includes:
- Context resolution
- Expression evaluation
- Control flow
- Snippet resolution

### githook-git
Git integration layer. Provides:
- File operations
- Diff parsing
- Branch information
- Commit metadata

### githook-cli
Command-line interface. Handles:
- Hook initialization
- Hook execution
- Validation
- Updates

### githook-lsp
Language Server Protocol implementation for editor integration.

## Execution Flow

1. **Parse** - `.ghook` file → AST
2. **Resolve** - Load imports and snippets
3. **Execute** - Evaluate AST with Git context
4. **Report** - Collect and display results

## Caching

Githook caches:
- Parsed AST
- Resolved imports
- Git context (where possible)

## Parallelization

- File checks run in parallel
- Foreach loops parallelized automatically
- Safe concurrent execution

## Performance

- Zero-copy string operations where possible
- Lazy evaluation of expressions
- Efficient diff parsing
- Minimal Git invocations
