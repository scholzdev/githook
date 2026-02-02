//! # GitHook Syntax
//!
//! Lexer, parser, and Abstract Syntax Tree (AST) definitions for the GitHook DSL.
//!
//! ## Overview
//!
//! This crate provides the foundational components for parsing GitHook scripts:
//!
//! - **Lexer**: Tokenizes source code into a stream of tokens
//! - **Parser**: Builds an Abstract Syntax Tree from tokens using recursive descent
//! - **AST**: Type-safe representation of GitHook programs
//! - **Error Handling**: Detailed error messages with source location information
//!
//! ## Architecture
//!
//! ```text
//! Source Code
//!     ↓
//! Lexer (tokenize)
//!     ↓
//! Vec<Token>
//!     ↓
//! Parser (parse)
//!     ↓
//! Vec<Statement> (AST)
//! ```
//!
//! ## Example
//!
//! ```rust
//! use githook_syntax::{tokenize, parse};
//!
//! let source = r#"
//!     if true {
//!         print "Hello, GitHook!"
//!     }
//! "#;
//!
//! // Tokenize the source code
//! let tokens = tokenize(source).expect("Tokenization failed");
//!
//! // Parse tokens into an AST
//! let statements = parse(tokens).expect("Parsing failed");
//!
//! assert_eq!(statements.len(), 1);
//! ```
//!
//! ## Grammar Overview
//!
//! ```text
//! Statement:
//!   - VariableDeclaration (let x = expr)
//!   - If/IfElse (if condition { ... })
//!   - Foreach (foreach item in collection { ... })
//!   - When (when condition { ... })
//!   - Print/Warn/Block (print "msg")
//!   - Run (run "command")
//!   - Assert (assert condition)
//!
//! Expression:
//!   - Literals (string, number, boolean, null)
//!   - Identifiers (variable names)
//!   - Binary Operations (+, -, *, /, ==, !=, <, >, etc.)
//!   - Property Access (obj.property)
//!   - Method Calls (obj.method(args))
//!   - Closures (|x| x > 5)
//! ```
//!
//! ## Error Handling
//!
//! The parser provides detailed error messages with source location information:
//!
//! ```rust
//! use githook_syntax::tokenize;
//!
//! let invalid = "let x = ";
//! match tokenize(invalid) {
//!     Err(diagnostic) => {
//!         // Diagnostic includes line, column, and helpful message
//!         println!("Error: {}", diagnostic.message);
//!     }
//!     Ok(_) => unreachable!(),
//! }
//! ```

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;

pub use ast::*;
pub use error::{Diagnostic, LexError, ParseError, Span};
pub use lexer::{SpannedToken, Token, tokenize};
pub use parser::parse;
