use std::collections::HashMap;
use crate::error::SSharpError;
use crate::interpreter::value::Value;

#[derive(Debug, Clone, Default)]
pub struct Environment {
    vars: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: impl Into<String>, value: Value) {
        self.vars.insert(name.into(), value);
    }

    pub fn get(&self, name: &str) -> Result<Value, SSharpError> {
        self.vars.get(name).cloned().ok_or_else(|| {
            SSharpError::RuntimeError {
                message: format!("Undefined variable '{}'", name),
            }
        })
    }
}
