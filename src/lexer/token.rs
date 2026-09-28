#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    When,
    Ask,
    Save,
    To,
    And,
    Display,
    If,
    Else,
    Repeat,
    While,
    Define,
    Function,
    Return,
    True,
    False,
    Or,
    Not,
    Item,
    Of,
    Add,
    Change,
    Remove,
    For,
    Each,
    In,
    Try,
    Catch,
    Import,
    Read,
    Write,
    File,
    Break,
    Continue,
    DivInt,

    // Literals & Identifiers
    Identifier(String),
    Integer(i64),
    Float(f64),
    String(String),

    // Punctuation
    Comma,
    Period,
    LParen,
    RParen,
    LBracket,
    RBracket,

    // Operators
    Plus,
    PlusPlus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEqual,
    Greater,
    Less,
    GreaterEq,
    LessEq,

    // End of file
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }
}
