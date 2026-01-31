use std::fmt;

pub fn format_error_with_source(error_msg: &str, source: &str, span: Span) -> String {
    let lines: Vec<&str> = source.lines().collect();
    
    let line_idx = if span.line > 0 { span.line - 1 } else { 0 };
    
    if line_idx >= lines.len() {
        return format!("{} at line {}", error_msg, span.line);
    }
    
    let error_line = lines[line_idx];
    let line_num = span.line;
    
    let mut output = String::new();
    output.push_str(&format!("  --> line {}:{}\n", line_num, span.col));
    output.push_str("   |\n");
    
    if line_idx > 0 {
        output.push_str(&format!(" {} | {}\n", line_num - 1, lines[line_idx - 1]));
    }
    
    output.push_str(&format!(" {} | {}\n", line_num, error_line));
    
    output.push_str(&format!("   | {}^ {}\n", " ".repeat(span.col.saturating_sub(1)), error_msg));
    
    if line_idx + 1 < lines.len() {
        output.push_str(&format!(" {} | {}\n", line_num + 1, lines[line_idx + 1]));
    }
    
    output.push_str("   |");
    
    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(line: usize, col: usize, start: usize, end: usize) -> Self {
        Self { line, col, start, end }
    }

    pub fn single(line: usize, col: usize, offset: usize) -> Self {
        Self { line, col, start: offset, end: offset + 1 }
    }

    pub fn merge(&self, other: &Span) -> Self {
        Self {
            line: self.line.min(other.line),
            col: if self.line == other.line { self.col.min(other.col) } else { self.col },
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LexError {
    UnexpectedChar { ch: char, span: Span, suggestion: Option<String> },
    UnterminatedString { span: Span },
    UnterminatedComment { span: Span },
    InvalidNumber { text: String, span: Span },
    InvalidEscape { ch: char, span: Span },
    UnexpectedEof { expected: String, span: Span },
}

impl LexError {
    pub fn span(&self) -> Span {
        match self {
            LexError::UnexpectedChar { span, .. } => *span,
            LexError::UnterminatedString { span } => *span,
            LexError::UnterminatedComment { span } => *span,
            LexError::InvalidNumber { span, .. } => *span,
            LexError::InvalidEscape { span, .. } => *span,
            LexError::UnexpectedEof { span, .. } => *span,
        }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnexpectedChar { ch, suggestion, .. } => {
                write!(f, "unexpected character '{}'", ch)?;
                if let Some(s) = suggestion {
                    write!(f, " (did you mean '{}'?)", s)?;
                }
                Ok(())
            }
            LexError::UnterminatedString { .. } => {
                write!(f, "unterminated string literal")
            }
            LexError::UnterminatedComment { .. } => {
                write!(f, "unterminated multi-line comment")
            }
            LexError::InvalidNumber { text, .. } => {
                write!(f, "invalid number: '{}'", text)
            }
            LexError::InvalidEscape { ch, .. } => {
                write!(f, "invalid escape sequence: '\\{}'", ch)
            }
            LexError::UnexpectedEof { expected, .. } => {
                write!(f, "unexpected end of file, expected {}", expected)
            }
        }
    }
}

impl std::error::Error for LexError {}

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedToken { expected: String, found: String, span: Span },
    UnexpectedEof { expected: String, context: Option<String> },
    MissingToken { expected: String, span: Span },
    InvalidSyntax { message: String, span: Span },
    LexError(LexError),
}

impl ParseError {
    pub fn span(&self) -> Option<Span> {
        match self {
            ParseError::UnexpectedToken { span, .. } => Some(*span),
            ParseError::UnexpectedEof { .. } => None,
            ParseError::MissingToken { span, .. } => Some(*span),
            ParseError::InvalidSyntax { span, .. } => Some(*span),
            ParseError::LexError(e) => Some(e.span()),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken { expected, found, .. } => {
                write!(f, "expected {}, found {}", expected, found)
            }
            ParseError::UnexpectedEof { expected, context } => {
                if let Some(ctx) = context {
                    write!(f, "unexpected end of file while parsing {}, expected {}", ctx, expected)
                } else {
                    write!(f, "unexpected end of file, expected {}", expected)
                }
            }
            ParseError::MissingToken { expected, .. } => {
                write!(f, "missing {}", expected)
            }
            ParseError::InvalidSyntax { message, .. } => {
                write!(f, "{}", message)
            }
            ParseError::LexError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(err: LexError) -> Self {
        ParseError::LexError(err)
    }
}

pub struct Diagnostic<'a> {
    source: &'a str,
    error: DiagnosticError,
}

pub enum DiagnosticError {
    Lex(LexError),
    Parse(ParseError),
}

impl<'a> Diagnostic<'a> {
    pub fn new_lex(source: &'a str, error: LexError) -> Self {
        Self { source, error: DiagnosticError::Lex(error) }
    }

    pub fn new_parse(source: &'a str, error: ParseError) -> Self {
        Self { source, error: DiagnosticError::Parse(error) }
    }

    fn span(&self) -> Option<Span> {
        match &self.error {
            DiagnosticError::Lex(e) => Some(e.span()),
            DiagnosticError::Parse(e) => e.span(),
        }
    }

    fn message(&self) -> String {
        match &self.error {
            DiagnosticError::Lex(e) => e.to_string(),
            DiagnosticError::Parse(e) => e.to_string(),
        }
    }

    fn error_label(&self) -> &str {
        match &self.error {
            DiagnosticError::Lex(_) => "lexical error",
            DiagnosticError::Parse(_) => "parse error",
        }
    }

    pub fn format_error(&self) -> String {
        let mut output = String::new();
        
        let span = self.span();
        let message = self.message();
        let label = self.error_label();

        output.push_str(&format!("\x1b[1;31merror\x1b[0m: {}\n", message));

        if let Some(span) = span {
            output.push_str(&format!("  \x1b[1;34m-->\x1b[0m line {}:{}\n", span.line, span.col));
            output.push_str("   \x1b[1;34m|\x1b[0m\n");

            let lines: Vec<&str> = self.source.lines().collect();
            
            if span.line > 0 && span.line <= lines.len() {
                let line_idx = span.line - 1;
                let line_content = lines[line_idx];
                
                let line_num_width = (span.line + 1).to_string().len().max(2);
                output.push_str(&format!(" {: >width$} \x1b[1;34m|\x1b[0m {}\n", 
                    span.line, line_content, width = line_num_width));

                let mut visual_col = 0;
                for (idx, ch) in line_content.chars().enumerate() {
                    if idx >= span.col - 1 {
                        break;
                    }
                    visual_col += if ch == '\t' { 4 } else { 1 };
                }

                let error_len = if span.end > span.start {
                    let error_text = if span.start < self.source.len() && span.end <= self.source.len() {
                        &self.source[span.start..span.end]
                    } else {
                        "?"
                    };
                    error_text.chars().count().max(1)
                } else {
                    1
                };

                output.push_str(&format!(" {: >width$} \x1b[1;34m|\x1b[0m {}\x1b[1;31m{}\x1b[0m {}\n",
                    "",
                    " ".repeat(visual_col),
                    "^".repeat(error_len),
                    label,
                    width = line_num_width
                ));
            }

            output.push_str("   \x1b[1;34m|\x1b[0m\n");
        } else {
            output.push_str(&format!("   \x1b[1;34m|\x1b[0m {}\n", label));
        }

        output
    }
}

impl<'a> fmt::Display for Diagnostic<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_error())
    }
}
