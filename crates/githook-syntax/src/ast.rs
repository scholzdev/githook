use crate::error::Span;

// ============================================================================
// EXPRESSIONS - Unified expression system
// ============================================================================

#[derive(Debug, Clone)]
pub enum Expression {
    // Literals
    String(String, Span),
    Number(f64, Span),
    Bool(bool, Span),
    Null(Span),
    
    // Identifiers and property access
    Identifier(String, Span),
    
    PropertyAccess {
        chain: Vec<String>,  // ["git", "branch", "name"]
        span: Span,
    },
    
    MethodCall {
        receiver: Box<Expression>,
        method: String,
        args: Vec<Expression>,
        span: Span,
    },
    
    // Binary operations
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
        span: Span,
    },
    
    // Unary operations
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
        span: Span,
    },
    
    // Array literal
    Array(Vec<Expression>, Span),
    
    // Closure/Lambda
    Closure {
        param: String,
        body: Box<Expression>,
        span: Span,
    },
    
    // String interpolation (for future processing)
    InterpolatedString {
        parts: Vec<StringPart>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Literal(String),
    Expression(Expression),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    // Comparison
    Eq,        // ==
    Ne,        // !=
    Lt,        // <
    Le,        // <=
    Gt,        // >
    Ge,        // >=
    
    // Logical
    And,       // and
    Or,        // or
    
    // Arithmetic
    Add,       // +
    Sub,       // -
    Mul,       // *
    Div,       // /
    Mod,       // %
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,       // not
    Minus,     // - (unary minus)
}

// ============================================================================
// STATEMENTS
// ============================================================================

#[derive(Debug, Clone)]
pub enum Statement {
    // Commands
    Run {
        command: String,  // Still string for now, can contain ${...}
        span: Span,
    },
    
    Block {
        message: String,
        span: Span,
    },
    
    Warn {
        message: String,
        span: Span,
    },
    
    Parallel {
        commands: Vec<String>,
        span: Span,
    },
    
    Allow {
        command: String,
        span: Span,
    },
    
    // Variables
    Let {
        name: String,
        value: LetValue,
        span: Span,
    },
    
    // UNIFIED FOREACH
    ForEach {
        collection: Expression,
        var: String,
        where_clause: Option<Expression>,  // Optional filter
        body: Vec<Statement>,
        span: Span,
    },
    
    // Conditional - Block form
    If {
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
        span: Span,
    },
    
    // Control flow
    Break {
        span: Span,
    },
    
    Continue {
        span: Span,
    },
    
    // Conditional - Short form
    BlockIf {
        condition: Expression,
        message: Option<String>,
        interactive: Option<String>,
        span: Span,
    },
    
    WarnIf {
        condition: Expression,
        message: Option<String>,
        interactive: Option<String>,
        span: Span,
    },
    
    // Match
    Match {
        subject: Expression,
        arms: Vec<MatchArm>,
        span: Span,
    },
    
    // Macros
    MacroDef {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        span: Span,
    },
    
    MacroCall {
        namespace: Option<String>,
        name: String,
        args: Vec<Expression>,
        span: Span,
    },
    
    // Imports
    Import {
        path: String,
        alias: Option<String>,
        span: Span,
    },
    
    Use {
        package: String,
        alias: Option<String>,
        span: Span,
    },
    
    // Group
    Group {
        name: String,
        severity: Option<Severity>,
        enabled: bool,
        body: Vec<Statement>,
        span: Span,
    },
    
    // Error handling
    Try {
        body: Vec<Statement>,
        catch_var: Option<String>,
        catch_body: Vec<Statement>,
        span: Span,
    },
}

// ============================================================================
// SUPPORTING TYPES
// ============================================================================

#[derive(Debug, Clone)]
pub enum LetValue {
    String(String),
    Number(f64),
    Array(Vec<String>),  // For now, only string arrays
    Expression(Expression),  // Future: any expression
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MatchPattern {
    // For file matching
    Wildcard(String, Span),    // "*.rs"
    
    // For content/value matching
    Expression(Expression, Span),  // any expression as pattern
    
    // Catch-all
    Underscore(Span),          // _
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

// ============================================================================
// HELPER METHODS
// ============================================================================

impl Expression {
    pub fn span(&self) -> &Span {
        match self {
            Expression::String(_, s) => s,
            Expression::Number(_, s) => s,
            Expression::Bool(_, s) => s,
            Expression::Null(s) => s,
            Expression::Identifier(_, s) => s,
            Expression::PropertyAccess { span, .. } => span,
            Expression::MethodCall { span, .. } => span,
            Expression::Binary { span, .. } => span,
            Expression::Unary { span, .. } => span,
            Expression::Array(_, s) => s,
            Expression::Closure { span, .. } => span,
            Expression::InterpolatedString { span, .. } => span,
        }
    }
    
    pub fn is_truthy(&self) -> Option<bool> {
        match self {
            Expression::Bool(b, _) => Some(*b),
            Expression::Null(_) => Some(false),
            Expression::Number(n, _) => Some(*n != 0.0),
            Expression::String(s, _) => Some(!s.is_empty()),
            _ => None,
        }
    }
}

impl Statement {
    pub fn span(&self) -> &Span {
        match self {
            Statement::Run { span, .. } => span,
            Statement::Block { span, .. } => span,
            Statement::Warn { span, .. } => span,
            Statement::Parallel { span, .. } => span,
            Statement::Allow { span, .. } => span,
            Statement::Let { span, .. } => span,
            Statement::ForEach { span, .. } => span,
            Statement::If { span, .. } => span,
            Statement::Break { span } => span,
            Statement::Continue { span } => span,
            Statement::BlockIf { span, .. } => span,
            Statement::WarnIf { span, .. } => span,
            Statement::Match { span, .. } => span,
            Statement::MacroDef { span, .. } => span,
            Statement::MacroCall { span, .. } => span,
            Statement::Import { span, .. } => span,
            Statement::Use { span, .. } => span,
            Statement::Group { span, .. } => span,
            Statement::Try { span, .. } => span,
        }
    }
}
