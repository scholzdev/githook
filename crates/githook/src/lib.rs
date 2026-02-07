//! # githook
//!
//! The top-level façade crate for the Githook scripting language.
//!
//! Re-exports the core types from [`githook_syntax`], [`githook_eval`],
//! and [`githook_git`] so downstream consumers only need a single dependency.

pub use githook_eval::{ExecutionResult, Executor, Value};
pub use githook_git;
pub use githook_syntax::{Diagnostic, Expression, Statement, Token, parse, tokenize};

/// Convenience re-exports for common Githook types.
///
/// `use githook::prelude::*` brings all the core types into scope.
pub mod prelude {
    pub use crate::{ExecutionResult, Expression, Statement, Token, Value};
    pub use crate::{Executor, parse, tokenize};
}
