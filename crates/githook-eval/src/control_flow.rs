#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    Continue,
    Break,
    ContinueLoop,
    Blocked,
}

impl ExecutionResult {
    pub fn should_stop(&self) -> bool {
        matches!(self, Self::Blocked)
    }

    pub fn is_break(&self) -> bool {
        matches!(self, Self::Break)
    }

    pub fn is_continue(&self) -> bool {
        matches!(self, Self::ContinueLoop)
    }
}
