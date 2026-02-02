use crate::value::Value;
use anyhow::{Result, bail};
use githook_syntax::ast::BinaryOp;

pub fn eval_binary_op(left: &Value, op: BinaryOp, right: &Value) -> Result<Value> {
    match op {
        BinaryOp::Add => eval_add(left, right),
        BinaryOp::Sub => eval_sub(left, right),
        BinaryOp::Mul => eval_mul(left, right),
        BinaryOp::Div => eval_div(left, right),
        BinaryOp::Mod => eval_mod(left, right),
        BinaryOp::Eq => Ok(Value::Bool(values_equal(left, right))),
        BinaryOp::Ne => Ok(Value::Bool(!values_equal(left, right))),
        BinaryOp::Lt => eval_less(left, right),
        BinaryOp::Gt => eval_greater(left, right),
        BinaryOp::Le => eval_less_eq(left, right),
        BinaryOp::Ge => eval_greater_eq(left, right),
        BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
        BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
    }
}

fn eval_add(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
        (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
        (Value::String(l), Value::Number(r)) => Ok(Value::String(format!("{}{}", l, r))),
        (Value::Number(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
        _ => bail!("Cannot add {:?} and {:?}", left, right),
    }
}

fn eval_sub(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l - r)),
        _ => bail!("Cannot subtract {:?} from {:?}", right, left),
    }
}

fn eval_mul(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
        _ => bail!("Cannot multiply {:?} and {:?}", left, right),
    }
}

fn eval_div(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => {
            if *r == 0.0 {
                bail!("Division by zero");
            }
            Ok(Value::Number(l / r))
        }
        _ => bail!("Cannot divide {:?} by {:?}", left, right),
    }
}

fn eval_mod(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => {
            if *r == 0.0 {
                bail!("Modulo by zero");
            }
            Ok(Value::Number(l % r))
        }
        _ => bail!("Cannot modulo {:?} by {:?}", left, right),
    }
}

fn eval_less(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Bool(l < r)),
        (Value::String(l), Value::String(r)) => Ok(Value::Bool(l < r)),
        _ => bail!("Cannot compare {:?} < {:?}", left, right),
    }
}

fn eval_greater(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Bool(l > r)),
        (Value::String(l), Value::String(r)) => Ok(Value::Bool(l > r)),
        _ => bail!("Cannot compare {:?} > {:?}", left, right),
    }
}

fn eval_less_eq(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Bool(l <= r)),
        (Value::String(l), Value::String(r)) => Ok(Value::Bool(l <= r)),
        _ => bail!("Cannot compare {:?} <= {:?}", left, right),
    }
}

fn eval_greater_eq(left: &Value, right: &Value) -> Result<Value> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Bool(l >= r)),
        (Value::String(l), Value::String(r)) => Ok(Value::Bool(l >= r)),
        _ => bail!("Cannot compare {:?} >= {:?}", left, right),
    }
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => (l - r).abs() < f64::EPSILON,
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}
