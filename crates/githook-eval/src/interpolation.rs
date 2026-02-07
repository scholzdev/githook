use crate::executor::Executor;
use anyhow::Result;

impl Executor {
    pub fn interpolate_string(&mut self, template: &str) -> Result<String> {
        let mut result = template.to_string();

        for (name, value) in &self.variables {
            let replacement = value.display();

            // Replace both `${name}` and `{name}` forms
            let dollar_placeholder = format!("${{{}}}", name);
            if result.contains(&dollar_placeholder) {
                result = result.replace(&dollar_placeholder, &replacement);
            }
            let bare_placeholder = format!("{{{}}}", name);
            if result.contains(&bare_placeholder) {
                result = result.replace(&bare_placeholder, &replacement);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;

    #[test]
    fn test_interpolate_no_placeholders() {
        let mut executor = Executor::new();
        assert_eq!(
            executor.interpolate_string("hello world").unwrap(),
            "hello world"
        );
    }

    #[test]
    fn test_interpolate_string_variable() {
        let mut executor = Executor::new();
        executor.set_variable("name".into(), Value::String("Alice".into()));
        assert_eq!(
            executor.interpolate_string("hello {name}").unwrap(),
            "hello Alice"
        );
    }

    #[test]
    fn test_interpolate_number_variable() {
        let mut executor = Executor::new();
        executor.set_variable("count".into(), Value::Number(42.0));
        assert_eq!(
            executor.interpolate_string("{count} items").unwrap(),
            "42 items"
        );
    }

    #[test]
    fn test_interpolate_bool_variable() {
        let mut executor = Executor::new();
        executor.set_variable("flag".into(), Value::Bool(true));
        assert_eq!(executor.interpolate_string("is {flag}").unwrap(), "is true");
    }

    #[test]
    fn test_interpolate_null_variable() {
        let mut executor = Executor::new();
        executor.set_variable("val".into(), Value::Null);
        assert_eq!(executor.interpolate_string("{val}").unwrap(), "null");
    }

    #[test]
    fn test_interpolate_array_variable() {
        let mut executor = Executor::new();
        executor.set_variable(
            "files".into(),
            Value::Array(vec![
                Value::String("a.rs".into()),
                Value::String("b.rs".into()),
            ]),
        );
        assert_eq!(
            executor.interpolate_string("{files}").unwrap(),
            "a.rs, b.rs"
        );
    }

    #[test]
    fn test_interpolate_multiple_variables() {
        let mut executor = Executor::new();
        executor.set_variable("first".into(), Value::String("hello".into()));
        executor.set_variable("second".into(), Value::String("world".into()));
        assert_eq!(
            executor.interpolate_string("{first} {second}").unwrap(),
            "hello world"
        );
    }

    #[test]
    fn test_interpolate_missing_variable_unchanged() {
        let mut executor = Executor::new();
        assert_eq!(
            executor.interpolate_string("hello {unknown}").unwrap(),
            "hello {unknown}"
        );
    }

    #[test]
    fn test_interpolate_repeated_placeholder() {
        let mut executor = Executor::new();
        executor.set_variable("x".into(), Value::String("!".into()));
        assert_eq!(executor.interpolate_string("{x}{x}{x}").unwrap(), "!!!");
    }

    #[test]
    fn test_interpolate_dollar_brace_form() {
        let mut executor = Executor::new();
        executor.set_variable("count".into(), Value::Number(42.0));
        assert_eq!(
            executor
                .interpolate_string("Large commit: ${count} files")
                .unwrap(),
            "Large commit: 42 files"
        );
    }

    #[test]
    fn test_interpolate_dollar_brace_no_leftover_dollar() {
        let mut executor = Executor::new();
        executor.set_variable("name".into(), Value::String("main".into()));
        let result = executor
            .interpolate_string("branch ${name} is protected")
            .unwrap();
        assert_eq!(result, "branch main is protected");
        assert!(!result.contains('$'));
    }

    #[test]
    fn test_interpolate_mixed_forms() {
        let mut executor = Executor::new();
        executor.set_variable("a".into(), Value::String("hello".into()));
        executor.set_variable("b".into(), Value::String("world".into()));
        assert_eq!(
            executor.interpolate_string("${a} {b}").unwrap(),
            "hello world"
        );
    }
}
