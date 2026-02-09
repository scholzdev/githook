//! Expression evaluation helpers.
//!
//! Extracted from the main executor module to keep file sizes manageable.

use anyhow::Result;

use crate::bail_span;
use crate::value::Value;
use githook_syntax::ast::{BinaryOp, Expression, UnaryOp};

use super::Executor;

impl Executor {
    /// Evaluates an expression and returns its runtime [`Value`].
    pub fn eval_expression(&mut self, expr: &Expression) -> Result<Value> {
        match expr {
            Expression::String(s, _) => Ok(Value::String(s.clone())),
            Expression::Number(n, _) => Ok(Value::Number(*n)),
            Expression::Bool(b, _) => Ok(Value::Bool(*b)),
            Expression::Null(_) => Ok(Value::Null),

            Expression::Identifier(name, span) => match name.as_str() {
                "git" => Ok(self.create_git_object()),
                "env" => Ok(Value::env_object()),
                "http" => {
                    let http_ctx = crate::contexts::HttpContext::with_config(
                        self.config.http_timeout.as_secs(),
                        self.config.auth_token.clone(),
                    );
                    Ok(Value::Object(
                        crate::value::Object::new("Http").with_http_context(http_ctx),
                    ))
                }
                _ => self.variables.get(name).cloned().ok_or_else(|| {
                    anyhow::anyhow!(crate::error::EvalError::spanned(
                        format!("Variable '{}' not found", name),
                        span,
                    ))
                }),
            },

            Expression::PropertyAccess {
                receiver,
                property,
                span: _,
            } => {
                let obj = self.eval_expression(receiver)?;
                obj.get_property(property)
            }

            Expression::MethodCall {
                receiver,
                method,
                args,
                span,
            } => {
                if let Expression::Identifier(name, _) = receiver.as_ref()
                    && self.builtins.has(name)
                {
                    let arg_values: Result<Vec<Value>> =
                        args.iter().map(|a| self.eval_expression(a)).collect();
                    let arg_values = arg_values?;

                    if let Some(result) = self.builtins.call(name, &arg_values)? {
                        return Ok(result);
                    }
                }

                let obj_value = self.eval_expression(receiver)?;

                if matches!(method.as_str(), "filter" | "map" | "find" | "any" | "all")
                    && args.len() == 1
                    && let Expression::Closure { param, body, .. } = &args[0]
                {
                    return self.eval_closure_method(&obj_value, method, param, body, span);
                }

                let arg_values: Result<Vec<Value>> =
                    args.iter().map(|a| self.eval_expression(a)).collect();
                obj_value.call_method(method, &arg_values?)
            }

            Expression::Binary {
                left,
                op,
                right,
                span,
            } => {
                let left_val = self.eval_expression(left)?;
                let right_val = self.eval_expression(right)?;
                self.eval_binary_op(&left_val, *op, &right_val, span)
            }

            Expression::Unary { op, expr, span } => {
                let val = self.eval_expression(expr)?;
                self.eval_unary_op(*op, &val, span)
            }

            Expression::Array(elements, _) => {
                let mut values = Vec::with_capacity(elements.len());
                for e in elements {
                    values.push(self.eval_expression(e)?);
                }
                Ok(Value::Array(values))
            }

            Expression::InterpolatedString { parts, span: _ } => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        githook_syntax::ast::StringPart::Literal(s) => result.push_str(s),
                        githook_syntax::ast::StringPart::Expression(expr) => {
                            let val = self.eval_expression(expr)?;
                            result.push_str(&val.display());
                        }
                    }
                }
                Ok(Value::String(result))
            }

            Expression::IfExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                let cond = self.eval_expression(condition)?;
                if cond.is_truthy() {
                    self.eval_expression(then_expr)
                } else {
                    self.eval_expression(else_expr)
                }
            }

            Expression::Closure { span, .. } => {
                bail_span!(
                    span,
                    "Closures cannot be evaluated directly; they must be used as arguments to methods like filter() or map()"
                )
            }

            Expression::IndexAccess {
                receiver,
                index,
                span,
            } => {
                let obj = self.eval_expression(receiver)?;
                let idx = self.eval_expression(index)?;
                match (&obj, &idx) {
                    (Value::Object(o), Value::String(key)) => {
                        // First check context properties (e.g. HttpResponse json_parsed)
                        o.get(key).cloned().ok_or_else(|| {
                            anyhow::anyhow!(crate::error::EvalError::spanned(
                                format!("Key '{}' not found on {}", key, o.type_name),
                                span,
                            ))
                        })
                    }
                    (Value::Array(arr), Value::Number(n)) => {
                        let i = *n as usize;
                        arr.get(i).cloned().ok_or_else(|| {
                            anyhow::anyhow!(crate::error::EvalError::spanned(
                                format!("Index {} out of bounds (array length {})", i, arr.len()),
                                span,
                            ))
                        })
                    }
                    (Value::String(s), Value::Number(n)) => {
                        let i = *n as usize;
                        s.chars()
                            .nth(i)
                            .map(|c| Value::String(c.to_string()))
                            .ok_or_else(|| {
                                anyhow::anyhow!(crate::error::EvalError::spanned(
                                    format!(
                                        "Index {} out of bounds (string length {})",
                                        i,
                                        s.len()
                                    ),
                                    span,
                                ))
                            })
                    }
                    _ => {
                        bail_span!(span, "Cannot index {:?} with {:?}", obj, idx)
                    }
                }
            }
        }
    }

    pub(super) fn eval_binary_op(
        &mut self,
        left: &Value,
        op: BinaryOp,
        right: &Value,
        span: &githook_syntax::error::Span,
    ) -> Result<Value> {
        match op {
            BinaryOp::Eq => Ok(Value::Bool(left.equals(right)?)),
            BinaryOp::Ne => Ok(Value::Bool(left.not_equals(right)?)),
            BinaryOp::Lt => Ok(Value::Bool(left.less_than(right)?)),
            BinaryOp::Le => Ok(Value::Bool(left.less_or_equal(right)?)),
            BinaryOp::Gt => Ok(Value::Bool(left.greater_than(right)?)),
            BinaryOp::Ge => Ok(Value::Bool(left.greater_or_equal(right)?)),

            BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),

            BinaryOp::Add => match (left, right) {
                (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
                (Value::String(l), r) => Ok(Value::String(format!("{}{}", l, r.display()))),
                (l, Value::String(r)) => Ok(Value::String(format!("{}{}", l.display(), r))),
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                _ => bail_span!(span, "Cannot add {:?} and {:?}", left, right),
            },
            BinaryOp::Sub => match (left, right) {
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l - r)),
                _ => bail_span!(span, "Cannot subtract {:?} from {:?}", right, left),
            },
            BinaryOp::Mul => match (left, right) {
                (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
                _ => bail_span!(span, "Cannot multiply {:?} and {:?}", left, right),
            },
            BinaryOp::Div => match (left, right) {
                (Value::Number(l), Value::Number(r)) => {
                    if *r == 0.0 {
                        bail_span!(span, "Division by zero");
                    }
                    Ok(Value::Number(l / r))
                }
                _ => bail_span!(span, "Cannot divide {:?} by {:?}", left, right),
            },
            BinaryOp::Mod => match (left, right) {
                (Value::Number(l), Value::Number(r)) => {
                    if *r == 0.0 {
                        bail_span!(span, "Modulo by zero");
                    }
                    Ok(Value::Number(l % r))
                }
                _ => bail_span!(span, "Cannot modulo {:?} by {:?}", left, right),
            },
        }
    }

    pub(super) fn eval_unary_op(
        &mut self,
        op: UnaryOp,
        operand: &Value,
        span: &githook_syntax::error::Span,
    ) -> Result<Value> {
        match op {
            UnaryOp::Not => Ok(Value::Bool(!operand.is_truthy())),
            UnaryOp::Minus => match operand {
                Value::Number(n) => Ok(Value::Number(-n)),
                _ => bail_span!(span, "Cannot negate {:?}", operand),
            },
        }
    }

    pub(super) fn eval_let_value(
        &mut self,
        value: &githook_syntax::ast::LetValue,
    ) -> Result<Value> {
        use githook_syntax::ast::LetValue;
        match value {
            LetValue::String(s) => Ok(Value::String(s.clone())),
            LetValue::Number(n) => Ok(Value::Number(*n)),
            LetValue::Array(arr) => {
                let vals: Vec<Value> = arr.iter().map(|s| Value::String(s.clone())).collect();
                Ok(Value::Array(vals))
            }
            LetValue::Expression(expr) => self.eval_expression(expr),
        }
    }

    pub(super) fn eval_closure_method(
        &mut self,
        obj: &Value,
        method: &str,
        param: &str,
        body: &Expression,
        span: &githook_syntax::error::Span,
    ) -> Result<Value> {
        let items = match obj {
            Value::Array(arr) => arr,
            _ => bail_span!(
                span,
                "Cannot call closure method '{}' on non-array value",
                method
            ),
        };

        // Save original value (if any) so we can restore after the loop.
        let saved = self.variables.remove(param);

        let result = (|| -> Result<Value> {
            match method {
                "filter" => {
                    let mut result = Vec::new();
                    for item in items {
                        self.variables.insert(param.to_string(), item.clone());
                        if self.eval_expression(body)?.is_truthy() {
                            result.push(item.clone());
                        }
                    }
                    Ok(Value::Array(result))
                }
                "map" => {
                    let mut result = Vec::with_capacity(items.len());
                    for item in items {
                        self.variables.insert(param.to_string(), item.clone());
                        result.push(self.eval_expression(body)?);
                    }
                    Ok(Value::Array(result))
                }
                "find" => {
                    for item in items {
                        self.variables.insert(param.to_string(), item.clone());
                        if self.eval_expression(body)?.is_truthy() {
                            return Ok(item.clone());
                        }
                    }
                    Ok(Value::Null)
                }
                "any" => {
                    for item in items {
                        self.variables.insert(param.to_string(), item.clone());
                        if self.eval_expression(body)?.is_truthy() {
                            return Ok(Value::Bool(true));
                        }
                    }
                    Ok(Value::Bool(false))
                }
                "all" => {
                    for item in items {
                        self.variables.insert(param.to_string(), item.clone());
                        if !self.eval_expression(body)?.is_truthy() {
                            return Ok(Value::Bool(false));
                        }
                    }
                    Ok(Value::Bool(true))
                }
                _ => bail_span!(span, "Unknown closure method: {}", method),
            }
        })();

        // Restore the original variable (or remove the closure param).
        if let Some(original) = saved {
            self.variables.insert(param.to_string(), original);
        } else {
            self.variables.remove(param);
        }

        result
    }
}
