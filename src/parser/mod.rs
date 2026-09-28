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
        let action = self.parse_action(false)?;

        // If action is ask, assign, or display at top-level, it requires a terminating '.'
        match action {
            Stmt::Ask { .. } | Stmt::Assign { .. } | Stmt::Display { .. } | Stmt::AddTo { .. } | Stmt::ChangeItem { .. } | Stmt::RemoveItem { .. } | Stmt::Import { .. } | Stmt::Read { .. } | Stmt::Write { .. } | Stmt::Break | Stmt::Continue => {
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
            Stmt::If { .. } | Stmt::Repeat { .. } | Stmt::While { .. } | Stmt::ForEach { .. } | Stmt::Try { .. } | Stmt::FunctionDef { .. } => {
                // These statements consume their closing '.' as part of their block definition
                Ok(action)
            }
        }
    }

    fn parse_action(&mut self, nested: bool) -> Result<Stmt, SSharpError> {
        let tok = self.peek().clone();

        match &tok.kind {
            TokenKind::Ask => self.parse_ask_stmt(),
            TokenKind::Save => self.parse_save_stmt(),
            TokenKind::Display => self.parse_display_stmt(),
            TokenKind::If => self.parse_if_stmt(nested),
            TokenKind::Repeat => self.parse_repeat_stmt(nested),
            TokenKind::While => self.parse_while_stmt(nested),
            TokenKind::For => self.parse_foreach_stmt(nested),
            TokenKind::Add => self.parse_add_stmt(),
            TokenKind::Change => self.parse_change_stmt(),
            TokenKind::Remove => self.parse_remove_stmt(),
            TokenKind::Try => self.parse_try_stmt(nested),
            TokenKind::Import => self.parse_import_stmt(),
            TokenKind::Read => self.parse_read_stmt(),
            TokenKind::Write => self.parse_write_stmt(),
            TokenKind::Break => {
                self.advance();
                Ok(Stmt::Break)
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Stmt::Continue)
            }
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

    fn parse_if_stmt(&mut self, nested: bool) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'if'

        self.consume(TokenKind::LParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after if condition")?;
        self.consume(TokenKind::Comma, "Expected ',' after 'if (...)' condition")?;

        let actions = self.parse_then_actions(nested)?;

        // Optional `else` branch: `..., else action1, action2.`
        // An optional comma after `else` is allowed: `else, display "x".`
        let else_actions = if matches!(self.peek_kind(), TokenKind::Else) {
            self.advance(); // consume 'else'
            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance(); // consume optional ',' after 'else'
            }
            // `else if (...)...` chaining: single nested if consumes its own '.'
            if matches!(self.peek_kind(), TokenKind::If) {
                let nested_if = self.parse_if_stmt(nested)?;
                Some(vec![nested_if])
            } else {
                Some(self.parse_action_list(nested)?)
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

    fn parse_repeat_stmt(&mut self, nested: bool) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'repeat'

        self.consume(TokenKind::LParen, "Expected '(' after 'repeat'")?;
        let count = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after repeat count")?;
        self.consume(TokenKind::Comma, "Expected ',' after 'repeat (...)'")?;

        let actions = self.parse_action_list(nested)?;
        Ok(Stmt::Repeat { count, actions })
    }

    fn parse_while_stmt(&mut self, nested: bool) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'while'

        self.consume(TokenKind::LParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RParen, "Expected ')' after while condition")?;
        self.consume(TokenKind::Comma, "Expected ',' after 'while (...)' condition")?;

        let actions = self.parse_action_list(nested)?;
        Ok(Stmt::While { condition, actions })
    }

    fn parse_foreach_stmt(&mut self, nested: bool) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'for'
        self.consume(TokenKind::Each, "Expected 'each' after 'for'")?;

        let var_tok = self.peek().clone();
        let var = match &var_tok.kind {
            TokenKind::Identifier(id) => {
                self.advance();
                id.clone()
            }
            _ => {
                return Err(SSharpError::ParseError {
                    message: format!("Expected loop variable identifier after 'for each', found {:?}", var_tok.kind),
                    line: var_tok.line,
                    column: var_tok.column,
                });
            }
        };

        self.consume(TokenKind::In, "Expected 'in' after loop variable")?;
        let iterable = self.parse_expression()?;
        self.consume(TokenKind::Comma, "Expected ',' after 'for each x in ...'")?;

        let actions = self.parse_action_list(nested)?;
        Ok(Stmt::ForEach { var, iterable, actions })
    }

    fn parse_add_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'add'

        let value = self.parse_expression()?;
        self.consume(TokenKind::To, "Expected 'to' after value in add statement")?;

        let target_tok = self.peek().clone();
        let target = match &target_tok.kind {
            TokenKind::Identifier(id) => {
                self.advance();
                id.clone()
            }
            _ => {
                return Err(SSharpError::ParseError {
                    message: format!("Expected target list identifier after 'to', found {:?}", target_tok.kind),
                    line: target_tok.line,
                    column: target_tok.column,
                });
            }
        };

        Ok(Stmt::AddTo { value, target })
    }

    fn parse_change_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'change'
        self.consume(TokenKind::Item, "Expected 'item' after 'change'")?;

        let index = self.parse_expression()?;
        self.consume(TokenKind::Of, "Expected 'of' after index in 'change item ... of ...'")?;

        let target_tok = self.peek().clone();
        let target = match &target_tok.kind {
            TokenKind::Identifier(id) => {
                self.advance();
                id.clone()
            }
            _ => {
                return Err(SSharpError::ParseError {
                    message: format!("Expected target list identifier, found {:?}", target_tok.kind),
                    line: target_tok.line,
                    column: target_tok.column,
                });
            }
        };

        self.consume(TokenKind::To, "Expected 'to' after list name in 'change item ... of ... to ...'")?;
        let value = self.parse_expression()?;

        Ok(Stmt::ChangeItem { target, index, value })
    }

    fn parse_remove_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'remove'
        self.consume(TokenKind::Item, "Expected 'item' after 'remove'")?;

        let index = self.parse_expression()?;
        self.consume(TokenKind::Of, "Expected 'of' after index in 'remove item ... of ...'")?;

        let target_tok = self.peek().clone();
        let target = match &target_tok.kind {
            TokenKind::Identifier(id) => {
                self.advance();
                id.clone()
            }
            _ => {
                return Err(SSharpError::ParseError {
                    message: format!("Expected target list identifier, found {:?}", target_tok.kind),
                    line: target_tok.line,
                    column: target_tok.column,
                });
            }
        };

        Ok(Stmt::RemoveItem { target, index })
    }

    fn parse_try_stmt(&mut self, nested: bool) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'try'

        let actions = self.parse_action_list(nested)?;

        // The action list stops at `, catch`, leaving it unconsumed.
        self.consume(TokenKind::Comma, "Expected ',' and 'catch' after 'try' actions")?;
        self.consume(TokenKind::Catch, "Expected 'catch' after 'try' actions")?;
        if matches!(self.peek_kind(), TokenKind::Comma) {
            self.advance(); // consume optional ',' after 'catch'
        }

        // Optional error variable: `catch e, <actions>` (identifier + comma).
        // `catch x and save to y.` is actions, not a binding (lookahead).
        let error_var = match (self.peek_kind().clone(), self.peek_next_kind().cloned()) {
            (TokenKind::Identifier(name), Some(TokenKind::Comma)) => {
                self.advance(); // consume variable name
                self.advance(); // consume ','
                Some(name)
            }
            _ => None,
        };

        let catch_actions = self.parse_action_list(nested)?;
        Ok(Stmt::Try { actions, error_var, catch_actions })
    }

    fn parse_import_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'import'

        let path = self.parse_expression()?;
        Ok(Stmt::Import { path })
    }

    fn parse_read_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'read'

        let path = self.parse_expression()?;

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

        Ok(Stmt::Read { path, target })
    }

    fn parse_write_stmt(&mut self) -> Result<Stmt, SSharpError> {
        self.advance(); // consume 'write'

        let value = self.parse_expression()?;
        self.consume(TokenKind::To, "Expected 'to' after value in write statement")?;
        self.consume(TokenKind::File, "Expected 'file' after 'to' in write statement (did you mean 'write <value> to file \"path\".'?)")?;
        let path = self.parse_expression()?;

        Ok(Stmt::Write { value, path })
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

        // Body: comma-separated actions, ending with `return <expr>.`
        // The single-expression form still works: `define function f(a), return a.`
        // A `.` after a body action is also accepted as a separator.
        let mut body = Vec::new();
        let return_expr = loop {
            if matches!(self.peek_kind(), TokenKind::Return) {
                self.advance(); // consume 'return'
                let ret = self.parse_expression()?;
                self.consume(TokenKind::Period, "Expected '.' at end of function definition")?;
                break ret;
            }
            let action = self.parse_action(true)?;
            body.push(action);
            let tok = self.peek().clone();
            match &tok.kind {
                TokenKind::Comma => {
                    self.advance(); // consume ',' and continue with next action or return
                }
                TokenKind::Period => {
                    self.advance(); // consume '.' separator left by a nested block
                }
                _ => {
                    return Err(SSharpError::ParseError {
                        message: format!("Expected ',' or 'return' in function body, found {:?}", tok.kind),
                        line: tok.line,
                        column: tok.column,
                    });
                }
            }
        };

        Ok(Stmt::FunctionDef { name, params, body, return_expr })
    }

    fn parse_then_actions(&mut self, nested: bool) -> Result<Vec<Stmt>, SSharpError> {
        let mut actions = Vec::new();
        loop {
            // Element actions are always nested: only the outermost list
            // owns the closing '.'.
            let action = self.parse_action(true)?;
            actions.push(action);

            let tok = self.peek();
            if matches!(tok.kind, TokenKind::Comma) {
                // `, return` ends the block: the return belongs to an
                // enclosing function body, so leave it unconsumed.
                // Same for `, catch`: it belongs to an enclosing try.
                let next_is_block_end = matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Return) | Some(TokenKind::Catch)
                );
                if next_is_block_end {
                    break;
                }
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
                // A nested block leaves its closing '.' for the enclosing block.
                if !nested {
                    self.advance(); // consume '.' and finish (no else branch)
                }
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

    fn parse_action_list(&mut self, nested: bool) -> Result<Vec<Stmt>, SSharpError> {
        let mut actions = Vec::new();
        loop {
            // Element actions are always nested: only the outermost list
            // owns the closing '.'.
            let action = self.parse_action(true)?;
            actions.push(action);

            let tok = self.peek();
            if matches!(tok.kind, TokenKind::Comma) {
                // `, return` ends the block: the return belongs to an
                // enclosing function body, so leave it unconsumed.
                // Same for `, catch`: it belongs to an enclosing try.
                let next_is_block_end = matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Return) | Some(TokenKind::Catch)
                );
                if next_is_block_end {
                    break;
                }
                self.advance(); // consume ',' and continue to next action
            } else if matches!(tok.kind, TokenKind::Period) {
                // A nested block leaves its closing '.' for the enclosing block.
                if !nested {
                    self.advance(); // consume '.' and finish action list
                }
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
            TokenKind::EqEq => BinOp::Eq,
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
            TokenKind::Percent => BinOp::Mod,
            TokenKind::DivInt => BinOp::DivInt,
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
            TokenKind::Integer(val) => {
                self.advance();
                Ok(Expr::Int(*val))
            }
            TokenKind::Float(val) => {
                self.advance();
                Ok(Expr::Float(*val))
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
            TokenKind::LBracket => {
                self.advance(); // consume '['
                let mut elements = Vec::new();
                if !matches!(self.peek_kind(), TokenKind::RBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if matches!(self.peek_kind(), TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RBracket, "Expected ']' after list elements")?;
                Ok(Expr::List(elements))
            }
            TokenKind::Item => {
                // Scratch-style access: `item <index-expr> of <list-expr>` (1-based).
                self.advance(); // consume 'item'
                let index = self.parse_expression()?;
                self.consume(TokenKind::Of, "Expected 'of' after index in 'item ... of ...'")?;
                let list = self.parse_expression()?;
                Ok(Expr::Index {
                    list: Box::new(list),
                    index: Box::new(index),
                })
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
                message: format!("Expected expression (number, string, list, true/false, variable, function call, 'item ... of ...', or '(expr)'), found {:?}", tok.kind),
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
        let source = "when (test). define function plus(a, b), return a + b. save plus(5, 3) to result. if (true or not false), display result.";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 3);
        assert!(matches!(&program.event.body[0], Stmt::FunctionDef { name, .. } if name == "plus"));
        match &program.event.body[1] {
            Stmt::Assign { value, target } => {
                assert_eq!(target, "result");
                assert!(matches!(value, Expr::Call { name, args } if name == "plus" && args.len() == 2));
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
    fn test_parse_list_literal_and_index() {
        let source = r#"when (test). save [1, "two", true] to xs. display item 2 of xs. display len(xs)."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 3);
        match &program.event.body[0] {
            Stmt::Assign { value, target } => {
                assert_eq!(target, "xs");
                assert!(matches!(value, Expr::List(items) if items.len() == 3));
            }
            _ => panic!("Expected Stmt::Assign with list literal"),
        }
        match &program.event.body[1] {
            Stmt::Display { value } => {
                assert!(matches!(value, Expr::Index { .. }));
            }
            _ => panic!("Expected Stmt::Display with index"),
        }
        match &program.event.body[2] {
            Stmt::Display { value } => {
                assert!(matches!(value, Expr::Call { name, args } if name == "len" && args.len() == 1));
            }
            _ => panic!("Expected Stmt::Display with len() call"),
        }
    }

    #[test]
    fn test_parse_empty_list() {
        let source = "when (test). save [] to xs.";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        match &program.event.body[0] {
            Stmt::Assign { value, target } => {
                assert_eq!(target, "xs");
                assert!(matches!(value, Expr::List(items) if items.is_empty()));
            }
            _ => panic!("Expected Stmt::Assign with empty list"),
        }
    }

    #[test]
    fn test_parse_equality_and_modulo() {
        let source = r#"when (test). if (n % 15 == 0), display "fizzbuzz"."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        match &program.event.body[0] {
            Stmt::If { condition, .. } => {
                assert!(matches!(condition, Expr::Binary { op: BinOp::Eq, .. }));
                match &**match condition {
                    Expr::Binary { left, .. } => left,
                    _ => unreachable!(),
                } {
                    Expr::Binary { op, .. } => assert!(matches!(op, BinOp::Mod)),
                    _ => panic!("Expected modulo on the left of =="),
                }
            }
            _ => panic!("Expected Stmt::If"),
        }
    }

    #[test]
    fn test_parse_foreach() {
        let source = "when (test). save [1, 2] to xs. for each x in xs, display x.";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 2);
        match &program.event.body[1] {
            Stmt::ForEach { var, actions, .. } => {
                assert_eq!(var, "x");
                assert_eq!(actions.len(), 1);
            }
            _ => panic!("Expected Stmt::ForEach"),
        }
    }

    #[test]
    fn test_parse_list_mutation() {
        let source = "when (test). add 5 to xs. change item 1 of xs to 9. remove item 2 of xs.";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 3);
        assert!(matches!(&program.event.body[0], Stmt::AddTo { target, .. } if target == "xs"));
        assert!(matches!(&program.event.body[1], Stmt::ChangeItem { target, .. } if target == "xs"));
        assert!(matches!(&program.event.body[2], Stmt::RemoveItem { target, .. } if target == "xs"));
    }

    #[test]
    fn test_parse_function_with_body() {        let source = "when (test). define function f(a), save a * 2 to t, save t + 1 to u, return u.";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        match &program.event.body[0] {
            Stmt::FunctionDef { name, params, body, return_expr } => {
                assert_eq!(name, "f");
                assert_eq!(params, &vec!["a".to_string()]);
                assert_eq!(body.len(), 2);
                assert!(matches!(body[0], Stmt::Assign { .. }));
                assert!(matches!(return_expr, Expr::Identifier(id) if id == "u"));
            }
            _ => panic!("Expected Stmt::FunctionDef with body"),
        }
    }

    #[test]
    fn test_parse_function_body_with_nested_block_and_return() {
        // `, return` after a nested block belongs to the function, not the block.
        let source = "when (test). define function f(xs), for each x in xs, display x, return len(xs).";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        match &program.event.body[0] {
            Stmt::FunctionDef { body, return_expr, .. } => {
                assert_eq!(body.len(), 1);
                assert!(matches!(body[0], Stmt::ForEach { .. }));
                assert!(matches!(return_expr, Expr::Call { name, .. } if name == "len"));
            }
            _ => panic!("Expected Stmt::FunctionDef"),
        }
    }

    #[test]
    fn test_parse_try_catch() {
        let source = r#"when (test). try display risky, catch e, display e."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 1);
        match &program.event.body[0] {
            Stmt::Try { actions, error_var, catch_actions } => {
                assert_eq!(actions.len(), 1);
                assert_eq!(error_var, &Some("e".to_string()));
                assert_eq!(catch_actions.len(), 1);
            }
            _ => panic!("Expected Stmt::Try"),
        }
    }

    #[test]
    fn test_parse_try_catch_without_var() {
        let source = r#"when (test). try display risky, catch display "fallback"."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        match &program.event.body[0] {
            Stmt::Try { actions, error_var, catch_actions } => {
                assert_eq!(actions.len(), 1);
                assert!(error_var.is_none());
                assert_eq!(catch_actions.len(), 1);
            }
            _ => panic!("Expected Stmt::Try"),
        }
    }

    #[test]
    fn test_parse_try_catch_nested_block() {
        // The try body can hold a nested block; `, catch` still closes it.
        let source = r#"when (test). try repeat (2), display "x", catch display "fallback"."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        match &program.event.body[0] {
            Stmt::Try { actions, catch_actions, .. } => {
                assert_eq!(actions.len(), 1);
                assert!(matches!(actions[0], Stmt::Repeat { .. }));
                assert_eq!(catch_actions.len(), 1);
            }
            _ => panic!("Expected Stmt::Try"),
        }
    }

    #[test]
    fn test_parse_import_read_write() {
        let source = r#"when (test). import "utils.ssharp". read "data.txt" and save to content. write content to file "out.txt"."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 3);
        assert!(matches!(&program.event.body[0], Stmt::Import { .. }));
        match &program.event.body[1] {
            Stmt::Read { target, .. } => assert_eq!(target, "content"),
            _ => panic!("Expected Stmt::Read"),
        }
        assert!(matches!(&program.event.body[2], Stmt::Write { .. }));
    }

    #[test]
    fn test_nested_if_else_inside_repeat() {
        // The inner if must not steal the repeat's closing '.'.
        let source = r#"when (test). repeat (2), if (true), display "a", else display "b". display "done"."#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        assert_eq!(program.event.body.len(), 2);
        assert!(matches!(&program.event.body[0], Stmt::Repeat { .. }));
    }

    #[test]
    fn test_parse_repeat_and_save() {        let source = "when (test). save 10 to score. repeat (5), display score.";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.event.body.len(), 2);
        assert!(matches!(&program.event.body[0], Stmt::Assign { target, .. } if target == "score"));
        assert!(matches!(&program.event.body[1], Stmt::Repeat { .. }));
    }
}
