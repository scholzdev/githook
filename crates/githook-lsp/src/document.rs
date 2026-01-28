use githook_syntax::{tokenize_with_spans, parse_spanned, Statement, ParseError};
use githook_syntax::error::Span;
use tower_lsp::lsp_types::Diagnostic;

/// Represents the state of a document in the LSP
pub struct DocumentState {
    /// Raw text content
    pub text: String,
    /// Parsed AST (if successful)
    pub ast: Option<Vec<Statement>>,
    /// Parse errors
    pub errors: Vec<ParseError>,
    /// Defined macro names
    pub macros: Vec<String>,
    /// Macro definitions with their spans and body (name -> (span, body))
    pub macro_definitions: Vec<(String, Span, Vec<Statement>)>,
    /// Imports: (alias, resolved_uri)
    pub imports: Vec<(String, String)>,
}

impl DocumentState {
    pub fn new(text: String, current_uri: Option<&str>) -> Self {
        let (ast, errors) = Self::parse(&text);
        let macros = Self::extract_macros(&ast);
        let macro_definitions = Self::extract_macro_definitions(&ast);
        let imports = Self::extract_imports(&ast, current_uri);
        Self { text, ast, errors, macros, macro_definitions, imports }
    }
    
    /// Extract all defined macro names from AST
    fn extract_macros(ast: &Option<Vec<Statement>>) -> Vec<String> {
        let mut macros = Vec::new();
        
        if let Some(statements) = ast {
            for stmt in statements {
                if let Statement::MacroDefinition { name, .. } = stmt {
                    macros.push(name.clone());
                }
            }
        }
        
        macros
    }
    
    /// Extract macro definitions with their spans and body
    fn extract_macro_definitions(ast: &Option<Vec<Statement>>) -> Vec<(String, Span, Vec<Statement>)> {
        let mut definitions = Vec::new();
        
        if let Some(statements) = ast {
            for stmt in statements {
                if let Statement::MacroDefinition { name, span, body, .. } = stmt {
                    definitions.push((name.clone(), *span, body.clone()));
                }
            }
        }
        
        definitions
    }
    
    /// Extract import statements: (alias, resolved_uri)
    fn extract_imports(ast: &Option<Vec<Statement>>, current_uri: Option<&str>) -> Vec<(String, String)> {
        use crate::import_resolver::{resolve_import_path, path_to_uri};
        
        let mut imports = Vec::new();
        
        if let Some(statements) = ast {
            for stmt in statements {
                if let Statement::Import { path, alias, .. } = stmt {
                    if let Some(alias_name) = alias {
                        // Resolve relative path to absolute URI
                        let resolved_uri = if let Some(uri) = current_uri {
                            if let Some(resolved_path) = resolve_import_path(uri, path) {
                                path_to_uri(&resolved_path)
                            } else {
                                path.clone() // fallback to relative path
                            }
                        } else {
                            path.clone() // fallback to relative path
                        };
                        
                        imports.push((alias_name.clone(), resolved_uri));
                    }
                }
            }
        }
        
        imports
    }

    fn parse(text: &str) -> (Option<Vec<Statement>>, Vec<ParseError>) {
        match tokenize_with_spans(text) {
            Ok(tokens) => match parse_spanned(tokens) {
                Ok(ast) => (Some(ast), vec![]),
                Err(e) => (None, vec![e]),
            },
            Err(lex_error) => {
                // Convert LexError to ParseError
                let parse_error = ParseError::LexError(lex_error);
                (None, vec![parse_error])
            }
        }
    }

    /// Get LSP diagnostics from parse errors
    pub fn diagnostics(&self) -> Option<Vec<Diagnostic>> {
        if self.errors.is_empty() {
            return None;
        }

        let diagnostics = self.errors.iter().map(|error| {
            crate::diagnostics::parse_error_to_diagnostic(error, &self.text)
        }).collect();

        Some(diagnostics)
    }
}
