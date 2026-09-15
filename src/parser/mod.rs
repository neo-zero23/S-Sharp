pub mod ast;

use ast::*;
use crate::error::SSharpError;
use crate::lexer::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens.last().expect("Tokens vector should not be empty")
        })
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_next_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos + 1).map(|t| &t.kind)
    }

    /// Returns true if the upcoming `and` is the `and save to <var>` construct
    /// (i.e. `and` is directly followed by `save`), in which case it must NOT
    /// be treated as a logical AND operator.
    fn and_is_save_construct(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::And)
            && matches!(self.peek_next_kind(), Some(TokenKind::Save))
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.tokens.get(self.pos - 1).unwrap()
    }

    fn consume(&mut self, expected: TokenKind, err_msg: &str) -> Result<&Token, SSharpError> {
        let token = self.peek();
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(&expected) {
            Ok(self.advance())
        } else {
            Err(SSharpError::ParseError {
                message: format!("{}, found {:?}", err_msg, token.kind),
                line: token.line,
                column: token.column,
            })
        }
    }

    pub fn parse(&mut self) -> Result<Program, SSharpError> {
        let event = self.parse_when_block()?;
        Ok(Program { event })
    }

    fn parse_when_block(&mut self) -> Result<WhenBlock, SSharpError> {
        let tok = self.peek();
        if !matches!(tok.kind, TokenKind::When) {
            return Err(SSharpError::ParseError {
                message: format!("Expected 'when' keyword at program start, found {:?}", tok.kind),
                line: tok.line,
                column: tok.column,
            });
        }
        self.advance(); // consume 'when'

        self.consume(TokenKind::LParen, "Expected '(' after 'when'")?;

        let event_tok = self.peek().clone();
        let event_name = match &event_tok.kind {
            TokenKind::Identifier(name) => {
                self.advance();
                name.clone()
            }
            _ => {
                return Err(SSharpError::ParseError {
                    message: format!("Expected event identifier inside 'when(...)', found {:?}", event_tok.kind),
                    line: event_tok.line,
                    column: event_tok.column,
                });
            }
        };

        self.consume(TokenKind::RParen, "Expected ')' after event name")?;
        self.consume(TokenKind::Period, "Expected '.' after 'when (...)' block header")?;

        let mut body = Vec::new();
        while !self.is_at_end() && !matches!(self.peek_kind(), TokenKind::When) {
            body.push(self.parse_statement()?);
        }

        Ok(WhenBlock { event_name, body })
    }

    fn parse_statement(&mut self) -> Result<Stmt, SSharpError> {
        let action = self.parse_action()?;

        // If action is ask, assign, or display at top-level, it requires a terminating '.'
        match action {
            Stmt::Ask { .. } | Stmt::Assign { .. } | Stmt::Display { .. } => {
                let tok = self.peek();
                if matches!(tok.kind, TokenKind::Period) {
                    self.advance();
                    Ok(action)
                } else {
                    Err(SSharpError::ParseError {
                        message: format!("Expected '.' to close statement, found {:?}", tok.kind),
                        line: tok.line,
                        column: tok.column,
                    })
                }
            }
            Stmt::If { .. } | Stmt::Repeat { .. } | Stmt::While { .. } | Stmt::FunctionDef { .. } => {
                // These statements consume their closing '.' as part of their block definition
                Ok(action)
            }
        }
    }

    fn parse_action(&mut self) -> Result<Stmt, SSharpError> {
        let tok = self.peek().clone();

        match &tok.kind {
            TokenKind::Ask => self.parse_ask_stmt(),
            TokenKind::Save => self.parse_save_stmt(),
            TokenKind::Display => self.parse_display_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::Repeat => self.parse_repeat_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::Define => self.parse_function_def_stmt(),
            _ => {
                // Check if it's an expression followed by 'and save to <ident>'
                let expr = self.parse_expression()?;
                if matches!(self.peek_kind(), TokenKind::And) {
                    self.advance(); // consume 'and'
                    self.consume(TokenKind::Save, "Expected 'save' after 'and'")?;
                    self.consume(TokenKind::To, "Expected 'to' after 'save'")?;

                    let target_tok = self.peek().clone();
                    let target = match &target_tok.kind {
                        TokenKind::Identifier(id) => {
                            self.advance();
                            id.clone()
                        }
                        _ => {
                            return Err(SSharpError::ParseError {
                                message: format!("Expected target variable identifier, found {:?}", target_tok.kind),
                                line: target_tok.line,
                                column: target_tok.column,
                            });
                        }
                    };

                    Ok(Stmt::Assign { value: expr, target })
                } else {
                    Err(SSharpError::ParseError {
                        message: format!("Unexpected statement starting with {:?}", tok.kind),
                        line: tok.line,
                        column: tok.column,
                    })
                }
            }
        }
    }

    fn parse_ask_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'ask'

        let prompt_tok = self.peek().clone();
        let prompt = match &prompt_tok.kind {
            TokenKind::String(s) => {
                self.advance();
                s.clone()
            }
            _ => {
                return Err(SSharpError::ParseError {
                    message: format!("Expected string prompt after 'ask', found {:?}", prompt_tok.kind),
                    line: prompt_tok.line,
                    column: prompt_tok.column,
                });
            }
        };

        let mut target = String::new();
        if matches!(self.peek_kind(), TokenKind::And) {
            self.advance(); // consume 'and'
            self.consume(TokenKind::Save, "Expected 'save' after 'and'")?;
            self.consume(TokenKind::To, "Expected 'to' after 'save'")?;

            let target_tok = self.peek().clone();
            target = match &target_tok.kind {
                TokenKind::Identifier(id) => {
                    self.advance();
                    id.clone()
                }
                _ => {
                    return Err(SSharpError::ParseError {
                        message: format!("Expected variable identifier after 'save to', found {:?}", target_tok.kind),
                        line: target_tok.line,
                        column: target_tok.column,
                    });
                }
            };
        }

        Ok(Stmt::Ask { prompt, target })
    }

    fn parse_save_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'save'

        let value = self.parse_expression()?;
        self.consume(TokenKind::To, "Expected 'to' after expression in save statement")?;

        let target_tok = self.peek().clone();
        let target = match &target_tok.kind {
            TokenKind::Identifier(id) => {
                self.advance();
                id.clone()
            }
            _ => {
                return Err(SSharpError::ParseError {
                    message: format!("Expected target variable identifier after 'to', found {:?}", target_tok.kind),
                    line: target_tok.line,
                    column: target_tok.column,
                });
            }
        };

        Ok(Stmt::Assign { value, target })
    }

    fn parse_display_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'display'

        let value = self.parse_expression()?;
        Ok(Stmt::Display { value })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'if'

        self.consume(TokenKind::LParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after if condition")?;
        self.consume(TokenKind::Comma, "Expected ',' after 'if (...)' condition")?;

        let actions = self.parse_then_actions()?;

        // Optional `else` branch: `..., else action1, action2.`
        // An optional comma after `else` is allowed: `else, display "x".`
        let else_actions = if matches!(self.peek_kind(), TokenKind::Else) {
            self.advance(); // consume 'else'
            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance(); // consume optional ',' after 'else'
            }
            // `else if (...)...` chaining: single nested if consumes its own '.'
            if matches!(self.peek_kind(), TokenKind::If) {
                let nested = self.parse_if_stmt()?;
                Some(vec![nested])
            } else {
                Some(self.parse_action_list()?)
            }
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            actions,
            else_actions,
        })
    }

    fn parse_repeat_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'repeat'

        self.consume(TokenKind::LParen, "Expected '(' after 'repeat'")?;
        let count = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after repeat count")?;
        self.consume(TokenKind::Comma, "Expected ',' after 'repeat (...)'")?;

        let actions = self.parse_action_list()?;
        Ok(Stmt::Repeat { count, actions })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'while'

        self.consume(TokenKind::LParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after while condition")?;
        self.consume(TokenKind::Comma, "Expected ',' after 'while (...)' condition")?;

        let actions = self.parse_action_list()?;
        Ok(Stmt::While { condition, actions })
    }

    fn parse_function_def_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'define'
        self.consume(TokenKind::Function, "Expected 'function' after 'define'")?;

        let name_tok = self.peek().clone();
        let name = match &name_tok.kind {
            TokenKind::Identifier(id) => {
                self.advance();
                id.clone()
            }
            _ => {
                return Err(SSharpError::ParseError {
                    message: format!("Expected function name identifier, found {:?}", name_tok.kind),
                    line: name_tok.line,
                    column: name_tok.column,
                });
            }
        };

        self.consume(TokenKind::LParen, "Expected '(' after function name")?;
        let mut params = Vec::new();
        if !matches!(self.peek_kind(), TokenKind::RParen) {
            loop {
                let param_tok = self.peek().clone();
                match &param_tok.kind {
                    TokenKind::Identifier(p) => {
                        self.advance();
                        params.push(p.clone());
                    }
                    _ => {
                        return Err(SSharpError::ParseError {
                            message: format!("Expected parameter identifier, found {:?}", param_tok.kind),
                            line: param_tok.line,
                            column: param_tok.column,
                        });
                    }
                }
                if matches!(self.peek_kind(), TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.consume(TokenKind::RParen, "Expected ')' after function parameters")?;
        self.consume(TokenKind::Comma, "Expected ',' after function header")?;
        self.consume(TokenKind::Return, "Expected 'return' in function definition")?;

        let return_expr = self.parse_expression()?;
        self.consume(TokenKind::Period, "Expected '.' at end of function definition")?;

        Ok(Stmt::FunctionDef { name, params, return_expr })
    }

    fn parse_then_actions(&mut self) -> Result<Vec<Stmt>, SSharpError> {
        let mut actions = Vec::new();
        loop {
            let action = self.parse_action()?;
            actions.push(action);

            let tok = self.peek();
            if matches!(tok.kind, TokenKind::Comma) {
                // `, else` ends the then-branch (comma is the separator before else)
                let next_is_else = matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Else)
                );
                self.advance(); // consume ','
                if next_is_else {
                    break;
                }
            } else if matches!(tok.kind, TokenKind::Else) {
                // `else` without preceding comma also ends the then-branch
                break;
            } else if matches!(tok.kind, TokenKind::Period) {
                self.advance(); // consume '.' and finish (no else branch)
                break;
            } else {
                return Err(SSharpError::ParseError {
                    message: format!("Expected ',' or '.' after action in statement body, found {:?}", tok.kind),
                    line: tok.line,
                    column: tok.column,
                });
            }
        }
        Ok(actions)
    }

    fn parse_action_list(&mut self) -> Result<Vec<Stmt>, SSharpError> {
        let mut actions = Vec::new();
        loop {
            let action = self.parse_action()?;
            actions.push(action);

            let tok = self.peek();
            if matches!(tok.kind, TokenKind::Comma) {
                self.advance(); // consume ',' and continue to next action
            } else if matches!(tok.kind, TokenKind::Period) {
                self.advance(); // consume '.' and finish action list
                break;
            } else {
                return Err(SSharpError::ParseError {
                    message: format!("Expected ',' or '.' after action in statement body, found {:?}", tok.kind),
                    line: tok.line,
                    column: tok.column,
                });
            }
        }
        Ok(actions)
    }

    // --- Expression Parsing with Precedence ---
    // Lowest to highest: or -> and -> comparison -> additive -> multiplicative -> unary -> primary

    fn parse_expression(&mut self) -> Result<Expr, SSharpError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, SSharpError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek_kind(), TokenKind::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, SSharpError> {
        let mut left = self.parse_comparison()?;
        // `and` directly followed by `save` is the `and save to` construct, not logic
        while matches!(self.peek_kind(), TokenKind::And) && !self.and_is_save_construct() {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, SSharpError> {
        let mut left = self.parse_additive()?;

        while let Some(op) = self.match_comparison_op() {
            let right = self.parse_additive()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn match_comparison_op(&mut self) -> Option<BinOp> {
        let op = match self.peek_kind() {
            TokenKind::Equals => BinOp::Eq,
            TokenKind::NotEqual => BinOp::NotEq,
            TokenKind::Greater => BinOp::Gt,
            TokenKind::Less => BinOp::Lt,
            TokenKind::GreaterEq => BinOp::GtEq,
            TokenKind::LessEq => BinOp::LtEq,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn parse_additive(&mut self) -> Result<Expr, SSharpError> {
        let mut left = self.parse_multiplicative()?;

        while let Some(op) = self.match_additive_op() {
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn match_additive_op(&mut self) -> Option<BinOp> {
        let op = match self.peek_kind() {
            TokenKind::Plus => BinOp::Add,
            TokenKind::PlusPlus => BinOp::Concat,
            TokenKind::Minus => BinOp::Sub,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, SSharpError> {
        let mut left = self.parse_unary()?;

        while let Some(op) = self.match_multiplicative_op() {
            let right = self.parse_unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn match_multiplicative_op(&mut self) -> Option<BinOp> {
        let op = match self.peek_kind() {
            TokenKind::Star => BinOp::Mul,
            TokenKind::Slash => BinOp::Div,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn parse_unary(&mut self) -> Result<Expr, SSharpError> {
        if matches!(self.peek_kind(), TokenKind::Not) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
            });
        }
        if matches!(self.peek_kind(), TokenKind::Minus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(expr),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, SSharpError> {
        let tok = self.peek().clone();

        match &tok.kind {
            TokenKind::Number(val) => {
                self.advance();
                Ok(Expr::Number(*val))
            }
            TokenKind::String(val) => {
                self.advance();
                Ok(Expr::Str(val.clone()))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::Identifier(id) => {
                let name = id.clone();
                self.advance();
                // Function call: `name(arg1, arg2, ...)`
                if matches!(self.peek_kind(), TokenKind::LParen) {
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    if !matches!(self.peek_kind(), TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.peek_kind(), TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.consume(TokenKind::RParen, "Expected ')' after function call arguments")?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Identifier(name))
                }
            }
            TokenKind::LParen => {
                self.advance(); // consume '('
                let expr = self.parse_expression()?;
                self.consume(TokenKind::RParen, "Expected ')' after parenthesized expression")?;
                Ok(expr)
            }
            _ => Err(SSharpError::ParseError {
                message: format!("Expected expression (number, string, true/false, variable, function call, or '(expr)'), found {:?}", tok.kind),
                line: tok.line,
                column: tok.column,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_primary_milestone() {
        let source = r#"
            when (start_clicked).
            ask "How old are you?" and save to age.
            if (age >= 18), display "Access granted".
            if (age < 18), display "Access denied".
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.event_name, "start_clicked");
        assert_eq!(program.event.body.len(), 3);

        match &program.event.body[0] {
            Stmt::Ask { prompt, target } => {
                assert_eq!(prompt, "How old are you?");
                assert_eq!(target, "age");
            }
            _ => panic!("Expected Stmt::Ask"),
        }

        match &program.event.body[1] {
            Stmt::If { condition, actions, else_actions } => {
                assert_eq!(actions.len(), 1);
                assert!(matches!(condition, Expr::Binary { op: BinOp::GtEq, .. }));
                assert!(else_actions.is_none());
            }
            _ => panic!("Expected Stmt::If"),
        }

        match &program.event.body[2] {
            Stmt::If { condition, actions, else_actions } => {
                assert_eq!(actions.len(), 1);
                assert!(matches!(condition, Expr::Binary { op: BinOp::Lt, .. }));
                assert!(else_actions.is_none());
            }
            _ => panic!("Expected Stmt::If"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        let source = r#"when (test). if (age >= 18), display "granted", else display "denied"."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 1);
        match &program.event.body[0] {
            Stmt::If { condition, actions, else_actions } => {
                assert!(matches!(condition, Expr::Binary { op: BinOp::GtEq, .. }));
                assert_eq!(actions.len(), 1);
                let else_branch = else_actions.as_ref().expect("Expected else branch");
                assert_eq!(else_branch.len(), 1);
                assert!(matches!(else_branch[0], Stmt::Display { .. }));
            }
            _ => panic!("Expected Stmt::If with else"),
        }
    }

    #[test]
    fn test_parse_else_if_chain() {
        let source = r#"when (test). if (x), display "a", else if (y), display "b", else display "c"."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        match &program.event.body[0] {
            Stmt::If { else_actions, .. } => {
                let else_branch = else_actions.as_ref().expect("Expected else branch");
                assert_eq!(else_branch.len(), 1);
                assert!(matches!(else_branch[0], Stmt::If { .. }));
            }
            _ => panic!("Expected Stmt::If"),
        }
    }

    #[test]
    fn test_parse_function_call_and_booleans() {
        let source = "when (test). define function add(a, b), return a + b. save add(5, 3) to result. if (true or not false), display result.";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 3);
        assert!(matches!(&program.event.body[0], Stmt::FunctionDef { name, .. } if name == "add"));
        match &program.event.body[1] {
            Stmt::Assign { value, target } => {
                assert_eq!(target, "result");
                assert!(matches!(value, Expr::Call { name, args } if name == "add" && args.len() == 2));
            }
            _ => panic!("Expected Stmt::Assign with call"),
        }
        match &program.event.body[2] {
            Stmt::If { condition, .. } => {
                assert!(matches!(condition, Expr::Binary { op: BinOp::Or, .. }));
            }
            _ => panic!("Expected Stmt::If"),
        }
    }

    #[test]
    fn test_parse_and_save_disambiguation() {
        // `and save to` must remain an assignment, not a logical AND
        let source = r#"when (test). save 5 to x. 10 and save to y."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 2);
        assert!(matches!(&program.event.body[1], Stmt::Assign { target, .. } if target == "y"));
    }

    #[test]
    fn test_parse_not_equal_and_concat() {
        let source = r#"when (test). if (name != "admin"), display "hi " ++ name."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        match &program.event.body[0] {
            Stmt::If { condition, actions, else_actions } => {
                assert!(matches!(condition, Expr::Binary { op: BinOp::NotEq, .. }));
                assert!(else_actions.is_none());
                assert_eq!(actions.len(), 1);
                match &actions[0] {
                    Stmt::Display { value } => {
                        assert!(matches!(value, Expr::Binary { op: BinOp::Concat, .. }));
                    }
                    _ => panic!("Expected Stmt::Display with concat"),
                }
            }
            _ => panic!("Expected Stmt::If"),
        }
    }

    #[test]
    fn test_parse_repeat_and_save() {
        let source = "when (test). save 10 to score. repeat (5), display score.";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 2);
        assert!(matches!(&program.event.body[0], Stmt::Assign { target, .. } if target == "score"));
        assert!(matches!(&program.event.body[1], Stmt::Repeat { .. }));
    }
}
