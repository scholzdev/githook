/// The result of executing a single statement or block.
///
/// Returned by [`Executor::execute_statement`](`crate::executor::Executor`) to signal
/// whether execution should continue, stop, or break out of a loop.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// Execution succeeded; continue to the next statement.
    Continue,
    /// A `break` statement was encountered inside a loop.
    Break,
    /// A `continue` statement was encountered inside a loop.
    ContinueLoop,
    /// The commit was blocked (a `block` check failed).
    Blocked,
}

impl ExecutionResult {
    /// Returns `true` if execution should be halted entirely (commit blocked).
    pub fn should_stop(&self) -> bool {
        matches!(self, Self::Blocked)
    }

    /// Returns `true` if a `break` was encountered.
    pub fn is_break(&self) -> bool {
        matches!(self, Self::Break)
    }

    /// Returns `true` if a `continue` was encountered.
    pub fn is_continue(&self) -> bool {
        matches!(self, Self::ContinueLoop)
    }
}
