// V2 API Re-exports
pub use githook_syntax::{Statement, Token, tokenize, parse, Diagnostic, Expression};
pub use githook_eval::{Executor, ExecutionResult, Value};
pub use githook_git;

pub mod prelude {
    pub use crate::{parse, tokenize, Executor};
    pub use crate::{Statement, Expression, Token, ExecutionResult, Value};
}