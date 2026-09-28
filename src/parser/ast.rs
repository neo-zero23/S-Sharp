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
    ForEach {
        var: String,
        iterable: Expr,
        actions: Vec<Stmt>,
    },
    AddTo {
        value: Expr,
        target: String,
    },
    ChangeItem {
        target: String,
        index: Expr,
        value: Expr,
    },
    RemoveItem {
        target: String,
        index: Expr,
    },
    FunctionDef {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        return_expr: Expr,
    },
    Try {
        actions: Vec<Stmt>,
        error_var: Option<String>,
        catch_actions: Vec<Stmt>,
    },
    Import {
        path: Expr,
    },
    Read {
        path: Expr,
        target: String,
    },
    Write {
        value: Expr,
        path: Expr,
    },
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    List(Vec<Expr>),
    Identifier(String),
    Index {
        list: Box<Expr>,
        index: Box<Expr>,
    },
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
    Mod,
    DivInt,
    Eq,
    NotEq,
    Gt,
    Lt,
    GtEq,
    LtEq,
}
