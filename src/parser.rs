use crate::ast::{Node, NodeIdx, NodeTag, StringTable};
use crate::lexer::Token;

pub struct ParseResult {
    pub root: NodeIdx,
    pub nodes: Vec<Node>,
    pub strings: StringTable,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    nodes: Vec<Node>,
    strings: StringTable,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            nodes: Vec::new(),
            strings: StringTable::new(),
        }
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

    fn add_node(&mut self, tag: NodeTag, start: u32, end: u32, payload: [u32; 4]) -> NodeIdx {
        let idx = self.nodes.len() as u32;
        self.nodes.push(Node {
            tag,
            span_start: start,
            span_end: end,
            payload,
        });
        NodeIdx(idx)
    }

    fn intern(&mut self, s: &str) -> u32 {
        self.strings.intern(s)
    }

    fn parse(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        let mut item_indices = Vec::new();
        while self.peek() != Token::Eof {
            let idx = self.parse_item()?;
            item_indices.push(idx);
        }
        let end = self.pos as u32;
        let first = self.nodes.len() as u32;
        for idx in item_indices {
            self.nodes.push(self.nodes[idx.0 as usize].clone());
        }
        let last = self.nodes.len() as u32;
        Ok(self.add_node(NodeTag::Program, start, end, [first, last - first, 0, 0]))
    }

    fn parse_item(&mut self) -> Result<NodeIdx, String> {
        match self.peek() {
            Token::Fn => self.parse_fn_decl(),
            Token::Struct => self.parse_struct_decl(),
            Token::Enum => self.parse_enum_decl(),
            Token::Union => self.parse_union_decl(),
            Token::Trait => self.parse_trait_decl(),
            Token::Impl => self.parse_impl_decl(),
            Token::Mod => self.parse_mod_decl(),
            Token::Use => self.parse_use_decl(),
            Token::Const => self.parse_const_decl(),
            Token::Static => self.parse_static_decl(),
            Token::Extern => self.parse_extern_decl(),
            Token::Actor => self.parse_actor_decl(),
            _ => Err(format!("unexpected item: {:?}", self.peek())),
        }
    }

    fn parse_fn_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Fn)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected function name".to_string()),
        };
        let name_idx = self.intern(&name);
        // Generic parameters: [T, U]
        if self.peek() == Token::LBracket {
            self.skip_balanced(Token::LBracket, Token::RBracket)?;
        }
        self.expect(Token::LParen)?;
        self.skip_params()?;
        self.expect(Token::RParen)?;
        if self.peek() == Token::Arrow {
            self.advance();
            self.skip_type()?;
        }
        if self.peek() == Token::Where {
            self.advance();
            self.skip_where()?;
        }
        self.expect(Token::LBrace)?;
        self.skip_balanced(Token::LBrace, Token::RBrace)?;
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::FnDecl, start, end, [name_idx, 0, 0, 0]))
    }

    fn parse_struct_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Struct)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected struct name".to_string()),
        };
        let name_idx = self.intern(&name);
        if self.peek() == Token::LBracket {
            self.skip_balanced(Token::LBracket, Token::RBracket)?;
        }
        match self.peek() {
            Token::Semicolon => {
                self.advance();
                let end = self.pos as u32;
                Ok(self.add_node(NodeTag::StructDecl, start, end, [name_idx, 0, 0, 0]))
            }
            Token::LParen => {
                self.advance();
                self.skip_balanced(Token::LParen, Token::RParen)?;
                self.expect(Token::Semicolon)?;
                let end = self.pos as u32;
                Ok(self.add_node(NodeTag::StructDecl, start, end, [name_idx, 0, 0, 0]))
            }
            Token::LBrace => {
                self.advance();
                self.skip_balanced(Token::LBrace, Token::RBrace)?;
                let end = self.pos as u32;
                Ok(self.add_node(NodeTag::StructDecl, start, end, [name_idx, 0, 0, 0]))
            }
            _ => Err("expected ';', '(', or '{{' after struct name".to_string()),
        }
    }

    fn parse_enum_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Enum)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected enum name".to_string()),
        };
        let name_idx = self.intern(&name);
        if self.peek() == Token::LBracket {
            self.skip_balanced(Token::LBracket, Token::RBracket)?;
        }
        self.expect(Token::LBrace)?;
        self.skip_balanced(Token::LBrace, Token::RBrace)?;
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::EnumDecl, start, end, [name_idx, 0, 0, 0]))
    }

    fn parse_union_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Union)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected union name".to_string()),
        };
        let name_idx = self.intern(&name);
        if self.peek() == Token::LBracket {
            self.skip_balanced(Token::LBracket, Token::RBracket)?;
        }
        self.expect(Token::LBrace)?;
        self.skip_balanced(Token::LBrace, Token::RBrace)?;
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::UnionDecl, start, end, [name_idx, 0, 0, 0]))
    }

    fn parse_trait_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Trait)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected trait name".to_string()),
        };
        let name_idx = self.intern(&name);
        if self.peek() == Token::LBracket {
            self.skip_balanced(Token::LBracket, Token::RBracket)?;
        }
        if self.peek() == Token::Colon {
            self.advance();
            self.skip_bound()?;
        }
        self.expect(Token::LBrace)?;
        self.skip_balanced(Token::LBrace, Token::RBrace)?;
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::TraitDecl, start, end, [name_idx, 0, 0, 0]))
    }

    fn parse_impl_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Impl)?;
        if self.peek() == Token::LBracket {
            self.skip_balanced(Token::LBracket, Token::RBracket)?;
        }
        if matches!(self.peek(), Token::Ident(_))
            && self.tokens.get(self.pos + 1) == Some(&Token::For)
        {
            self.advance(); // trait name
            self.expect(Token::For)?;
            self.skip_type()?;
        } else {
            self.skip_type()?;
        }
        self.expect(Token::LBrace)?;
        self.skip_balanced(Token::LBrace, Token::RBrace)?;
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::ImplDecl, start, end, [0, 0, 0, 0]))
    }

    fn parse_mod_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Mod)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected module name".to_string()),
        };
        let name_idx = self.intern(&name);
        if self.peek() == Token::Semicolon {
            self.advance();
        } else if self.peek() == Token::LBrace {
            self.advance();
            self.skip_balanced(Token::LBrace, Token::RBrace)?;
        } else {
            return Err("expected ';' or block after mod".to_string());
        }
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::ModDecl, start, end, [name_idx, 0, 0, 0]))
    }

    fn parse_use_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Use)?;
        self.skip_use_path()?;
        self.expect(Token::Semicolon)?;
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::UseDecl, start, end, [0, 0, 0, 0]))
    }

    fn parse_const_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Const)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected constant name".to_string()),
        };
        let name_idx = self.intern(&name);
        if self.peek() == Token::Colon {
            self.advance();
            self.skip_type()?;
        }
        self.expect(Token::Eq)?;
        self.parse_expr()?;
        self.expect(Token::Semicolon)?;
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::ConstDecl, start, end, [name_idx, 0, 0, 0]))
    }

    fn parse_static_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Static)?;
        let is_mut = if self.peek() == Token::Mut {
            self.advance();
            true
        } else {
            false
        };
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected static name".to_string()),
        };
        let name_idx = self.intern(&name);
        if self.peek() == Token::Colon {
            self.advance();
            self.skip_type()?;
        }
        self.expect(Token::Eq)?;
        self.parse_expr()?;
        self.expect(Token::Semicolon)?;
        let end = self.pos as u32;
        let flags = if is_mut { 1 } else { 0 };
        Ok(self.add_node(NodeTag::StaticDecl, start, end, [name_idx, flags, 0, 0]))
    }

    fn parse_extern_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Extern)?;
        if let Token::StringLit(_) = self.peek() {
            self.advance();
        }
        self.expect(Token::LBrace)?;
        self.skip_balanced(Token::LBrace, Token::RBrace)?;
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::ExternDecl, start, end, [0, 0, 0, 0]))
    }

    fn parse_actor_decl(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        self.expect(Token::Actor)?;
        let name = match self.advance() {
            Token::Ident(s) => s,
            _ => return Err("expected actor name".to_string()),
        };
        let name_idx = self.intern(&name);
        if self.peek() == Token::LBracket {
            self.skip_balanced(Token::LBracket, Token::RBracket)?;
        }
        self.expect(Token::LBrace)?;
        self.skip_balanced(Token::LBrace, Token::RBrace)?;
        let end = self.pos as u32;
        Ok(self.add_node(NodeTag::ActorDecl, start, end, [name_idx, 0, 0, 0]))
    }

    // ----- Skipping utilities (iterative, guaranteed termination) -----

    fn skip_balanced(&mut self, open: Token, close: Token) -> Result<(), String> {
        // Consume the opening token first
        self.advance();
        let mut depth = 1;
        while depth > 0 && self.peek() != Token::Eof {
            let tok = self.advance();
            if tok == open {
                depth += 1;
            } else if tok == close {
                depth -= 1;
            }
        }
        if depth != 0 {
            Err(format!("unclosed {:?}", open))
        } else {
            Ok(())
        }
    }

    fn skip_type(&mut self) -> Result<(), String> {
        while self.peek() != Token::Eof {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    self.skip_balanced(Token::LParen, Token::RParen)?;
                }
                Token::LBracket => {
                    self.advance();
                    self.skip_balanced(Token::LBracket, Token::RBracket)?;
                }
                Token::Ident(_) => {
                    self.advance();
                    if self.peek() == Token::Lt {
                        self.advance();
                        self.skip_balanced(Token::Lt, Token::Gt)?;
                    }
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    }

    fn skip_bound(&mut self) -> Result<(), String> {
        self.skip_type()?;
        while self.peek() == Token::Plus {
            self.advance();
            self.skip_type()?;
        }
        Ok(())
    }

    fn skip_where(&mut self) -> Result<(), String> {
        while self.peek() != Token::LBrace && self.peek() != Token::Eof {
            self.advance();
        }
        if self.peek() == Token::Eof {
            Err("unclosed where clause".to_string())
        } else {
            Ok(())
        }
    }

    fn skip_params(&mut self) -> Result<(), String> {
        while self.peek() != Token::RParen && self.peek() != Token::Eof {
            if let Token::Ident(_) = self.peek() {
                self.advance();
            }
            if self.peek() == Token::Colon {
                self.advance();
                self.skip_type()?;
            }
            if self.peek() == Token::Eq {
                self.advance();
                self.parse_expr()?;
            }
            if self.peek() == Token::Comma {
                self.advance();
            }
        }
        Ok(())
    }

    fn skip_use_path(&mut self) -> Result<(), String> {
        if self.peek() == Token::LBrace {
            self.advance();
            self.skip_balanced(Token::LBrace, Token::RBrace)?;
        } else {
            self.skip_path()?;
            if self.peek() == Token::As {
                self.advance();
                if let Token::Ident(_) = self.peek() {
                    self.advance();
                } else {
                    return Err("expected identifier after as".to_string());
                }
            }
        }
        Ok(())
    }

    fn skip_path(&mut self) -> Result<(), String> {
        if let Token::Ident(_) = self.advance() {
            while self.peek() == Token::Colon
                && self.tokens.get(self.pos + 1) == Some(&Token::Colon)
            {
                self.advance();
                self.advance();
                if let Token::Ident(_) = self.advance() {
                } else {
                    return Err("expected identifier after ::".to_string());
                }
            }
        }
        Ok(())
    }

    // ----- Expression parsing (recursive descent, but depth is finite) -----

    fn parse_expr(&mut self) -> Result<NodeIdx, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<NodeIdx, String> {
        let left = self.parse_logical_or()?;
        if self.peek() == Token::Eq {
            let start = left.0;
            self.advance();
            let right = self.parse_assignment()?;
            let end = right.0;
            let payload = [left.0, right.0, 0, 0];
            Ok(self.add_node(NodeTag::BinaryOp, start, end, payload))
        } else {
            Ok(left)
        }
    }

    fn parse_logical_or(&mut self) -> Result<NodeIdx, String> {
        let mut left = self.parse_logical_and()?;
        while self.peek() == Token::Or {
            let op_start = self.pos as u32;
            self.advance();
            let right = self.parse_logical_and()?;
            let payload = [left.0, right.0, 0, 0];
            left = self.add_node(NodeTag::BinaryOp, op_start, op_start + 1, payload);
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<NodeIdx, String> {
        let mut left = self.parse_equality()?;
        while self.peek() == Token::And {
            let op_start = self.pos as u32;
            self.advance();
            let right = self.parse_equality()?;
            let payload = [left.0, right.0, 0, 0];
            left = self.add_node(NodeTag::BinaryOp, op_start, op_start + 1, payload);
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<NodeIdx, String> {
        let mut left = self.parse_relational()?;
        while let Token::EqEq | Token::NotEq = self.peek() {
            let op_start = self.pos as u32;
            self.advance();
            let right = self.parse_relational()?;
            let payload = [left.0, right.0, 0, 0];
            left = self.add_node(NodeTag::BinaryOp, op_start, op_start + 1, payload);
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<NodeIdx, String> {
        let mut left = self.parse_additive()?;
        while let Token::Lt | Token::Gt | Token::Le | Token::Ge = self.peek() {
            let op_start = self.pos as u32;
            self.advance();
            let right = self.parse_additive()?;
            let payload = [left.0, right.0, 0, 0];
            left = self.add_node(NodeTag::BinaryOp, op_start, op_start + 1, payload);
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<NodeIdx, String> {
        let mut left = self.parse_multiplicative()?;
        while let Token::Plus | Token::Minus = self.peek() {
            let op_start = self.pos as u32;
            self.advance();
            let right = self.parse_multiplicative()?;
            let payload = [left.0, right.0, 0, 0];
            left = self.add_node(NodeTag::BinaryOp, op_start, op_start + 1, payload);
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<NodeIdx, String> {
        let mut left = self.parse_unary()?;
        while let Token::Star | Token::Slash | Token::Percent = self.peek() {
            let op_start = self.pos as u32;
            self.advance();
            let right = self.parse_unary()?;
            let payload = [left.0, right.0, 0, 0];
            left = self.add_node(NodeTag::BinaryOp, op_start, op_start + 1, payload);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<NodeIdx, String> {
        match self.peek() {
            Token::Minus | Token::Not | Token::Star | Token::BitAnd => {
                let op_start = self.pos as u32;
                self.advance();
                let expr = self.parse_unary()?;
                let payload = [expr.0, 0, 0, 0];
                Ok(self.add_node(NodeTag::UnaryOp, op_start, op_start + 1, payload))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<NodeIdx, String> {
        let start = self.pos as u32;
        match self.peek() {
            Token::Integer(n) => {
                self.advance();
                Ok(self.add_node(
                    NodeTag::LiteralInt,
                    start,
                    self.pos as u32,
                    [n as u32, 0, 0, 0],
                ))
            }
            Token::True => {
                self.advance();
                Ok(self.add_node(NodeTag::LiteralBool, start, self.pos as u32, [1, 0, 0, 0]))
            }
            Token::False => {
                self.advance();
                Ok(self.add_node(NodeTag::LiteralBool, start, self.pos as u32, [0, 0, 0, 0]))
            }
            Token::StringLit(s) => {
                self.advance();
                let idx = self.intern(&s);
                Ok(self.add_node(
                    NodeTag::LiteralString,
                    start,
                    self.pos as u32,
                    [idx, 0, 0, 0],
                ))
            }
            Token::CharLit(c) => {
                self.advance();
                Ok(self.add_node(
                    NodeTag::LiteralChar,
                    start,
                    self.pos as u32,
                    [c as u32, 0, 0, 0],
                ))
            }
            Token::Ident(name) => {
                self.advance();
                let idx = self.intern(&name);
                if self.peek() == Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != Token::RParen {
                        loop {
                            let arg = self.parse_expr()?;
                            args.push(arg);
                            if self.peek() == Token::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    let end = self.pos as u32;
                    let payload = [
                        idx,
                        args.len() as u32,
                        args.first().map(|x| x.0).unwrap_or(0),
                        0,
                    ];
                    Ok(self.add_node(NodeTag::Call, start, end, payload))
                } else if self.peek() == Token::LBrace {
                    self.advance();
                    let mut fields = Vec::new();
                    loop {
                        let field_expr = self.parse_expr()?;
                        fields.push(field_expr);
                        if self.peek() == Token::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(Token::RBrace)?;
                    let end = self.pos as u32;
                    let payload = [
                        idx,
                        fields.len() as u32,
                        fields.first().map(|x| x.0).unwrap_or(0),
                        0,
                    ];
                    Ok(self.add_node(NodeTag::StructInit, start, end, payload))
                } else if self.peek() == Token::Dot {
                    self.advance();
                    let field = match self.advance() {
                        Token::Ident(f) => f,
                        _ => return Err("expected field name".to_string()),
                    };
                    let field_idx = self.intern(&field);
                    let end = self.pos as u32;
                    let payload = [idx, field_idx, 0, 0];
                    Ok(self.add_node(NodeTag::FieldAccess, start, end, payload))
                } else {
                    Ok(self.add_node(NodeTag::Ident, start, self.pos as u32, [idx, 0, 0, 0]))
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            _ => Err(format!("unexpected token in expression: {:?}", self.peek())),
        }
    }
}

pub fn parse(tokens: &[Token]) -> Result<ParseResult, String> {
    let mut parser = Parser::new(tokens.to_vec());
    let root = parser.parse()?;
    Ok(ParseResult {
        root,
        nodes: parser.nodes,
        strings: parser.strings,
    })
}
