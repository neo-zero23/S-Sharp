use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Str(String),
    Bool(bool),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Str(s) => s.trim().parse::<f64>().ok(),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
        }
    }
}
