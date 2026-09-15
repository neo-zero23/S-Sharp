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
        else_actions: Option<Vec<Stmt>>,
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
    Bool(bool),
    Identifier(String),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Add,
    Concat,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Gt,
    Lt,
    GtEq,
    LtEq,
}
