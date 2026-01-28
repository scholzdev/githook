pub use githook_syntax::{Statement, Token, tokenize_with_spans, parse_spanned, Diagnostic};
pub use githook_eval::{execute, execute_with_filters, ExecutionStatus};
pub use githook_git;

pub mod prelude {
    pub use crate::{parse_spanned, execute, execute_with_filters};
    pub use crate::{Statement, Token, ExecutionStatus};
}