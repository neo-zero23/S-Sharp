pub mod environment;
pub mod value;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use environment::Environment;
use value::Value;
use crate::error::SSharpError;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::ast::*;

/// Maximum bytes read from a single file via `read` (10 MiB).
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone)]
struct Function {
    params: Vec<String>,
    body: Vec<Stmt>,
    return_expr: Expr,
}

/// Control-flow signal produced by `break` / `continue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Normal,
    Break,
    Continue,
}

pub struct Interpreter<R, W> {
    env: Environment,
    functions: HashMap<String, Function>,
    reader: R,
    writer: W,
    base_dir: PathBuf,
    imported: HashSet<PathBuf>,
}

impl Interpreter<io::BufReader<io::Stdin>, io::Stdout> {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            functions: HashMap::new(),
            reader: io::BufReader::new(io::stdin()),
            writer: io::stdout(),
            base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            imported: HashSet::new(),
        }
    }
}

impl<R: BufRead, W: Write> Interpreter<R, W> {
    pub fn with_io(reader: R, writer: W) -> Self {
        Self {
            env: Environment::new(),
            functions: HashMap::new(),
            reader,
            writer,
            base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            imported: HashSet::new(),
        }
    }

    /// Sets the directory that `import`, `read` and `write` resolve
    /// relative paths against (defaults to the process working directory).
    pub fn set_base_dir(&mut self, dir: PathBuf) {
        self.base_dir = dir;
    }

    /// Marks a file as already imported so `import` skips it
    /// (used for the entry script itself, plus cycle protection).
    pub fn mark_imported(&mut self, path: PathBuf) {
        self.imported.insert(path);
    }

    pub fn interpret(&mut self, program: &Program) -> Result<(), SSharpError> {
        match self.exec_block(&program.event.body)? {
            Flow::Normal => Ok(()),
            Flow::Break => Err(SSharpError::RuntimeError {
                message: "'break' outside of a loop".to_string(),
            }),
            Flow::Continue => Err(SSharpError::RuntimeError {
                message: "'continue' outside of a loop".to_string(),
            }),
        }
    }

    /// Runs a sequence of statements, stopping early on the first
    /// non-`Normal` flow signal (`break` / `continue` / error).
    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Flow, SSharpError> {
        for stmt in stmts {
            match self.exec_stmt(stmt)? {
                Flow::Normal => {}
                flow => return Ok(flow),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow, SSharpError> {
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
                let val = if let Ok(n) = trimmed.parse::<i64>() {
                    Value::Int(n)
                } else if let Ok(num) = trimmed.parse::<f64>() {
                    Value::Float(num)
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
            Stmt::If { condition, actions, else_actions } => {
                let cond_val = self.eval_expr(condition)?;
                if cond_val.is_truthy() {
                    return self.exec_block(actions);
                } else if let Some(else_branch) = else_actions {
                    return self.exec_block(else_branch);
                }
            }
            Stmt::Repeat { count, actions } => {
                let count_val = self.eval_expr(count)?;
                if let Some(n) = count_val.as_number() {
                    let times = n.max(0.0) as usize;
                    for _ in 0..times {
                        for action in actions {
                            match self.exec_stmt(action)? {
                                Flow::Normal => {}
                                Flow::Break => return Ok(Flow::Normal),
                                Flow::Continue => break,
                            }
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
                        match self.exec_stmt(action)? {
                            Flow::Normal => {}
                            Flow::Break => return Ok(Flow::Normal),
                            Flow::Continue => break,
                        }
                    }
                }
            }
            Stmt::ForEach { var, iterable, actions } => {
                let collection = self.eval_expr(iterable)?;
                let items: Vec<Value> = match collection {
                    Value::List(items) => items,
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    other => {
                        return Err(SSharpError::RuntimeError {
                            message: format!("'for each ... in ...' expects a list or a string, got '{}'", other),
                        });
                    }
                };
                for item in items {
                    self.env.set(var, item);
                    for action in actions {
                        match self.exec_stmt(action)? {
                            Flow::Normal => {}
                            Flow::Break => return Ok(Flow::Normal),
                            Flow::Continue => break,
                        }
                    }
                }
            }
            Stmt::AddTo { value, target } => {
                let val = self.eval_expr(value)?;
                let mut list = self.env.get(target)?;
                match &mut list {
                    Value::List(items) => items.push(val),
                    other => {
                        return Err(SSharpError::RuntimeError {
                            message: format!("'add ... to ...' expects a list in '{}', got '{}'", target, other),
                        });
                    }
                }
                self.env.set(target, list);
            }
            Stmt::ChangeItem { target, index, value } => {
                let index_val = self.eval_expr(index)?;
                let val = self.eval_expr(value)?;
                let mut list = self.env.get(target)?;
                match &mut list {
                    Value::List(items) => {
                        let pos = Self::to_list_position(&index_val, items.len())?;
                        items[pos] = val;
                    }
                    other => {
                        return Err(SSharpError::RuntimeError {
                            message: format!("'change item ... of ...' expects a list in '{}', got '{}'", target, other),
                        });
                    }
                }
                self.env.set(target, list);
            }
            Stmt::RemoveItem { target, index } => {
                let index_val = self.eval_expr(index)?;
                let mut list = self.env.get(target)?;
                match &mut list {
                    Value::List(items) => {
                        let pos = Self::to_list_position(&index_val, items.len())?;
                        items.remove(pos);
                    }
                    other => {
                        return Err(SSharpError::RuntimeError {
                            message: format!("'remove item ... of ...' expects a list in '{}', got '{}'", target, other),
                        });
                    }
                }
                self.env.set(target, list);
            }
            Stmt::FunctionDef { name, params, body, return_expr } => {
                self.functions.insert(
                    name.clone(),
                    Function {
                        params: params.clone(),
                        body: body.clone(),
                        return_expr: return_expr.clone(),
                    },
                );
            }
            Stmt::Try { actions, error_var, catch_actions } => {
                match self.exec_block(actions) {
                    Ok(Flow::Normal) => {}
                    Ok(flow) => return Ok(flow), // break/continue pass through, not caught
                    Err(SSharpError::RuntimeError { message }) => {
                        if let Some(var) = error_var {
                            self.env.set(var, Value::Str(message));
                        }
                        return self.exec_block(catch_actions);
                    }
                    // Lex/parse errors (e.g. from a broken import) are
                    // programmer errors, not catchable runtime failures.
                    Err(other) => return Err(other),
                }
            }
            Stmt::Import { path } => {
                let path_val = self.eval_expr(path)?;
                let rel = match &path_val {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(SSharpError::RuntimeError {
                            message: format!("'import' expects a string path, got '{}'", other),
                        });
                    }
                };
                let canonical = self
                    .base_dir
                    .join(&rel)
                    .canonicalize()
                    .map_err(|e| SSharpError::RuntimeError {
                        message: format!("Cannot import '{}': {}", rel, e),
                    })?;
                if self.imported.contains(&canonical) {
                    return Ok(Flow::Normal); // already imported: skip (also breaks cycles)
                }
                let source = fs::read_to_string(&canonical).map_err(|e| SSharpError::RuntimeError {
                    message: format!("Cannot import '{}': {}", rel, e),
                })?;
                let mut lexer = Lexer::new(&source);
                let tokens = lexer.tokenize().map_err(|e| SSharpError::RuntimeError {
                    message: format!("Cannot import '{}': {}", rel, e),
                })?;
                let mut parser = Parser::new(tokens);
                let program = parser.parse().map_err(|e| SSharpError::RuntimeError {
                    message: format!("Cannot import '{}': {}", rel, e),
                })?;
                self.imported.insert(canonical);
                return self.exec_block(&program.event.body);
            }            Stmt::Read { path, target } => {
                let path_val = self.eval_expr(path)?;
                let rel = match &path_val {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(SSharpError::RuntimeError {
                            message: format!("'read' expects a string path, got '{}'", other),
                        });
                    }
                };
                let full = self.base_dir.join(&rel);
                let size = fs::metadata(&full).map_err(|e| SSharpError::RuntimeError {
                    message: format!("Cannot read file '{}': {}", full.display(), e),
                })?.len();
                if size > MAX_FILE_BYTES {
                    return Err(SSharpError::RuntimeError {
                        message: format!(
                            "Cannot read file '{}': size {} bytes exceeds the {} byte limit",
                            full.display(),
                            size,
                            MAX_FILE_BYTES
                        ),
                    });
                }
                let content = fs::read_to_string(&full).map_err(|e| SSharpError::RuntimeError {
                    message: format!("Cannot read file '{}': {}", full.display(), e),
                })?;
                if !target.is_empty() {
                    self.env.set(target, Value::Str(content));
                }
            }
            Stmt::Write { value, path } => {
                let val = self.eval_expr(value)?;
                let path_val = self.eval_expr(path)?;
                let rel = match &path_val {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(SSharpError::RuntimeError {
                            message: format!("'write ... to file' expects a string path, got '{}'", other),
                        });
                    }
                };
                let full = self.base_dir.join(&rel);
                fs::write(&full, val.to_string()).map_err(|e| SSharpError::RuntimeError {
                    message: format!("Cannot write file '{}': {}", full.display(), e),
                })?;
            }
            Stmt::Break => return Ok(Flow::Break),
            Stmt::Continue => return Ok(Flow::Continue),
        }
        Ok(Flow::Normal)
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, SSharpError> {
        match expr {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(n) => Ok(Value::Float(*n)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::List(elements) => {
                let mut values = Vec::with_capacity(elements.len());
                for el in elements {
                    values.push(self.eval_expr(el)?);
                }
                Ok(Value::List(values))
            }
            Expr::Identifier(id) => self.env.get(id),
            Expr::Call { name, args } => self.eval_call(name, args),
            Expr::Index { list, index } => self.eval_index(list, index),
            Expr::Unary { op, expr } => {
                let val = self.eval_expr(expr)?;
                match op {
                    UnOp::Not => Ok(Value::Bool(!val.is_truthy())),
                    UnOp::Neg => match val {
                        Value::Int(n) => n.checked_neg().map(Value::Int).ok_or_else(|| SSharpError::RuntimeError {
                            message: "Integer negation overflow".to_string(),
                        }),
                        Value::Float(n) => Ok(Value::Float(-n)),
                        other => Err(SSharpError::RuntimeError {
                            message: format!("Operator '-' requires a numeric operand, got '{}'", other),
                        }),
                    },
                }
            }
            Expr::Binary { left, op, right } => {
                // Short-circuit logical operators
                match op {
                    BinOp::Or => {
                        let l_val = self.eval_expr(left)?;
                        if l_val.is_truthy() {
                            return Ok(Value::Bool(true));
                        }
                        let r_val = self.eval_expr(right)?;
                        return Ok(Value::Bool(r_val.is_truthy()));
                    }
                    BinOp::And => {
                        let l_val = self.eval_expr(left)?;
                        if !l_val.is_truthy() {
                            return Ok(Value::Bool(false));
                        }
                        let r_val = self.eval_expr(right)?;
                        return Ok(Value::Bool(r_val.is_truthy()));
                    }
                    _ => {}
                }
                let l_val = self.eval_expr(left)?;
                let r_val = self.eval_expr(right)?;
                self.eval_binary_op(&l_val, *op, &r_val)
            }
        }
    }

    fn eval_call(&mut self, name: &str, args: &[Expr]) -> Result<Value, SSharpError> {
        // Built-in functions (checked before user-defined ones).
        if name == "len" {
            if args.len() != 1 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'len' expects 1 argument, got {}", args.len()),
                });
            }
            let val = self.eval_expr(&args[0])?;
            return match &val {
                Value::List(items) => Ok(Value::Int(items.len() as i64)),
                Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                _ => Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'len' expects a list or a string, got '{}'", val),
                }),
            };
        }
        if name == "range" {
            if args.len() != 1 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'range' expects 1 argument, got {}", args.len()),
                });
            }
            let val = self.eval_expr(&args[0])?;
            let n = match &val {
                Value::Int(n) if *n >= 0 => *n as usize,
                Value::Float(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
                _ => {
                    return Err(SSharpError::RuntimeError {
                        message: format!("Builtin 'range' expects a whole number >= 0, got '{}'", val),
                    });
                }
            };
            // 1-based to match `item i of xs`: range(5) is [1, 2, 3, 4, 5].
            return Ok(Value::List((1..=n).map(|i| Value::Int(i as i64)).collect()));
        }
        if name == "str" {
            if args.len() != 1 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'str' expects 1 argument, got {}", args.len()),
                });
            }
            let val = self.eval_expr(&args[0])?;
            return Ok(Value::Str(val.to_string()));
        }
        if name == "num" {
            if args.len() != 1 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'num' expects 1 argument, got {}", args.len()),
                });
            }
            let val = self.eval_expr(&args[0])?;
            return match &val {
                Value::Int(n) => Ok(Value::Int(*n)),
                Value::Float(n) => Ok(Value::Float(*n)),
                Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                Value::Str(s) => {
                    let t = s.trim();
                    if let Ok(n) = t.parse::<i64>() {
                        Ok(Value::Int(n))
                    } else if let Ok(n) = t.parse::<f64>() {
                        Ok(Value::Float(n))
                    } else {
                        Err(SSharpError::RuntimeError {
                            message: format!("Builtin 'num' cannot convert '{}' to a number", val),
                        })
                    }
                }
                Value::List(_) => Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'num' cannot convert '{}' to a number", val),
                }),
            };
        }
        if name == "upper" || name == "lower" {
            if args.len() != 1 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin '{}' expects 1 argument, got {}", name, args.len()),
                });
            }
            let val = self.eval_expr(&args[0])?;
            return match &val {
                Value::Str(s) => {
                    if name == "upper" {
                        Ok(Value::Str(s.to_uppercase()))
                    } else {
                        Ok(Value::Str(s.to_lowercase()))
                    }
                }
                _ => Err(SSharpError::RuntimeError {
                    message: format!("Builtin '{}' expects a string, got '{}'", name, val),
                }),
            };
        }
        if name == "split" {
            if args.len() != 2 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'split' expects 2 arguments (string, separator), got {}", args.len()),
                });
            }
            let text = self.eval_expr(&args[0])?;
            let sep = self.eval_expr(&args[1])?;
            return match (&text, &sep) {
                (Value::Str(s), Value::Str(d)) => {
                    if d.is_empty() {
                        return Err(SSharpError::RuntimeError {
                            message: "Builtin 'split' separator must not be empty".to_string(),
                        });
                    }
                    Ok(Value::List(s.split(d.as_str()).map(|p| Value::Str(p.to_string())).collect()))
                }
                _ => Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'split' expects (string, string), got ('{}', '{}')", text, sep),
                }),
            };
        }
        if name == "join" {
            if args.len() != 2 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'join' expects 2 arguments (list, separator), got {}", args.len()),
                });
            }
            let list = self.eval_expr(&args[0])?;
            let sep = self.eval_expr(&args[1])?;
            return match (&list, &sep) {
                (Value::List(items), Value::Str(d)) => {
                    let parts: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                    Ok(Value::Str(parts.join(d)))
                }
                _ => Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'join' expects (list, string), got ('{}', '{}')", list, sep),
                }),
            };
        }
        if name == "sqrt" {
            if args.len() != 1 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'sqrt' expects 1 argument, got {}", args.len()),
                });
            }
            let val = self.eval_expr(&args[0])?;
            let n = self.require_number(&val, "sqrt")?;
            if n < 0.0 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'sqrt' expects a non-negative number, got '{}'", val),
                });
            }
            return Ok(Value::Float(n.sqrt()));
        }
        if name == "type" {
            if args.len() != 1 {
                return Err(SSharpError::RuntimeError {
                    message: format!("Builtin 'type' expects 1 argument, got {}", args.len()),
                });
            }
            let val = self.eval_expr(&args[0])?;
            return Ok(Value::Str(
                match &val {
                    Value::Int(_) => "int",
                    Value::Float(_) => "float",
                    Value::Str(_) => "string",
                    Value::Bool(_) => "bool",
                    Value::List(_) => "list",
                }
                .to_string(),
            ));
        }
        let func = self.functions.get(name).cloned().ok_or_else(|| SSharpError::RuntimeError {
            message: format!("Undefined function '{}'", name),
        })?;
        if args.len() != func.params.len() {
            return Err(SSharpError::RuntimeError {
                message: format!(
                    "Function '{}' expects {} argument(s), got {}",
                    name,
                    func.params.len(),
                    args.len()
                ),
            });
        }
        let mut evaluated = Vec::with_capacity(args.len());
        for arg in args {
            evaluated.push(self.eval_expr(arg)?);
        }
        // Isolate function scope: save caller env, bind params, run body, eval return, restore
        let saved_env = self.env.clone();
        for (param, val) in func.params.iter().zip(evaluated) {
            self.env.set(param, val);
        }
        let result = (|| {
            match self.exec_block(&func.body)? {
                Flow::Normal => {}
                Flow::Break => {
                    return Err(SSharpError::RuntimeError {
                        message: format!("'break' in function '{}' is outside of a loop", name),
                    });
                }
                Flow::Continue => {
                    return Err(SSharpError::RuntimeError {
                        message: format!("'continue' in function '{}' is outside of a loop", name),
                    });
                }
            }
            self.eval_expr(&func.return_expr)
        })();
        self.env = saved_env;
        result
    }

    /// Converts a 1-based index value into a 0-based position, checking bounds.
    /// Accepts ints and whole-number floats.
    fn to_list_position(index_val: &Value, len: usize) -> Result<usize, SSharpError> {
        let position = match index_val {
            Value::Int(n) => *n,
            Value::Float(n) => {
                if n.fract() != 0.0 {
                    return Err(SSharpError::RuntimeError {
                        message: format!("List index must be a whole number, got '{}'", index_val),
                    });
                }
                *n as i64
            }
            _ => {
                return Err(SSharpError::RuntimeError {
                    message: format!("List index must be a number, got '{}'", index_val),
                });
            }
        };
        // Scratch-style 1-based indexing: `item 1 of xs` is the first element.
        if position < 1 || position as usize > len {
            return Err(SSharpError::RuntimeError {
                message: format!(
                    "List index out of bounds: index is {}, but the list has {} element(s) (valid: 1..={})",
                    position,
                    len,
                    len.max(1)
                ),
            });
        }
        Ok((position - 1) as usize)
    }

    fn eval_index(&mut self, list_expr: &Expr, index_expr: &Expr) -> Result<Value, SSharpError> {
        let collection = self.eval_expr(list_expr)?;
        let index_val = self.eval_expr(index_expr)?;
        match collection {
            Value::List(items) => {
                let pos = Self::to_list_position(&index_val, items.len())?;
                Ok(items[pos].clone())
            }
            Value::Str(s) => {
                let chars: Vec<char> = s.chars().collect();
                let pos = Self::to_list_position(&index_val, chars.len()).map_err(|_| SSharpError::RuntimeError {
                    message: format!(
                        "String index out of bounds: index is {}, but the string has {} character(s)",
                        index_val,
                        chars.len()
                    ),
                })?;
                Ok(Value::Str(chars[pos].to_string()))
            }
            other => Err(SSharpError::RuntimeError {
                message: format!(
                    "'item ... of ...' expects a list or a string, got '{}'",
                    other
                ),
            }),
        }
    }

    fn eval_binary_op(&self, left: &Value, op: BinOp, right: &Value) -> Result<Value, SSharpError> {
        match op {
            BinOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            BinOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinOp::Add => {
                if matches!(left, Value::Str(_)) || matches!(right, Value::Str(_)) {
                    Ok(Value::Str(format!("{}{}", left, right)))
                } else if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    l.checked_add(*r).map(Value::Int).ok_or_else(|| SSharpError::RuntimeError {
                        message: "Integer overflow in '+'".to_string(),
                    })
                } else if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Float(l + r))
                } else {
                    Err(SSharpError::RuntimeError {
                        message: format!("Cannot add '{}' and '{}'", left, right),
                    })
                }
            }
            BinOp::Concat => {
                // Explicit concatenation: always joins the text of both sides.
                // Unlike `+`, numbers are NOT added: `1 ++ 2` is "12", not 3.
                Ok(Value::Str(format!("{}{}", left, right)))
            }
            BinOp::Sub => {
                if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    return l.checked_sub(*r).map(Value::Int).ok_or_else(|| SSharpError::RuntimeError {
                        message: "Integer overflow in '-'".to_string(),
                    });
                }
                let l = self.require_number(left, "-")?;
                let r = self.require_number(right, "-")?;
                Ok(Value::Float(l - r))
            }
            BinOp::Mul => {
                if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    return l.checked_mul(*r).map(Value::Int).ok_or_else(|| SSharpError::RuntimeError {
                        message: "Integer overflow in '*'".to_string(),
                    });
                }
                let l = self.require_number(left, "*")?;
                let r = self.require_number(right, "*")?;
                Ok(Value::Float(l * r))
            }
            BinOp::Div => {
                // `/` is always float division: `7 / 2` is 3.5 (use `div` for 3).
                let l = self.require_number(left, "/")?;
                let r = self.require_number(right, "/")?;
                if r == 0.0 {
                    return Err(SSharpError::RuntimeError {
                        message: "Division by zero".to_string(),
                    });
                }
                Ok(Value::Float(l / r))
            }
            BinOp::DivInt => {
                // Truncating integer division: `7 div 2` is 3, `-7 div 2` is -3.
                let l = self.require_whole(left, "div")?;
                let r = self.require_whole(right, "div")?;
                if r == 0 {
                    return Err(SSharpError::RuntimeError {
                        message: "Division by zero".to_string(),
                    });
                }
                l.checked_div(r).map(Value::Int).ok_or_else(|| SSharpError::RuntimeError {
                    message: "Integer overflow in 'div'".to_string(),
                })
            }
            BinOp::Mod => {
                if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    if *r == 0 {
                        return Err(SSharpError::RuntimeError {
                            message: "Modulo by zero".to_string(),
                        });
                    }
                    return Ok(Value::Int(l % r));
                }
                let l = self.require_number(left, "%")?;
                let r = self.require_number(right, "%")?;
                if r == 0.0 {
                    return Err(SSharpError::RuntimeError {
                        message: "Modulo by zero".to_string(),
                    });
                }
                Ok(Value::Float(l % r))
            }
            BinOp::Eq => {
                if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    Ok(Value::Bool(l == r))
                } else if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool((l - r).abs() < f64::EPSILON))
                } else {
                    Ok(Value::Bool(left.to_string() == right.to_string()))
                }
            }
            BinOp::NotEq => {
                if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    Ok(Value::Bool(l != r))
                } else if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool((l - r).abs() >= f64::EPSILON))
                } else {
                    Ok(Value::Bool(left.to_string() != right.to_string()))
                }
            }
            BinOp::Gt => {
                if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    Ok(Value::Bool(l > r))
                } else if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool(l > r))
                } else {
                    Ok(Value::Bool(left.to_string() > right.to_string()))
                }
            }
            BinOp::Lt => {
                if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    Ok(Value::Bool(l < r))
                } else if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool(l < r))
                } else {
                    Ok(Value::Bool(left.to_string() < right.to_string()))
                }
            }
            BinOp::GtEq => {
                if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    Ok(Value::Bool(l >= r))
                } else if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
                    Ok(Value::Bool(l >= r))
                } else {
                    Ok(Value::Bool(left.to_string() >= right.to_string()))
                }
            }
            BinOp::LtEq => {
                if let (Value::Int(l), Value::Int(r)) = (left, right) {
                    Ok(Value::Bool(l <= r))
                } else if let (Some(l), Some(r)) = (left.as_number(), right.as_number()) {
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

    /// Requires a whole number for `div`, accepting ints and whole floats.
    fn require_whole(&self, val: &Value, op: &str) -> Result<i64, SSharpError> {
        match val {
            Value::Int(n) => Ok(*n),
            Value::Float(n) if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 => Ok(*n as i64),
            _ => Err(SSharpError::RuntimeError {
                message: format!("Operator '{}' requires a whole-number operand, got '{}'", op, val),
            }),
        }
    }
}

/// Interactive REPL driver.
///
/// Reads S# fragments from `reader` (each fragment must end with `.`),
/// keeps variables and functions alive between fragments, and writes
/// prompts plus program output to `writer`. Ends on EOF (Ctrl-D),
/// `:quit` or `:exit`.
pub fn run_repl<R: BufRead, W: Write>(reader: R, writer: W) {
    let mut interpreter = Interpreter::with_io(reader, writer);
    let version = env!("CARGO_PKG_VERSION");
    writeln!(interpreter.writer, "S# (ssharp) v{} -- interactive mode.", version).ok();
    writeln!(
        interpreter.writer,
        "Type any S# fragment ending with '.'. Commands: :help, :reset, :quit."
    )
    .ok();

    let mut buffer = String::new();
    loop {
        let prompt = if buffer.is_empty() { ">> " } else { ".. " };
        write!(interpreter.writer, "{}", prompt).ok();
        interpreter.writer.flush().ok();

        let mut line = String::new();
        match interpreter.reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                writeln!(interpreter.writer, "Input error: {}", e).ok();
                break;
            }
        }

        let trimmed = line.trim().to_string();
        if buffer.is_empty() {
            if trimmed == ":quit" || trimmed == ":exit" {
                break;
            }
            if trimmed == ":help" {
                write_repl_help(&mut interpreter.writer);
                continue;
            }
            if trimmed == ":reset" {
                interpreter.env = Environment::new();
                interpreter.functions.clear();
                writeln!(interpreter.writer, "Environment cleared.").ok();
                continue;
            }
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with(':') {
                writeln!(interpreter.writer, "Unknown command '{}'. Type :help.", trimmed).ok();
                continue;
            }
        }

        buffer.push_str(&line);
        if !buffer.ends_with('\n') {
            buffer.push('\n');
        }

        // A fragment is complete once it ends with the '.' terminator.
        if buffer.trim_end().ends_with('.') {
            let wrapped = format!("when (repl).\n{}", buffer);
            match run_fragment(&wrapped, &mut interpreter) {
                Ok(()) => {}
                Err(e) => {
                    writeln!(interpreter.writer, "{}", e).ok();
                }
            }
            buffer.clear();
        }
    }
    writeln!(interpreter.writer, "Bye.").ok();
}

fn run_fragment<R: BufRead, W: Write>(
    source: &str,
    interpreter: &mut Interpreter<R, W>,
) -> Result<(), SSharpError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    interpreter.interpret(&program)
}

fn write_repl_help<W: Write>(writer: &mut W) {
    writeln!(writer, "S# interactive mode:").ok();
    writeln!(writer, "  Write any S# fragment ending with '.' and it runs at once.").ok();
    writeln!(writer, "  Variables and functions persist between fragments.").ok();
    writeln!(writer, "  Fragments can span several lines; they run once the '.' arrives.").ok();
    writeln!(writer, "  :help    Show this help.").ok();
    writeln!(writer, "  :reset   Forget all variables and functions.").ok();
    writeln!(writer, "  :quit    Exit (Ctrl-D works too).").ok();
    writeln!(writer, "  Example: save 40 ++ 2 to answer.").ok();
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

    #[test]
    fn test_if_else() {
        let source = r#"
            when (test).
            save 20 to age.
            if (age >= 18), display "granted", else display "denied".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert!(output.contains("granted"));
        assert!(!output.contains("denied"));

        let source2 = r#"
            when (test).
            save 15 to age.
            if (age >= 18), display "granted", else display "denied".
        "#;
        let output2 = run_ssharp(source2, "").unwrap();
        assert!(output2.contains("denied"));
        assert!(!output2.contains("granted"));
    }

    #[test]
    fn test_else_if_chain() {
        let source = r#"
            when (test).
            save 5 to x.
            if (x), display "a", else if (not x), display "b", else display "c".
        "#;
        // x=5 is truthy -> "a"
        let output = run_ssharp(source, "").unwrap();
        assert!(output.contains("a"));
    }

    #[test]
    fn test_function_call() {
        let source = r#"
            when (test).
            define function plus(a, b), return a + b.
            save plus(5, 3) to result.
            display result.
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output.trim(), "8");
    }

    #[test]
    fn test_function_call_nested_and_scope() {
        let source = r#"
            when (test).
            define function double(n), return n * 2.
            define function plus(a, b), return a + b.
            save double(plus(2, 3)) to result.
            display result.
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output.trim(), "10");
    }

    #[test]
    fn test_function_arity_error() {
        let source = r#"
            when (test).
            define function plus(a, b), return a + b.
            display plus(1).
        "#;
        assert!(run_ssharp(source, "").is_err());
    }

    #[test]
    fn test_boolean_logic() {
        let source = r#"
            when (test).
            if (true or false), display "or-ok".
            if (true and not false), display "and-ok".
            if (not true), display "should-not-appear", else display "not-ok".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert!(output.contains("or-ok"));
        assert!(output.contains("and-ok"));
        assert!(output.contains("not-ok"));
        assert!(!output.contains("should-not-appear"));
    }

    #[test]
    fn test_string_escapes() {
        let source = "when (test). display \"a\\nb\". display \"say \\\"hi\\\"\".";
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "a\nb\nsay \"hi\"\n");
    }

    #[test]
    fn test_not_equal() {
        let source = r#"
            when (test).
            if (1 != 2), display "diff-ok", else display "broken".
            if (3 != 3), display "broken", else display "same-ok".
            if ("a" != "b"), display "str-ok".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert!(output.contains("diff-ok"));
        assert!(output.contains("same-ok"));
        assert!(output.contains("str-ok"));
        assert!(!output.contains("broken"));
    }

    #[test]
    fn test_explicit_concat() {
        // `++` always concatenates text; `+` adds numbers
        let source = r#"
            when (test).
            display 1 ++ 2.
            display 1 + 2.
            display "hi " ++ "there".
            save "Ada" to name.
            display "hello, " ++ name ++ "!".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "12\n3\nhi there\nhello, Ada!\n");
    }

    #[test]
    fn test_lists_literal_display_and_len() {
        let source = r#"
            when (test).
            save [1, "two", true] to xs.
            display xs.
            display len(xs).
            display len("hello").
            save [] to empty.
            display len(empty).
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "[1, two, true]\n3\n5\n0\n");
    }

    #[test]
    fn test_item_of_list_one_based() {
        let source = r#"
            when (test).
            save ["a", "b", "c"] to xs.
            display item 1 of xs.
            display item 3 of xs.
            display item 2 of "hey".
            save 2 to i.
            display item (i + 1) of xs.
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "a\nc\ne\nc\n");
    }

    #[test]
    fn test_item_of_out_of_bounds() {
        let source = r#"
            when (test).
            save [1] to xs.
            display item 2 of xs.
        "#;
        assert!(run_ssharp(source, "").is_err());

        let source2 = r#"
            when (test).
            save [1] to xs.
            display item 0 of xs.
        "#;
        assert!(run_ssharp(source2, "").is_err());
    }

    #[test]
    fn test_len_arity_and_type_errors() {
        let source = "when (test). display len([1], [2]).";
        assert!(run_ssharp(source, "").is_err());

        let source2 = "when (test). display len(42).";
        assert!(run_ssharp(source2, "").is_err());
    }

    #[test]
    fn test_list_mutation() {
        let source = r#"
            when (test).
            save [1, 2, 3] to xs.
            add 4 to xs.
            change item 1 of xs to 10.
            remove item 2 of xs.
            display xs.
            display len(xs).
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "[10, 3, 4]\n3\n");
    }

    #[test]
    fn test_list_mutation_errors() {
        // add to a non-list
        let source = "when (test). save 5 to x. add 1 to x.";
        assert!(run_ssharp(source, "").is_err());
        // change out of bounds
        let source2 = "when (test). save [1] to xs. change item 5 of xs to 9.";
        assert!(run_ssharp(source2, "").is_err());
        // remove from empty list
        let source3 = "when (test). save [] to xs. remove item 1 of xs.";
        assert!(run_ssharp(source3, "").is_err());
    }

    #[test]
    fn test_foreach_over_list_and_string() {
        let source = r#"
            when (test).
            save [1, 2, 3] to xs.
            save 0 to total.
            for each x in xs, save total + x to total.
            display total.
            for each c in "hey", display c.
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "6\nh\ne\ny\n");
    }

    #[test]
    fn test_foreach_non_iterable_error() {
        let source = "when (test). for each x in 42, display x.";
        assert!(run_ssharp(source, "").is_err());
    }

    #[test]
    fn test_function_with_body() {
        let source = r#"
            when (test).
            define function sum(xs), save 0 to total, for each x in xs, save total + x to total, return total.
            display sum([1, 2, 3, 4]).
            display sum(range(5)).
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "10\n15\n");
    }

    #[test]
    fn test_function_body_locals_do_not_leak() {
        let source = r#"
            when (test).
            save "outer" to t.
            define function f(a), save a * 2 to t, return t.
            display f(21).
            display t.
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "42\nouter\n");
    }

    #[test]
    fn test_equality_and_modulo() {
        let source = r#"
            when (test).
            if (7 % 3 == 1), display "mod-ok".
            if (10 % 5 == 0), display "zero-ok".
            if ("a" == "a"), display "str-eq-ok".
            if (1 == 2), display "broken", else display "neq-ok".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert!(output.contains("mod-ok"));
        assert!(output.contains("zero-ok"));
        assert!(output.contains("str-eq-ok"));
        assert!(output.contains("neq-ok"));
        assert!(!output.contains("broken"));
    }

    #[test]
    fn test_builtins_range_str_num() {
        let ok_part = "when (test). display range(4). display len(range(4)). display str(40 + 2). display num(\"3\") + 4.";
        let output = run_ssharp(ok_part, "").unwrap();
        assert_eq!(output, "[1, 2, 3, 4]\n4\n42\n7\n");
        let bad_part = "when (test). display num(\"age: \").";
        assert!(run_ssharp(bad_part, "").is_err());
    }

    #[test]
    fn test_nested_if_else_inside_repeat_runs() {
        let source = r#"when (test). repeat (2), if (true), display "a", else display "b". display "done"."#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "a\na\ndone\n");
    }

    #[test]
    fn test_try_catch_recovers() {
        let source = r#"
            when (test).
            try display item 5 of [1], catch display "caught".
            display "after".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "caught\nafter\n");
    }

    #[test]
    fn test_try_catch_binds_error_var() {
        let source = r#"
            when (test).
            try display item 5 of [1], catch e, display e.
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert!(output.contains("out of bounds"), "Expected bounds message, got: {}", output);
    }

    #[test]
    fn test_try_without_error_skips_catch() {
        let source = r#"
            when (test).
            try display "ok", catch display "fallback".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "ok\n");
    }

    #[test]
    fn test_try_catch_missing_file() {
        let source = r#"
            when (test).
            try read "definitely-not-here-12345.txt" and save to c, catch e, display "missing-ok".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert!(output.contains("missing-ok"), "Got: {}", output);
    }

    #[test]
    fn test_read_write_roundtrip() {
        let dir = std::env::temp_dir().join(format!("ssharp-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("roundtrip.txt");

        let source = format!(
            "when (test). write \"hello\\nworld\" to file \"{}\". read \"{}\" and save to back. display back.",
            file.display(),
            file.display()
        );
        // Absolute paths ignore base_dir, so this is hermetic.
        let output = run_ssharp(&source, "").unwrap();
        assert_eq!(output, "hello\nworld\n");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_import_runs_once_and_shares_functions() {
        let dir = std::env::temp_dir().join(format!("ssharp-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.ssharp");
        std::fs::write(&lib, "when (lib). define function triple(n), return n * 3.\n").unwrap();

        let source = format!(
            "when (test). import \"{}\". import \"{}\". display triple(7).",
            lib.display(),
            lib.display()
        );
        let output = run_ssharp(&source, "").unwrap();
        assert_eq!(output, "21\n");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_import_missing_file_is_error() {
        let source = "when (test). import \"definitely-not-here-12345.ssharp\".";
        let err = run_ssharp(source, "").unwrap_err();
        assert!(err.to_string().contains("Cannot import"), "Got: {}", err);
    }

    #[test]
    fn test_new_builtins() {
        let source = r#"when (test). display upper("hey"). display lower("HEY"). display split("a,b,c", ","). display join(["a", "b"], "-"). display sqrt(16). display type([1]). display type(1). display type("s"). display type(true)."#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "HEY\nhey\n[a, b, c]\na-b\n4\nlist\nint\nstring\nbool\n");
    }

    #[test]
    fn test_new_builtins_errors() {
        assert!(run_ssharp("when (test). display sqrt(0 - 1).", "").is_err());
        assert!(run_ssharp("when (test). display split(\"a\", \"\").", "").is_err());
        assert!(run_ssharp("when (test). display join(42, \",\").", "").is_err());
        assert!(run_ssharp("when (test). display upper(42).", "").is_err());
    }

    #[test]
    fn test_break_and_continue() {
        // Note: trailing loop code goes in `else` — `if (c), A, B.` parses
        // B as part of the then-branch.
        let source = r#"
            when (test).
            for each n in range(10), if (n == 3), continue, else display n.
            save 0 to i.
            while (true), save i + 1 to i, if (i == 3), break.
            display i.
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "1\n2\n4\n5\n6\n7\n8\n9\n10\n3\n");
    }

    #[test]
    fn test_break_only_exits_inner_loop() {
        let source = r#"
            when (test).
            repeat (2), for each x in [1, 2, 3], if (x == 2), break, else display x.
            display "done".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(output, "1\n1\ndone\n");
    }

    #[test]
    fn test_break_outside_loop_is_error() {
        assert!(run_ssharp("when (test). break.", "").is_err());
        assert!(run_ssharp("when (test). continue.", "").is_err());
        assert!(run_ssharp("when (test). if (true), break.", "").is_err());
    }

    #[test]
    fn test_integers_stay_exact() {
        let source = r#"
            when (test).
            display type(5).
            display type(5.5).
            display type(1 + 2).
            display type(1 + 2.5).
            display type(7 / 2).
            display 7 / 2.
            display 7 div 2.
            display type(7 div 2).
            display 0 - 7 div 2.
            if (9007199254740993 == 9007199254740993), display "big-eq".
            if (9007199254740993 == 9007199254740992), display "broken", else display "big-ne".
        "#;
        let output = run_ssharp(source, "").unwrap();
        assert_eq!(
            output,
            "int\nfloat\nint\nfloat\nfloat\n3.5\n3\nint\n-3\nbig-eq\nbig-ne\n"
        );
    }

    #[test]
    fn test_integer_overflow_and_div_errors() {
        assert!(run_ssharp("when (test). display 9223372036854775807 + 1.", "").is_err());
        assert!(run_ssharp("when (test). display 7 div 0.", "").is_err());
        assert!(run_ssharp("when (test). display 7.5 div 2.", "").is_err());
        assert!(run_ssharp("when (test). display 7 div 2.5.", "").is_err());
    }

    fn run_repl_session(input: &str) -> String {
        let reader = Cursor::new(input.as_bytes().to_vec());
        let mut writer = Vec::new();
        run_repl(reader, &mut writer);
        String::from_utf8(writer).unwrap()
    }

    #[test]
    fn test_repl_keeps_state_between_fragments() {
        let output = run_repl_session(
            "save 5 to x.\ndisplay x.\ndefine function double(n), return n * 2.\ndisplay double(x).\n",
        );
        assert!(output.contains("interactive mode"));
        assert!(output.contains(">> 5\n"));
        assert!(output.contains(">> 10\n"));
        assert!(output.contains("Bye."));
    }

    #[test]
    fn test_repl_multiline_fragment_and_ask() {
        let output = run_repl_session(
            "if (1 != 2),\ndisplay \"ne-ok\".\nask \"Name?\" and save to n.\nAda\ndisplay \"hi \" ++ n.\n",
        );
        assert!(output.contains("ne-ok"));
        assert!(output.contains("hi Ada"));
    }

    #[test]
    fn test_repl_commands_and_error_recovery() {
        let output = run_repl_session(
            "display 1 ++ 2.\n:bogus\nthis is not valid ssharp.\ndisplay \"after-error\".\n:reset\n:quit\ndisplay 999.\n",
        );
        assert!(output.contains("12"));
        assert!(output.contains("Unknown command"));
        assert!(output.contains("after-error"));
        assert!(output.contains("Environment cleared."));
        // The REPL survives parse errors and :quit stops the session.
        assert!(!output.contains("999"));
    }
}
