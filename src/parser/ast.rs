#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub event: WhenBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhenBlock {
    pub event_name: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Ask {
        prompt: String,
        target: String,
    },
    Assign {
        value: Expr,
        target: String,
    },
    Display {
        value: Expr,
    },
    If {
        condition: Expr,
        actions: Vec<Stmt>,
    },
    Repeat {
        count: Expr,
        actions: Vec<Stmt>,
    },
    While {
        condition: Expr,
        actions: Vec<Stmt>,
    },
    FunctionDef {
        name: String,
        params: Vec<String>,
        return_expr: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Str(String),
    Identifier(String),
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Gt,
    Lt,
    GtEq,
    LtEq,
}
