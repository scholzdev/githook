//! # githook-eval
//!
//! Tree-walking interpreter for the Githook scripting language.
//!
//! The main entry point is [`Executor`], which evaluates a list of
//! [`githook_syntax::Statement`] nodes and produces [`ExecutionResult`]s
//! with pass/fail/warn outcomes.

/// Built-in functions (`file`, `dir`, `glob`, `exec`, `rm`).
pub mod builtins;
/// Runtime configuration (timeouts, parallelism, auth).
pub mod config;
/// Context types for the Git object model (`git.branch`, `git.files`, etc.).
pub mod contexts;
/// Loop and block control flow.
pub mod control_flow;
/// Runtime error type with source-location tracking.
pub mod error;
/// The main tree-walking executor.
pub mod executor;
/// String interpolation (`"${expr}"`).
pub mod interpolation;
/// Runtime value types.
pub mod value;

/// Remote package resolution.
pub mod package_resolver;

pub use config::Config;
pub use control_flow::ExecutionResult;
pub use error::EvalError;
pub use executor::{CheckResult, CheckStatus, Executor};
pub use value::Value;
