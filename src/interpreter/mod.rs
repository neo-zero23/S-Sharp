pub mod environment;
pub mod value;

use std::io::{self, BufRead, Write};
use environment::Environment;
use value::Value;
use crate::error::SSharpError;
use crate::parser::ast::*;

pub struct Interpreter<R, W> {
    env: Environment,
    reader: R,
    writer: W,
}

impl Interpreter<io::BufReader<io::Stdin>, io::Stdout> {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            reader: io::BufReader::new(io::stdin()),
            writer: io::stdout(),
        }
    }
}

impl<R: BufRead, W: Write> Interpreter<R, W> {
    pub fn with_io(reader: R, writer: W) -> Self {
        Self {
            env: Environment::new(),
            reader,
            writer,
        }
    }

    pub fn interpret(&mut self, program: &Program) -> Result<(), SSharpError> {
        for stmt in &program.event.body {
            self.exec_stmt(stmt)?;
        }
        Ok(())
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<(), SSharpError> {
        match stmt {
            Stmt::Ask { prompt, target } => {
                write!(self.writer, "{}", prompt).map_err(|e| SSharpError::RuntimeError {
                    message: format!("IO Error writing prompt: {}", e),
                })?;
                self.writer.flush().map_err(|e| SSharpError::RuntimeError {
                    message: format!("IO Error flushing prompt: {}", e),
                })?;

                let mut line = String::new();
                self.reader.read_line(&mut line).map_err(|e| SSharpError::RuntimeError {
                    message: format!("IO Error reading input: {}", e),
                })?;

                let trimmed = line.trim();
                let val = if let Ok(num) = trimmed.parse::<f64>() {
                    Value::Number(num)
                } else {
                    Value::Str(trimmed.to_string())
                };

                if !target.is_empty() {
                    self.env.set(target, val);
                }
            }
            Stmt::Assign { value, target } => {
                let val = self.eval_expr(value)?;
                self.env.set(target, val);
            }
            Stmt::Display { value } => {
                let val = self.eval_expr(value)?;
                writeln!(self.writer, "{}", val).map_err(|e| SSharpError::RuntimeError {
                    message: format!("IO Error writing display output: {}", e),
                })?;
            }
            Stmt::If { condition, actions } => {
                let cond_val = self.eval_expr(condition)?;
                if cond_val.is_truthy() {
                    for action in actions {
                        self.exec_stmt(action)?;
                    }
                }
            }
            Stmt::Repeat { count, actions } => {
                let count_val = self.eval_expr(count)?;
                if let Some(n) = count_val.as_number() {
                    let times = n.max(0.0) as usize;
                    for _ in 0..times {
                        for action in actions {
                            self.exec_stmt(action)?;
                        }
                    }
                } else {
                    return Err(SSharpError::RuntimeError {
                        message: format!("Repeat count must be a number, got '{}'", count_val),
                    });
                }
            }
            Stmt::While { condition, actions } => {
                while self.eval_expr(condition)?.is_truthy() {
                    for action in actions {
                        self.exec_stmt(action)?;
                    }
                }
            }
            Stmt::FunctionDef { .. } => {
                // Reserved for function definitions in future milestones
            }
        }
        Ok(())
    }

    fn eval_expr(&self, expr: &Expr) -> Result<Value, SSharpError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Identifier(id) => self.env.get(id),
            Expr::Binary { left, op, right } => {
                let l_val = self.eval_expr(left)?;
                let r_val = self.eval_expr(right)?;
                self.eval_binary_op(&l_val, *op, &r_val)
            }
        }
    }

    fn eval_binary_op(&self, left: &Value, op: BinOp, right: &Value) -> Result<Value, SSharpError> {
        match op {
            BinOp::Add => {
                if matches!(left, Value::Str(_)) || matches!(right, Value::Str(_)) {
                    Ok(Value::Str(format!("{}{}", left, right)))
                } else if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Number(l + r))
                } else {
                    Err(SSharpError::RuntimeError {
                        message: format!("Cannot add '{}' and '{}'", left, right),
                    })
                }
            }
            BinOp::Sub => {
                let l = self.require_number(left, "-")?;
                let r = self.require_number(right, "-")?;
                Ok(Value::Number(l - r))
            }
            BinOp::Mul => {
                let l = self.require_number(left, "*")?;
                let r = self.require_number(right, "*")?;
                Ok(Value::Number(l * r))
            }
            BinOp::Div => {
                let l = self.require_number(left, "/")?;
                let r = self.require_number(right, "/")?;
                if r == 0.0 {
                    return Err(SSharpError::RuntimeError {
                        message: "Division by zero".to_string(),
                    });
                }
                Ok(Value::Number(l / r))
            }
            BinOp::Eq => {
                if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool((l - r).abs() < f64::EPSILON))
                } else {
                    Ok(Value::Bool(left.to_string() == right.to_string()))
                }
            }
            BinOp::Gt => {
                if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool(l > r))
                } else {
                    Ok(Value::Bool(left.to_string() > right.to_string()))
                }
            }
            BinOp::Lt => {
                if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool(l < r))
                } else {
                    Ok(Value::Bool(left.to_string() < right.to_string()))
                }
            }
            BinOp::GtEq => {
                if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool(l >= r))
                } else {
                    Ok(Value::Bool(left.to_string() >= right.to_string()))
                }
            }
            BinOp::LtEq => {
                if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool(l <= r))
                } else {
                    Ok(Value::Bool(left.to_string() <= right.to_string()))
                }
            }
        }
    }

    fn require_number(&self, val: &Value, op: &str) -> Result<f64, SSharpError> {
        val.as_number().ok_or_else(|| SSharpError::RuntimeError {
            message: format!("Operator '{}' requires numeric operand, got '{}'", op, val),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use std::io::Cursor;

    fn run_ssharp(source: &str, input: &str) -> Result<String, SSharpError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;

        let reader = Cursor::new(input.as_bytes().to_vec());
        let mut writer = Vec::new();

        let mut interpreter = Interpreter::with_io(reader, &mut writer);
        interpreter.interpret(&program)?;

        Ok(String::from_utf8(writer).unwrap())
    }

    #[test]
    fn test_primary_milestone_accepted() {
        let source = r#"
            when (start_clicked).
            ask "How old are you?" and save to age.
            if (age >= 18), display "Access granted".
            if (age < 18), display "Access denied".
        "#;

        let output = run_ssharp(source, "25\n").unwrap();
        assert!(output.contains("Access granted"));
        assert!(!output.contains("Access denied"));
    }

    #[test]
    fn test_primary_milestone_rejected() {
        let source = r#"
            when (start_clicked).
            ask "How old are you?" and save to age.
            if (age >= 18), display "Access granted".
            if (age < 18), display "Access denied".
        "#;

        let output = run_ssharp(source, "15\n").unwrap();
        assert!(output.contains("Access denied"));
        assert!(!output.contains("Access granted"));
    }

    #[test]
    fn test_repeat_loop() {
        let source = r#"
            when (start_clicked).
            repeat (3), display "Hello".
        "#;

        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "Hello\nHello\nHello\n");
    }
}
