pub mod token;

use token::{Token, TokenKind};
use crate::error::SSharpError;

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, SSharpError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            let line = self.line;
            let column = self.column;

            let ch = match self.peek() {
                Some(c) => c,
                None => {
                    tokens.push(Token::new(TokenKind::Eof, line, column));
                    break;
                }
            };

            if ch == '"' {
                tokens.push(self.read_string()?);
            } else if ch.is_ascii_digit() {
                tokens.push(self.read_number()?);
            } else if ch.is_ascii_alphabetic() || ch == '_' {
                tokens.push(self.read_identifier_or_keyword());
            } else {
                self.advance(); // consume single/multi char symbol
                let kind = match ch {
                    ',' => TokenKind::Comma,
                    '.' => TokenKind::Period,
                    '(' => TokenKind::LParen,
                    ')' => TokenKind::RParen,
                    '[' => TokenKind::LBracket,
                    ']' => TokenKind::RBracket,
                    '+' => {
                        if self.peek() == Some('+') {
                            self.advance();
                            TokenKind::PlusPlus
                        } else {
                            TokenKind::Plus
                        }
                    }
                    '-' => TokenKind::Minus,
                    '*' => TokenKind::Star,
                    '/' => TokenKind::Slash,
                    '%' => TokenKind::Percent,
                    '=' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::EqEq
                        } else {
                            return Err(SSharpError::LexError {
                                message: "Unexpected '=', did you mean '==' for comparison?".to_string(),
                                line,
                                column,
                            });
                        }
                    }
                    '!' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::NotEqual
                        } else {
                            return Err(SSharpError::LexError {
                                message: "Unexpected character '!', did you mean '!='?".to_string(),
                                line,
                                column,
                            });
                        }
                    }
                    '>' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::GreaterEq
                        } else {
                            TokenKind::Greater
                        }
                    }
                    '<' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::LessEq
                        } else {
                            TokenKind::Less
                        }
                    }
                    _ => {
                        return Err(SSharpError::LexError {
                            message: format!("Unexpected character '{}'", ch),
                            line,
                            column,
                        });
                    }
                };
                tokens.push(Token::new(kind, line, column));
            }
        }

        Ok(tokens)
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '#' {
                // Reserve '#' for comment support (skip until newline)
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Result<Token, SSharpError> {
        let start_line = self.line;
        let start_column = self.column;

        self.advance(); // Consume opening quote '"'

        let mut content = String::new();
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.advance(); // Consume closing quote
                return Ok(Token::new(TokenKind::String(content), start_line, start_column));
            }
            if ch == '\\' {
                self.advance(); // Consume '\'
                match self.peek() {
                    Some('n') => {
                        content.push('\n');
                        self.advance();
                    }
                    Some('t') => {
                        content.push('\t');
                        self.advance();
                    }
                    Some('r') => {
                        content.push('\r');
                        self.advance();
                    }
                    Some('"') => {
                        content.push('"');
                        self.advance();
                    }
                    Some('\\') => {
                        content.push('\\');
                        self.advance();
                    }
                    Some('0') => {
                        content.push('\0');
                        self.advance();
                    }
                    Some(other) => {
                        // Unknown escape: keep the character as-is (drop backslash)
                        content.push(other);
                        self.advance();
                    }
                    None => {
                        return Err(SSharpError::LexError {
                            message: "Unterminated string literal".to_string(),
                            line: start_line,
                            column: start_column,
                        });
                    }
                }
                continue;
            }
            content.push(ch);
            self.advance();
        }

        Err(SSharpError::LexError {
            message: "Unterminated string literal".to_string(),
            line: start_line,
            column: start_column,
        })
    }

    fn read_number(&mut self) -> Result<Token, SSharpError> {
        let start_line = self.line;
        let start_column = self.column;

        let mut num_str = String::new();
        let mut is_float = false;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && self.peek_next().map_or(false, |next| next.is_ascii_digit()) {
                // Decimal point followed by digits
                is_float = true;
                num_str.push('.');
                self.advance();
            } else {
                break;
            }
        }

        if is_float {
            let val = num_str.parse::<f64>().map_err(|_| SSharpError::LexError {
                message: format!("Invalid number literal '{}'", num_str),
                line: start_line,
                column: start_column,
            })?;
            Ok(Token::new(TokenKind::Float(val), start_line, start_column))
        } else {
            num_str.parse::<i64>().map(|val| Token::new(TokenKind::Integer(val), start_line, start_column)).map_err(|_| SSharpError::LexError {
                message: format!("Integer literal '{}' is out of range (use a decimal point for big magnitudes)", num_str),
                line: start_line,
                column: start_column,
            })
        }
    }

    fn read_identifier_or_keyword(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;

        let mut word = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                word.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match word.as_str() {
            "when" => TokenKind::When,
            "ask" => TokenKind::Ask,
            "save" => TokenKind::Save,
            "to" => TokenKind::To,
            "and" => TokenKind::And,
            "display" => TokenKind::Display,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "repeat" => TokenKind::Repeat,
            "while" => TokenKind::While,
            "define" => TokenKind::Define,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "item" => TokenKind::Item,
            "of" => TokenKind::Of,
            "add" => TokenKind::Add,
            "change" => TokenKind::Change,
            "remove" => TokenKind::Remove,
            "for" => TokenKind::For,
            "each" => TokenKind::Each,
            "in" => TokenKind::In,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "import" => TokenKind::Import,
            "read" => TokenKind::Read,
            "write" => TokenKind::Write,
            "file" => TokenKind::File,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "div" => TokenKind::DivInt,
            _ => TokenKind::Identifier(word),
        };

        Token::new(kind, start_line, start_column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use token::TokenKind::*;

    #[test]
    fn test_keywords_and_identifiers() {
        let mut lexer = Lexer::new("when ask save to and display if else repeat while define function return true false or not item of add change remove for each in try catch import read write file break continue div score");
        let tokens = lexer.tokenize().unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                When, Ask, Save, To, And, Display, If, Else, Repeat, While, Define, Function, Return,
                True, False, Or, Not, Item, Of, Add, Change, Remove, For, Each, In,
                Try, Catch, Import, Read, Write, File, Break, Continue, DivInt,
                Identifier("score".into()),
                Eof
            ]
        );
    }

    #[test]
    fn test_string_with_punctuation() {
        let mut lexer = Lexer::new(r#"display "Hello, world." ."#);
        let tokens = lexer.tokenize().unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                Display,
                String("Hello, world.".into()),
                Period,
                Eof
            ]
        );
    }

    #[test]
    fn test_number_and_period() {
        let mut lexer = Lexer::new("18. 3.14.");
        let tokens = lexer.tokenize().unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                Integer(18),
                Period,
                Float(3.14),
                Period,
                Eof
            ]
        );
    }

    #[test]
    fn test_string_escape_sequences() {
        let mut lexer = Lexer::new(r#"display "a\nb\tc\"q\"\\"."#);
        let tokens = lexer.tokenize().unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                Display,
                String("a\nb\tc\"q\"\\".into()),
                Period,
                Eof
            ]
        );
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("+ - * / % == > < >= <= != ++ [ ]");
        let tokens = lexer.tokenize().unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![Plus, Minus, Star, Slash, Percent, EqEq, Greater, Less, GreaterEq, LessEq, NotEqual, PlusPlus, LBracket, RBracket, Eof]
        );
    }

    #[test]
    fn test_lone_equals_error() {
        let mut lexer = Lexer::new("if (x = 1).");
        let err = lexer.tokenize().unwrap_err();
        match err {
            SSharpError::LexError { message, line, column } => {
                assert!(message.contains("'=='"), "Unexpected message: {}", message);
                assert_eq!(line, 1);
                assert_eq!(column, 7);
            }
            _ => panic!("Expected LexError"),
        }
    }

    #[test]
    fn test_list_literal_and_comment() {
        let mut lexer = Lexer::new("save [1, 2] to xs. # a comment\n display item 1 of xs.");
        let tokens = lexer.tokenize().unwrap();
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                Save,
                LBracket,
                Integer(1),
                Comma,
                Integer(2),
                RBracket,
                To,
                Identifier("xs".into()),
                Period,
                Display,
                Item,
                Integer(1),
                Of,
                Identifier("xs".into()),
                Period,
                Eof
            ]
        );
    }

    #[test]
    fn test_integer_overflow_error() {
        let mut lexer = Lexer::new("display 99999999999999999999.");
        let err = lexer.tokenize().unwrap_err();
        match err {
            SSharpError::LexError { message, .. } => {
                assert!(message.contains("out of range"), "Unexpected message: {}", message);
            }
            _ => panic!("Expected LexError"),
        }
    }

    #[test]
    fn test_lone_bang_error() {
        let mut lexer = Lexer::new("display !.");
        let err = lexer.tokenize().unwrap_err();
        match err {
            SSharpError::LexError { message, line, column } => {
                assert!(message.contains("'!'"), "Unexpected message: {}", message);
                assert_eq!(line, 1);
                assert_eq!(column, 9);
            }
            _ => panic!("Expected LexError"),
        }
    }

    #[test]
    fn test_unknown_symbol_error() {        let mut lexer = Lexer::new("when @");
        let err = lexer.tokenize().unwrap_err();
        match err {
            SSharpError::LexError { message, line, column } => {
                assert_eq!(message, "Unexpected character '@'");
                assert_eq!(line, 1);
                assert_eq!(column, 6);
            }
            _ => panic!("Expected LexError"),
        }
    }
}
