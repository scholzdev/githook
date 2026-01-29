// Redesigned GitHook Language System
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod error;

// Re-exports for convenience
pub use lexer::{Token, SpannedToken, tokenize};
pub use parser::parse;
pub use ast::*;
pub use error::{Span, LexError, ParseError, Diagnostic};