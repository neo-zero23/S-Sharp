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
    Repeat,
    While,
    Define,
    Function,
    Return,

    // Literals & Identifiers
    Identifier(String),
    Number(f64),
    String(String),

    // Punctuation
    Comma,
    Period,
    LParen,
    RParen,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Equals,
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
