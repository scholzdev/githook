use crate::error::Span;
use smallvec::SmallVec;

type MacroParams = SmallVec<[String; 4]>;
type ParallelCommands = SmallVec<[String; 4]>;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,
    Minus,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Run {
        command: String,
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
        interactive: Option<String>,
        span: Span,
    },

    WarnIf {
        condition: Expression,
        message: Option<String>,
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

#[derive(Debug, Clone)]
pub enum LetValue {
    String(String),
    Number(f64),
    Array(Vec<String>),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MatchPattern {
    Wildcard(String, Span),
    Expression(Expression, Span),
    Underscore(Span),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

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
