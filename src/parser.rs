use crate::ast::*;
use crate::lexer::Token;
use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Token {
        self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected {:?}, got {:?}", expected, self.peek()))
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut functions = HashMap::new();
        let mut structs = HashMap::new();
        while self.peek() != Token::Eof {
            match self.peek() {
                Token::Fn => {
                    let func = self.parse_function()?;
                    functions.insert(func.name.clone(), func);
                }
                Token::Struct => {
                    let s = self.parse_struct()?;
                    structs.insert(s.name.clone(), s);
                }
                _ => {
                    return Err(format!(
                        "expected function or struct, got {:?}",
                        self.peek()
                    ))
                }
            }
        }
        Ok(Program { functions, structs })
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        self.expect(Token::Fn)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected function name".to_string()),
        };
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        if self.peek() != Token::RParen {
            loop {
                match self.advance() {
                    Token::Ident(p) => params.push(p),
                    _ => return Err("expected parameter name".to_string()),
                }
                match self.peek() {
                    Token::Comma => {
                        self.advance();
                        continue;
                    }
                    Token::RParen => break,
                    _ => return Err("expected ',' or ')' after parameter".to_string()),
                }
            }
        }
        self.expect(Token::RParen)?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != Token::RBrace {
            body.push(self.parse_stmt()?);
        }
        self.expect(Token::RBrace)?;
        // Default return type: Void
        let return_type = Type::Void;
        Ok(Function {
            name,
            params,
            body,
            return_type,
        })
    }

    fn parse_struct(&mut self) -> Result<Struct, String> {
        self.expect(Token::Struct)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected struct name".to_string()),
        };
        self.expect(Token::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != Token::RBrace {
            let fname = match self.advance() {
                Token::Ident(s) => s,
                _ => return Err("expected field name".to_string()),
            };
            self.expect(Token::Colon)?;
            let typ = match self.advance() {
                Token::Ident(s) if s == "i32" => Type::I32,
                Token::Ident(s) if s == "bool" => Type::Bool,
                _ => return Err("expected type i32 or bool".to_string()),
            };
            self.expect(Token::Semicolon)?;
            fields.push((fname, typ));
        }
        self.expect(Token::RBrace)?;
        Ok(Struct { name, fields })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Token::Let => {
                self.advance();
                let name = match self.advance() {
                    Token::Ident(s) => s,
                    _ => return Err("expected variable name".to_string()),
                };
                self.expect(Token::Eq)?;
                let init = self.parse_expr()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::Let {
                    name,
                    init: Some(init),
                })
            }
            Token::If => {
                self.advance();
                let cond = self.parse_expr()?;
                self.expect(Token::LBrace)?;
                let then_block = self.parse_block()?;
                let else_block = if self.peek() == Token::Else {
                    self.advance();
                    if self.peek() == Token::If {
                        Some(vec![Stmt::If {
                            cond: self.parse_expr()?,
                            then_block: self.parse_block()?,
                            else_block: None,
                        }])
                    } else {
                        self.expect(Token::LBrace)?;
                        Some(self.parse_block()?)
                    }
                } else {
                    None
                };
                Ok(Stmt::If {
                    cond,
                    then_block,
                    else_block,
                })
            }
            Token::While => {
                self.advance();
                let cond = self.parse_expr()?;
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                Ok(Stmt::While { cond, body })
            }
            Token::Loop => {
                self.advance();
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                Ok(Stmt::Loop { body })
            }
            Token::Break => {
                self.advance();
                self.expect(Token::Semicolon)?;
                Ok(Stmt::Break)
            }
            Token::Continue => {
                self.advance();
                self.expect(Token::Semicolon)?;
                Ok(Stmt::Continue)
            }
            Token::Return => {
                self.advance();
                let expr = if self.peek() != Token::Semicolon {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect(Token::Semicolon)?;
                Ok(Stmt::Return(expr))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(Token::Semicolon)?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while self.peek() != Token::RBrace {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_logical_or()?;
        if self.peek() == Token::Eq {
            self.advance();
            let rhs = self.parse_assignment()?;
            match lhs {
                Expr::Variable(name) => Ok(Expr::BinaryOp {
                    op: BinOp::Eq,
                    left: Box::new(Expr::Variable(name)),
                    right: Box::new(rhs),
                }),
                _ => Err("left side of assignment must be a variable".to_string()),
            }
        } else {
            Ok(lhs)
        }
    }

    fn parse_logical_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_logical_and()?;
        while self.peek() == Token::Or {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expr::BinaryOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while self.peek() == Token::And {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinaryOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_relational()?;
        while let Token::EqEq | Token::NotEq = self.peek() {
            let op = match self.peek() {
                Token::EqEq => BinOp::Eq,
                Token::NotEq => BinOp::Ne,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_relational()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        while let Token::Lt | Token::Gt | Token::Le | Token::Ge = self.peek() {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Le => BinOp::Le,
                Token::Ge => BinOp::Ge,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        while let Token::Plus | Token::Minus = self.peek() {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        while let Token::Star | Token::Slash | Token::Percent = self.peek() {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Rem,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            Token::Not => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Token::Integer(n) => {
                self.advance();
                Ok(Expr::LiteralInt(n))
            }
            Token::True => {
                self.advance();
                Ok(Expr::LiteralBool(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::LiteralBool(false))
            }
            Token::Ident(s) => {
                let name = s.clone();
                self.advance();
                if self.peek() == Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != Token::RParen {
                        loop {
                            args.push(self.parse_expr()?);
                            match self.peek() {
                                Token::Comma => {
                                    self.advance();
                                    continue;
                                }
                                Token::RParen => break,
                                _ => return Err("expected ',' or ')' in argument list".to_string()),
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call { func: name, args })
                } else if self.peek() == Token::LBrace {
                    self.advance();
                    let mut fields = Vec::new();
                    loop {
                        fields.push(self.parse_expr()?);
                        match self.peek() {
                            Token::Comma => {
                                self.advance();
                                continue;
                            }
                            Token::RBrace => break,
                            _ => return Err("expected ',' or '}' in struct init".to_string()),
                        }
                    }
                    self.expect(Token::RBrace)?;
                    Ok(Expr::StructInit { name, fields })
                } else if self.peek() == Token::Dot {
                    self.advance();
                    let field = match self.advance() {
                        Token::Ident(f) => f,
                        _ => return Err("expected field name after '.'".to_string()),
                    };
                    Ok(Expr::FieldAccess {
                        struct_expr: Box::new(Expr::Variable(name)),
                        field,
                    })
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            _ => Err(format!("unexpected token: {:?}", self.peek())),
        }
    }
}

pub fn parse(tokens: &[Token]) -> Result<Program, String> {
    let mut parser = Parser::new(tokens.to_vec());
    parser.parse_program()
}
