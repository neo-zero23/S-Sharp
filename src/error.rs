use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum SSharpError {
    LexError {
        message: String,
        line: usize,
        column: usize,
    },
    ParseError {
        message: String,
        line: usize,
        column: usize,
    },
    RuntimeError {
        message: String,
    },
}

impl fmt::Display for SSharpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SSharpError::LexError { message, line, column } => {
                write!(f, "Lex Error at line {}, column {}: {}", line, column, message)
            }
            SSharpError::ParseError { message, line, column } => {
                write!(f, "Parse Error at line {}, column {}: {}", line, column, message)
            }
            SSharpError::RuntimeError { message } => {
                write!(f, "Runtime Error: {}", message)
            }
        }
    }
}

impl std::error::Error for SSharpError {}
