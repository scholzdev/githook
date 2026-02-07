//! # githook-syntax
//!
//! Lexer, parser, AST definitions, and error types for the Githook
//! scripting language (`.ghook` files).
//!
//! This crate turns source text into a sequence of [`Statement`] nodes
//! that can be evaluated by [`githook_eval`].

/// Abstract syntax tree node types.
pub mod ast;
/// Error types, spans, and diagnostic formatting.
pub mod error;
/// Tokenizer / lexical analysis.
pub mod lexer;
/// Recursive-descent parser.
pub mod parser;

pub use ast::*;
pub use error::{Diagnostic, LexError, ParseError, Span};
pub use lexer::{SpannedToken, Token, tokenize};
pub use parser::parse;
