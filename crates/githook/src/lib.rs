//! # GitHook
//!
//! A flexible and powerful DSL (Domain Specific Language) for configuring Git hooks.
//!
//! ## Overview
//!
//! GitHook provides a high-level, expressive language for writing Git hook scripts that
//! are easier to read, write, and maintain than traditional shell scripts. It includes
//! built-in functions for common Git operations, file manipulations, and validation checks.
//!
//! ## Features
//!
//! - **Expressive DSL**: Write hooks in a clear, readable syntax
//! - **Git Integration**: Built-in functions for Git operations (staged files, diffs, etc.)
//! - **Type System**: Strong typing with strings, numbers, booleans, arrays, and objects
//! - **Control Flow**: If/else, foreach loops, when blocks
//! - **LSP Support**: Full Language Server Protocol implementation for IDE integration
//! - **Performance**: LRU caching and parallel execution support
//!
//! ## Quick Start
//!
//! ```rust
//! use githook::prelude::*;
//!
//! // Parse a GitHook script
//! let source = r#"
//!     if git.files.staged.any(|f| f.name.ends_with(".rs")) {
//!         run "cargo fmt --check"
//!         run "cargo clippy"
//!     }
//! "#;
//!
//! // Tokenize and parse
//! let tokens = tokenize(source).expect("Failed to tokenize");
//! let statements = parse(tokens).expect("Failed to parse");
//!
//! // Execute the script
//! let mut executor = Executor::new()
//!     .with_git_files(vec!["src/main.rs".to_string()]);
//!
//! let result = executor.execute_statements(&statements)
//!     .expect("Execution failed");
//! ```
//!
//! ## Architecture
//!
//! This crate is the main entry point and re-exports components from:
//!
//! - [`githook-syntax`](../githook_syntax): Lexer, parser, and AST definitions
//! - [`githook-eval`](../githook_eval): Expression evaluator and script executor
//! - [`githook-git`](../githook_git): Git operations and integrations
//!
//! ## Examples
//!
//! ### Pre-commit Hook
//!
//! ```githook
//! // Check for TODOs in staged files
//! foreach file in git.files.staged {
//!     if file.content.contains("TODO") {
//!         warn "TODO found in " + file.name
//!     }
//! }
//!
//! // Run tests for Rust files
//! if git.files.staged.any(|f| f.name.ends_with(".rs")) {
//!     run "cargo test"
//! }
//! ```
//!
//! ### Commit Message Hook
//!
//! ```githook
//! let msg = git.commit.message
//!
//! // Enforce conventional commits
//! if !msg.matches("^(feat|fix|docs|chore):") {
//!     block "Commit message must follow conventional commits format"
//! }
//! ```

pub use githook_eval::{ExecutionResult, Executor, Value};
pub use githook_git;
pub use githook_syntax::{Diagnostic, Expression, Statement, Token, parse, tokenize};

/// Convenient re-exports for common use cases.
///
/// Import everything you need with `use githook::prelude::*;`
pub mod prelude {
    pub use crate::{ExecutionResult, Expression, Statement, Token, Value};
    pub use crate::{Executor, parse, tokenize};
}
