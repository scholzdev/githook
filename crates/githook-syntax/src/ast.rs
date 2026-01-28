use crate::error::Span;

#[derive(Debug, Clone)]
pub enum Statement {
    Run(String, Span),
    Block(String, Span),
    BoolLiteral(bool, Span),
    Parallel {
        commands: Vec<String>,
        span: Span,
    },

    LetStringList { name: String, items: Vec<String>, span: Span },
    ForEachStringList { var: String, list: String, body: Vec<Statement>, span: Span },
    ForEachArray { var: String, items: Vec<Argument>, body: Vec<Statement>, span: Span },
    ForEachStagedFiles { var: String, pattern: String, where_cond: Option<BlockCondition>, body: Vec<Statement>, span: Span },
    MacroDefinition {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        span: Span,
    },
    MacroCall {
        namespace: Option<String>,
        name: String,
        args: Vec<Argument>,
        span: Span,
    },
    Use {
        namespace: String,
        name: String,
        alias: Option<String>,
        span: Span,
    },
    Import {
        path: String,
        alias: Option<String>,
        span: Span,
    },
    Group { definition: GroupDefinition, span: Span },
    StagedFiles {
        pattern: String,
        body: Vec<Statement>,
        span: Span,
    },
    StagedContentValidation {
        must: bool,
        check: ContentCheck,
        pattern: Option<String>,
        span: Span,
    },
    StagedContentForeach {
        pattern: String,
        body: Vec<Statement>,
        span: Span,
    },
    AllowCommand(String, Span),
    AllFiles {
        pattern: String,
        body: Vec<Statement>,
        span: Span,
    },
    FileRule {
        pattern: String,
        must_be_staged: bool,
        span: Span,
    },
    ContentValidation {
        scope: ContentScope,
        must: bool,
        check: ContentCheck,
        pattern: Option<String>,
        span: Span,
    },
    MessageValidation {
        must: bool,
        check: MessageCheck,
        span: Span,
    },
    ConditionalRule {
        severity: RuleSeverity,
        condition: BlockCondition,
        message: Option<String>,
        interactive: Option<String>,
        span: Span,
    },
    When {
        condition: BlockCondition,
        body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
        span: Span,
    },
    Match {
        subject: MatchSubject,
        arms: Vec<MatchArm>,
        span: Span,
    }
}

#[derive(Debug, Clone)]
pub enum MatchSubject {
    File(Span),
    Content(Span),
    Diff(Span),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub action: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MatchPattern {
    Wildcard(String, Span),
    Contains(String, Span),
    Matches(String, Span),
    GreaterThan(f64, Span),
    LessThan(f64, Span),
}

#[derive(Debug, Clone)]
pub struct GroupDefinition {
    pub name: String,
    pub severity: Option<GroupSeverity>,
    pub enabled: Option<bool>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum GroupSeverity {
    Critical(Span),
    Warning(Span),
    Info(Span),
}

#[derive(Debug, Clone)]
pub enum Expression {
    MacroCall(String, Vec<Argument>, Span),
    Not(Box<Expression>, Span),
    And(Box<Expression>, Box<Expression>, Span),
    Or(Box<Expression>, Box<Expression>, Span),
    Condition(BlockCondition, Span),
}

#[derive(Debug, Clone)]
pub enum Argument {
    String(String, Span),
    Number(f64, Span),
    Identifier(String, Span),
    Array(Vec<Argument>, Span),
}

#[derive(Debug, Clone)]
pub enum ContentScope {
    Content(Span),
    Diff(Span),
}

#[derive(Debug, Clone)]
pub enum ContentCheck {
    Match(String, Span),
    Contain(String, Span),
}

#[derive(Debug, Clone)]
pub enum RuleSeverity {
    Warn(Span),
    Block(Span),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonOperator {
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    Equals,
    
    Matches,
    Contains,
    In,
}

#[derive(Debug, Clone)]
pub enum PropertyValue {
    FileSize(Span),
    BranchName(Span),
    Content(Span),
    Diff(Span),
    CommitMessage(Span),
    Extension(Span),
    Filename(Span),
    Basename(Span),
    Dirname(Span),
    ModifiedLines(Span),
    FilesChanged(Span),
    Additions(Span),
    Deletions(Span),
    CommitsAhead(Span),
    EnvVar(String, Span),
    Placeholder(String, Span),
}

#[derive(Debug, Clone)]
pub enum ComparisonValue {
    String(String, Span),
    Number(f64, Span),
    Identifier(String, Span),
    ListIdentifier(String, Span),
}

#[derive(Debug, Clone)]
pub enum BlockCondition {
    Comparison {
        left: PropertyValue,
        operator: ComparisonOperator,
        right: ComparisonValue,
        negated: bool,
        span: Span,
    },
    
    InStringList { value: String, list: String, span: Span },
    StringEquals { left: String, right: String, right_is_identifier: bool, span: Span },
    ContentCheck { scope: ContentScope, check: ContentCheck, span: Span },
    
    ContainsSecrets(Span),
    AuthorSet(Span),
    AuthorEmailSet(Span),
    AuthorMissing(Span),
    EnvEquals(String, String, Span),
    
    MacroCall { name: String, args: Vec<Argument>, span: Span },
    NotMacroCall { name: String, args: Vec<Argument>, span: Span },
    
    Not { inner: Box<BlockCondition>, span: Span },
    And { left: Box<BlockCondition>, right: Box<BlockCondition>, span: Span },
    Or { left: Box<BlockCondition>, right: Box<BlockCondition>, span: Span },
    Bool(bool, Span),
}

impl BlockCondition {
    pub fn default_message(&self) -> String {
        match self {
            BlockCondition::Comparison { left, operator, right, negated, .. } => {
                let property = match left {
                    PropertyValue::FileSize(_) => "file size".to_string(),
                    PropertyValue::BranchName(_) => "branch name".to_string(),
                    PropertyValue::Content(_) => "content".to_string(),
                    PropertyValue::Diff(_) => "diff".to_string(),
                    PropertyValue::CommitMessage(_) => "commit message".to_string(),
                    PropertyValue::Extension(_) => "extension".to_string(),
                    PropertyValue::Filename(_) => "filename".to_string(),
                    PropertyValue::Basename(_) => "basename".to_string(),
                    PropertyValue::Dirname(_) => "dirname".to_string(),
                    PropertyValue::ModifiedLines(_) => "modified lines".to_string(),
                    PropertyValue::FilesChanged(_) => "files changed".to_string(),
                    PropertyValue::Additions(_) => "additions".to_string(),
                    PropertyValue::Deletions(_) => "deletions".to_string(),
                    PropertyValue::CommitsAhead(_) => "commits ahead".to_string(),
                    PropertyValue::EnvVar(key, _) => format!("env:{}", key),
                    PropertyValue::Placeholder(p, _) => format!("{{{}}}", p),
                };
                
                let value_str = match right {
                    ComparisonValue::String(s, _) => format!("\"{}\"", s),
                    ComparisonValue::Number(n, _) => n.to_string(),
                    ComparisonValue::Identifier(id, _) => id.clone(),
                    ComparisonValue::ListIdentifier(id, _) => format!("{{{}}}", id),
                };
                
                if *negated && operator == &ComparisonOperator::Equals {
                    format!("{} != {}", property, value_str)
                } else {
                    let op_str = match operator {
                        ComparisonOperator::Greater => ">",
                        ComparisonOperator::GreaterOrEqual => ">=",
                        ComparisonOperator::Less => "<",
                        ComparisonOperator::LessOrEqual => "<=",
                        ComparisonOperator::Equals => "==",
                        ComparisonOperator::Matches => "matches",
                        ComparisonOperator::Contains => "contains",
                        ComparisonOperator::In => "in",
                    };
                    
                    if *negated {
                        format!("{} not {} {}", property, op_str, value_str)
                    } else {
                        format!("{} {} {}", property, op_str, value_str)
                    }
                }
            }
            
            BlockCondition::ContainsSecrets(_) => "Potential secrets detected".into(),
            BlockCondition::AuthorMissing(_) => "Git author is missing".into(),
            BlockCondition::AuthorSet(_) => "Git author must be set".into(),
            BlockCondition::AuthorEmailSet(_) => "Git author email must be set".into(),
            BlockCondition::EnvEquals(key, val, _) => {
                format!("Environment {} must equal \"{}\"", key, val)
            }
            BlockCondition::MacroCall { name, .. } => {
                format!("@{} check passed", name)
            }
            BlockCondition::NotMacroCall { name, .. } => {
                format!("Not @{} check passed", name)
            }
            BlockCondition::InStringList { value, list, .. } => {
                format!("{} must not be in {}", value, list)
            }
            BlockCondition::Not { inner, .. } => {
                format!("not {}", inner.default_message())
            }
            BlockCondition::Bool(b, _) => {
                format!("{}", b)
            }
            BlockCondition::And { left, right, .. } => {
                format!("{} and {}", left.default_message(), right.default_message())
            }
            BlockCondition::Or { left, right, .. } => {
                format!("{} or {}", left.default_message(), right.default_message())
            }
            BlockCondition::StringEquals { left, right, .. } => {
                format!("{} must equal \"{}\"", left, right)
            }
            BlockCondition::ContentCheck { scope, check, .. } => {
                let what = match scope { 
                    ContentScope::Content(_) => "content", 
                    ContentScope::Diff(_) => "diff" 
                };
                match check {
                    ContentCheck::Match(p, _) => format!("{} must match \"{}\"", what, p),
                    ContentCheck::Contain(t, _) => format!("{} must contain \"{}\"", what, t),
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum MessageCheck {
    Match(String, Span),
    Contain(String, Span),
}

impl Statement {
    pub fn span(&self) -> Span {
        match self {
            Statement::Run(_, span) => *span,
            Statement::Block(_, span) => *span,
            Statement::BoolLiteral(_, span) => *span,
            Statement::Parallel { span, .. } => *span,
            Statement::LetStringList { span, .. } => *span,
            Statement::ForEachStringList { span, .. } => *span,
            Statement::ForEachArray { span, .. } => *span,
            Statement::ForEachStagedFiles { span, .. } => *span,
            Statement::MacroDefinition { span, .. } => *span,
            Statement::MacroCall { span, .. } => *span,
            Statement::Use { span, .. } => *span,
            Statement::Import { span, .. } => *span,
            Statement::Group { span, .. } => *span,
            Statement::StagedFiles { span, .. } => *span,
            Statement::StagedContentValidation { span, .. } => *span,
            Statement::StagedContentForeach { span, .. } => *span,
            Statement::AllowCommand(_, span) => *span,
            Statement::AllFiles { span, .. } => *span,
            Statement::FileRule { span, .. } => *span,
            Statement::ContentValidation { span, .. } => *span,
            Statement::MessageValidation { span, .. } => *span,
            Statement::ConditionalRule { span, .. } => *span,
            Statement::When { span, .. } => *span,
            Statement::Match { span, .. } => *span,
        }
    }
}

impl MatchSubject {
    pub fn span(&self) -> Span {
        match self {
            MatchSubject::File(span) => *span,
            MatchSubject::Content(span) => *span,
            MatchSubject::Diff(span) => *span,
        }
    }
}

impl MatchPattern {
    pub fn span(&self) -> Span {
        match self {
            MatchPattern::Wildcard(_, span) => *span,
            MatchPattern::Contains(_, span) => *span,
            MatchPattern::Matches(_, span) => *span,
            MatchPattern::GreaterThan(_, span) => *span,
            MatchPattern::LessThan(_, span) => *span,
        }
    }
}

impl GroupSeverity {
    pub fn span(&self) -> Span {
        match self {
            GroupSeverity::Critical(span) => *span,
            GroupSeverity::Warning(span) => *span,
            GroupSeverity::Info(span) => *span,
        }
    }
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::MacroCall(_, _, span) => *span,
            Expression::Not(_, span) => *span,
            Expression::And(_, _, span) => *span,
            Expression::Or(_, _, span) => *span,
            Expression::Condition(_, span) => *span,
        }
    }
}

impl Argument {
    pub fn span(&self) -> Span {
        match self {
            Argument::String(_, span) => *span,
            Argument::Number(_, span) => *span,
            Argument::Identifier(_, span) => *span,
            Argument::Array(_, span) => *span,
        }
    }
}

impl ContentScope {
    pub fn span(&self) -> Span {
        match self {
            ContentScope::Content(span) => *span,
            ContentScope::Diff(span) => *span,
        }
    }
}

impl ContentCheck {
    pub fn span(&self) -> Span {
        match self {
            ContentCheck::Match(_, span) => *span,
            ContentCheck::Contain(_, span) => *span,
        }
    }
}

impl RuleSeverity {
    pub fn span(&self) -> Span {
        match self {
            RuleSeverity::Warn(span) => *span,
            RuleSeverity::Block(span) => *span,
        }
    }
}

impl PropertyValue {
    pub fn span(&self) -> Span {
        match self {
            PropertyValue::FileSize(span) => *span,
            PropertyValue::BranchName(span) => *span,
            PropertyValue::Content(span) => *span,
            PropertyValue::Diff(span) => *span,
            PropertyValue::CommitMessage(span) => *span,
            PropertyValue::Extension(span) => *span,
            PropertyValue::Filename(span) => *span,
            PropertyValue::Basename(span) => *span,
            PropertyValue::Dirname(span) => *span,
            PropertyValue::ModifiedLines(span) => *span,
            PropertyValue::FilesChanged(span) => *span,
            PropertyValue::Additions(span) => *span,
            PropertyValue::Deletions(span) => *span,
            PropertyValue::CommitsAhead(span) => *span,
            PropertyValue::EnvVar(_, span) => *span,
            PropertyValue::Placeholder(_, span) => *span,
        }
    }
}

impl ComparisonValue {
    pub fn span(&self) -> Span {
        match self {
            ComparisonValue::String(_, span) => *span,
            ComparisonValue::Number(_, span) => *span,
            ComparisonValue::Identifier(_, span) => *span,
            ComparisonValue::ListIdentifier(_, span) => *span,
        }
    }
}

impl BlockCondition {
    pub fn span(&self) -> Span {
        match self {
            BlockCondition::Comparison { span, .. } => *span,
            BlockCondition::InStringList { span, .. } => *span,
            BlockCondition::StringEquals { span, .. } => *span,
            BlockCondition::ContentCheck { span, .. } => *span,
            BlockCondition::ContainsSecrets(span) => *span,
            BlockCondition::AuthorSet(span) => *span,
            BlockCondition::AuthorEmailSet(span) => *span,
            BlockCondition::AuthorMissing(span) => *span,
            BlockCondition::EnvEquals(_, _, span) => *span,
            BlockCondition::MacroCall { span, .. } => *span,
            BlockCondition::NotMacroCall { span, .. } => *span,
            BlockCondition::Not { span, .. } => *span,
            BlockCondition::And { span, .. } => *span,
            BlockCondition::Or { span, .. } => *span,
            BlockCondition::Bool(_, span) => *span,
        }
    }
}

impl MessageCheck {
    pub fn span(&self) -> Span {
        match self {
            MessageCheck::Match(_, span) => *span,
            MessageCheck::Contain(_, span) => *span,
        }
    }
}