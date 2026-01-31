use colored::*;
use githook_syntax::error::Span;
use std::fmt;

/// Enhanced error with context and suggestions
pub struct EnhancedError {
    pub message: String,
    pub span: Option<Span>,
    pub file: Option<String>,
    pub source: Option<String>,
    pub suggestion: Option<String>,
    pub help: Option<String>,
}

impl EnhancedError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            file: None,
            source: None,
            suggestion: None,
            help: None,
        }
    }
    
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
    
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }
    
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
    
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
    
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
    
    /// Display the error with colored output and context
    pub fn display(&self) {
        // Error header
        eprintln!("{} {}", "error:".red().bold(), self.message.bold());
        
        // File location
        if let (Some(file), Some(span)) = (&self.file, &self.span) {
            eprintln!("  {} {}:{}:{}", 
                "-->".blue().bold(), 
                file,
                span.line,
                span.col
            );
        }
        
        // Source code with span
        if let (Some(source), Some(span)) = (&self.source, &self.span) {
            eprintln!();
            self.display_source_with_span(source, span);
        }
        
        // Suggestion
        if let Some(suggestion) = &self.suggestion {
            eprintln!();
            eprintln!("{} {}", "suggestion:".green().bold(), suggestion);
        }
        
        // Help
        if let Some(help) = &self.help {
            eprintln!();
            eprintln!("{} {}", "help:".cyan().bold(), help);
        }
    }
    
    fn display_source_with_span(&self, source: &str, span: &Span) {
        let lines: Vec<&str> = source.lines().collect();
        
        // Get the line index (0-based)
        let line_idx = if span.line > 0 { span.line - 1 } else { 0 };
        
        if line_idx >= lines.len() {
            return;
        }
        
        // Calculate line number width for alignment
        let max_line = (span.line + 2).min(lines.len());
        let line_num_width = max_line.to_string().len();
        
        // Show context: 2 lines before and after
        let start = line_idx.saturating_sub(2);
        let end = (line_idx + 3).min(lines.len());
        
        for i in start..end {
            let line_num = i + 1;
            let line = lines.get(i).unwrap_or(&"");
            
            if line_num == span.line {
                // Error line
                eprintln!("{:>width$} {} {}", 
                    line_num.to_string().blue().bold(), 
                    "|".blue().bold(),
                    line,
                    width = line_num_width
                );
                
                // Error indicator with caret
                let spaces = " ".repeat(span.col.saturating_sub(1));
                let caret_len = if span.end > span.start {
                    (span.end - span.start).max(1)
                } else {
                    1
                };
                let carets = "^".repeat(caret_len);
                eprintln!("{:>width$} {} {}{}", 
                    "",
                    "|".blue().bold(),
                    spaces,
                    carets.red().bold(),
                    width = line_num_width
                );
            } else {
                // Context lines
                eprintln!("{:>width$} {} {}", 
                    line_num.to_string().dimmed(), 
                    "|".blue().bold(),
                    line,
                    width = line_num_width
                );
            }
        }
    }
}

impl fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Debug for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EnhancedError: {}", self.message)
    }
}

impl std::error::Error for EnhancedError {}

/// Convert anyhow::Error to EnhancedError with suggestions
pub fn enhance_error(err: anyhow::Error, file: Option<String>, source: Option<String>) -> EnhancedError {
    let message = err.to_string();
    
    let mut enhanced = EnhancedError::new(message.clone());
    
    if let Some(file) = file {
        enhanced = enhanced.with_file(file);
    }
    
    if let Some(source) = source {
        enhanced = enhanced.with_source(source);
    }
    
    // Add contextual suggestions based on error type
    if message.contains("Variable") && message.contains("not found") {
        enhanced = enhanced.with_suggestion("Check variable name spelling or declare it with 'let'");
        enhanced = enhanced.with_help("Available built-in variables: git, env");
    } else if message.contains("Macro") && message.contains("not defined") {
        enhanced = enhanced.with_suggestion("Define the macro with 'macro name(params) { ... }' or import it");
        enhanced = enhanced.with_help("Use 'use \"@namespace/package\"' to import external macros");
    } else if message.contains("Cannot iterate") {
        enhanced = enhanced.with_suggestion("Ensure the value is an array or git.files/git.staged");
        enhanced = enhanced.with_help("Use 'foreach item in array { ... }' syntax");
    } else if message.contains("Expected") && message.contains("got") {
        enhanced = enhanced.with_suggestion("Check syntax - missing semicolon, bracket, or keyword?");
    } else if message.contains("command not found") {
        enhanced = enhanced.with_suggestion("Verify the command is installed and in your PATH");
        enhanced = enhanced.with_help("Use 'which <command>' to check if command exists");
    } else if message.contains("Config file") && message.contains("not") {
        enhanced = enhanced.with_suggestion("Create a hook file with 'touch .githook/pre-commit.ghook'");
        enhanced = enhanced.with_help("Hook files should end with .ghook extension");
    }
    
    enhanced
}
