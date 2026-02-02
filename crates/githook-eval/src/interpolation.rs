
use crate::executor::Executor;
use crate::value::Value;
use anyhow::Result;

impl Executor {
    pub fn interpolate_string(&self, template: &str) -> Result<String> {
        let mut result = template.to_string();
        
        for (name, value) in &self.variables {
            let placeholder = format!("{{{}}}", name);
            if result.contains(&placeholder) {
                let replacement = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    Value::Array(arr) => {
                        let strings: Vec<String> = arr
                            .iter()
                            .map(|v| match v {
                                Value::String(s) => s.clone(),
                                other => format!("{:?}", other),
                            })
                            .collect();
                        strings.join(", ")
                    }
                    Value::Object(obj) => {
                        if let Some(path_ctx) = &obj.path_context {
                            path_ctx.to_string()
                        } else if let Some(Value::String(s)) = obj.properties.get("name") {
                            s.clone()
                        } else {
                            format!("{}({})", obj.type_name, obj.properties.len())
                        }
                    }
                };
                result = result.replace(&placeholder, &replacement);
            }
        }
        
        Ok(result)
    }
}
