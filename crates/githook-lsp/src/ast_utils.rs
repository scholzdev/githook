use githook_syntax::ast::Statement;
use githook_syntax::error::Span;

pub fn extract_imports(statements: &[Statement]) -> Vec<ImportInfo> {
    let mut imports = Vec::new();
    extract_imports_recursive(statements, &mut imports);
    imports
}

fn extract_imports_recursive(statements: &[Statement], imports: &mut Vec<ImportInfo>) {
    for stmt in statements {
        match stmt {
            Statement::Import { path, alias, .. } => {
                imports.push(ImportInfo {
                    path: path.clone(),
                    alias: alias.clone(),
                });
            }
            Statement::Group { body, .. } | Statement::MacroDef { body, .. } => {
                extract_imports_recursive(body, imports);
            }
            Statement::ForEach { body, .. } => {
                extract_imports_recursive(body, imports);
            }
            Statement::If {
                then_body,
                else_body,
                ..
            } => {
                extract_imports_recursive(then_body, imports);
                if let Some(else_b) = else_body {
                    extract_imports_recursive(else_b, imports);
                }
            }
            _ => {}
        }
    }
}

pub fn extract_macros(statements: &[Statement]) -> Vec<MacroInfo> {
    let mut macros = Vec::new();
    extract_macros_recursive(statements, &mut macros);
    macros
}

fn extract_macros_recursive(statements: &[Statement], macros: &mut Vec<MacroInfo>) {
    for stmt in statements {
        match stmt {
            Statement::MacroDef { name, params, span, .. } => {
                macros.push(MacroInfo {
                    name: name.clone(),
                    params: params.iter().cloned().collect(),
                    span: *span,
                });
            }
            Statement::Group { body, .. } => {
                extract_macros_recursive(body, macros);
            }
            Statement::ForEach { body, .. } => {
                extract_macros_recursive(body, macros);
            }
            Statement::If {
                then_body,
                else_body,
                ..
            } => {
                extract_macros_recursive(then_body, macros);
                if let Some(else_b) = else_body {
                    extract_macros_recursive(else_b, macros);
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub path: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MacroInfo {
    pub name: String,
    pub params: Vec<String>,
    pub span: Span,
}
