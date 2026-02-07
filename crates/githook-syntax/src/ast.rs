use crate::error::Span;
use smallvec::SmallVec;

type MacroParams = SmallVec<[String; 4]>;
type ParallelCommands = SmallVec<[String; 4]>;

/// An expression node in the Githook AST.
///
/// Expressions evaluate to a [`crate::value::Value`] and include literals,
/// identifiers, property access, method calls, binary/unary operations,
/// arrays, closures, and interpolated strings.
#[derive(Debug, Clone)]
pub enum Expression {
    String(String, Span),
    Number(f64, Span),
    Bool(bool, Span),
    Null(Span),

    Identifier(String, Span),

    PropertyAccess {
        receiver: Box<Expression>,
        property: String,
        span: Span,
    },

    MethodCall {
        receiver: Box<Expression>,
        method: String,
        args: Vec<Expression>,
        span: Span,
    },

    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
        span: Span,
    },

    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
        span: Span,
    },

    Array(Vec<Expression>, Span),

    Closure {
        param: String,
        body: Box<Expression>,
        span: Span,
    },

    /// Inline conditional: `if cond then expr else expr`.
    IfExpr {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
        span: Span,
    },

    InterpolatedString {
        parts: Vec<StringPart>,
        span: Span,
    },

    /// Bracket/index access: `expr["key"]` or `expr[0]`.
    IndexAccess {
        receiver: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },
}

/// A part of an interpolated string (`"hello ${name}"`).
#[derive(Debug, Clone)]
pub enum StringPart {
    /// Raw text between interpolation boundaries.
    Literal(String),
    /// An embedded `${...}` expression.
    Expression(Expression),
}

/// Binary operators supported in expressions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

/// Unary operators (`not`, `-`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    /// Logical negation (`not expr`).
    Not,
    /// Arithmetic negation (`-expr`).
    Minus,
}

/// A top-level statement node in the Githook AST.
///
/// Each variant corresponds to one language construct (e.g. `run`, `let`,
/// `foreach`, `if`, `group`, `macro`, etc.).
#[derive(Debug, Clone)]
pub enum Statement {
    Run {
        command: Expression,
        span: Span,
    },

    Print {
        message: Expression,
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
        commands: ParallelCommands,
        span: Span,
    },

    Allow {
        command: String,
        span: Span,
    },

    Let {
        name: String,
        value: LetValue,
        span: Span,
    },

    ForEach {
        collection: Expression,
        var: String,
        pattern: Option<String>,
        body: Vec<Statement>,
        span: Span,
    },

    If {
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
        span: Span,
    },

    Break {
        span: Span,
    },

    Continue {
        span: Span,
    },

    BlockIf {
        condition: Expression,
        message: Option<String>,
        /// Reserved for future use: optional prompt text for interactive mode.
        interactive: Option<String>,
        span: Span,
    },

    WarnIf {
        condition: Expression,
        message: Option<String>,
        /// Reserved for future use: optional prompt text for interactive mode.
        interactive: Option<String>,
        span: Span,
    },

    Match {
        subject: Expression,
        arms: Vec<MatchArm>,
        span: Span,
    },

    MacroDef {
        name: String,
        params: MacroParams,
        body: Vec<Statement>,
        span: Span,
    },

    MacroCall {
        namespace: Option<String>,
        name: String,
        args: Vec<Expression>,
        span: Span,
    },

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

    Group {
        name: String,
        severity: Option<Severity>,
        enabled: bool,
        body: Vec<Statement>,
        span: Span,
    },

    Try {
        body: Vec<Statement>,
        catch_var: Option<String>,
        catch_body: Vec<Statement>,
        span: Span,
    },
}

/// The right-hand side of a `let` binding.
#[derive(Debug, Clone)]
pub enum LetValue {
    /// A string literal value.
    String(String),
    /// A numeric literal value.
    Number(f64),
    /// An array literal of strings.
    Array(Vec<String>),
    /// An arbitrary expression.
    Expression(Expression),
}

/// One arm of a `match` expression.
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// The pattern to match against.
    pub pattern: MatchPattern,
    /// Statements to execute when the pattern matches.
    pub body: Vec<Statement>,
    /// Source location.
    pub span: Span,
}

/// A pattern in a `match` arm.
#[derive(Debug, Clone)]
pub enum MatchPattern {
    /// A glob/wildcard string pattern (e.g. `"feature/*"`).
    Wildcard(String, Span),
    /// A literal expression pattern.
    Expression(Expression, Span),
    /// The catch-all `_` pattern.
    Underscore(Span),
}

/// Severity level for a `group` block.
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    /// Blocks the commit on failure (default).
    Critical,
    /// Emits a warning but allows the commit.
    Warning,
    /// Informational only.
    Info,
}

impl Expression {
    /// Returns the source [`Span`] for this expression.
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
            Expression::IfExpr { span, .. } => span,
            Expression::InterpolatedString { span, .. } => span,
            Expression::IndexAccess { span, .. } => span,
        }
    }

    /// Statically evaluates whether the expression is truthy, if possible.
    ///
    /// Returns `None` for expressions that cannot be evaluated at parse time.
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
    /// Returns the source [`Span`] for this statement.
    pub fn span(&self) -> &Span {
        match self {
            Statement::Run { span, .. } => span,
            Statement::Print { span, .. } => span,
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
