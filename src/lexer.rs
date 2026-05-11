use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn,
    If,
    Else,
    While,
    Loop,
    Let,
    Return,
    Struct,
    Break,
    Continue,
    True,
    False,
    Pub,
    Const,
    Static,
    For,
    Match,
    Enum,
    Union,
    Unsafe,
    Use,
    Mod,
    Extern,
    Actor,
    Trait,
    Impl,
    Where,
    Effect,
    Dyn,
    Async,
    Await,
    Move,
    Ref,
    Seq,
    Unroll,
    Guard,
    Defer,
    Try,
    Catch,
    With,
    Type,
    As,
    Mut,
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    At,
    Hash,
    Tilde,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    Dot,
    Arrow,
    FatArrow,
    Question,
    AtAt,
    // Literals and identifiers
    Ident(String),
    Integer(i32),
    Float(f64),
    StringLit(String),
    CharLit(char),
    Eof,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&ch) = chars.peek() {
        match ch {
            'a'..='z' | 'A'..='Z' | '_' => {
                let ident = read_ident(&mut chars);
                let token = match ident.as_str() {
                    "fn" => Token::Fn,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "loop" => Token::Loop,
                    "let" => Token::Let,
                    "return" => Token::Return,
                    "struct" => Token::Struct,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "true" => Token::True,
                    "false" => Token::False,
                    "pub" => Token::Pub,
                    "const" => Token::Const,
                    "static" => Token::Static,
                    "for" => Token::For,
                    "match" => Token::Match,
                    "enum" => Token::Enum,
                    "union" => Token::Union,
                    "unsafe" => Token::Unsafe,
                    "use" => Token::Use,
                    "mod" => Token::Mod,
                    "extern" => Token::Extern,
                    "actor" => Token::Actor,
                    "trait" => Token::Trait,
                    "impl" => Token::Impl,
                    "where" => Token::Where,
                    "effect" => Token::Effect,
                    "dyn" => Token::Dyn,
                    "async" => Token::Async,
                    "await" => Token::Await,
                    "move" => Token::Move,
                    "ref" => Token::Ref,
                    "seq" => Token::Seq,
                    "unroll" => Token::Unroll,
                    "guard" => Token::Guard,
                    "defer" => Token::Defer,
                    "try" => Token::Try,
                    "catch" => Token::Catch,
                    "with" => Token::With,
                    "type" => Token::Type,
                    "as" => Token::As,
                    "mut" => Token::Mut,
                    _ => Token::Ident(ident),
                };
                tokens.push(token);
            }
            '0'..='9' => {
                let num = read_number(&mut chars);
                tokens.push(Token::Integer(num));
            }
            '+' => {
                tokens.push(Token::Plus);
                chars.next();
            }
            '-' => {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::Arrow);
                } else {
                    tokens.push(Token::Minus);
                }
            }
            '*' => {
                tokens.push(Token::Star);
                chars.next();
            }
            '/' => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        chars.next();
                    }
                } else if chars.peek() == Some(&'*') {
                    chars.next();
                    let mut depth = 1;
                    while let Some(c) = chars.next() {
                        if c == '/' && chars.peek() == Some(&'*') {
                            chars.next();
                            depth += 1;
                        } else if c == '*' && chars.peek() == Some(&'/') {
                            chars.next();
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                } else {
                    tokens.push(Token::Slash);
                }
            }
            '%' => {
                tokens.push(Token::Percent);
                chars.next();
            }
            '=' => {
                chars.next();
                match chars.peek() {
                    Some(&'=') => {
                        chars.next();
                        tokens.push(Token::EqEq);
                    }
                    Some(&'>') => {
                        chars.next();
                        tokens.push(Token::FatArrow);
                    }
                    _ => tokens.push(Token::Eq),
                }
            }
            '!' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::NotEq);
                } else {
                    tokens.push(Token::Not);
                }
            }
            '<' => {
                chars.next();
                match chars.peek() {
                    Some(&'=') => {
                        chars.next();
                        tokens.push(Token::Le);
                    }
                    Some(&'<') => {
                        chars.next();
                        tokens.push(Token::Shl);
                    }
                    _ => tokens.push(Token::Lt),
                }
            }
            '>' => {
                chars.next();
                match chars.peek() {
                    Some(&'=') => {
                        chars.next();
                        tokens.push(Token::Ge);
                    }
                    Some(&'>') => {
                        chars.next();
                        tokens.push(Token::Shr);
                    }
                    _ => tokens.push(Token::Gt),
                }
            }
            '&' => {
                chars.next();
                if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(Token::And);
                } else {
                    tokens.push(Token::BitAnd);
                }
            }
            '|' => {
                chars.next();
                if chars.peek() == Some(&'|') {
                    chars.next();
                    tokens.push(Token::Or);
                } else {
                    tokens.push(Token::BitOr);
                }
            }
            '^' => {
                tokens.push(Token::BitXor);
                chars.next();
            }
            '~' => {
                tokens.push(Token::Tilde);
                chars.next();
            }
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            '{' => {
                tokens.push(Token::LBrace);
                chars.next();
            }
            '}' => {
                tokens.push(Token::RBrace);
                chars.next();
            }
            '[' => {
                tokens.push(Token::LBracket);
                chars.next();
            }
            ']' => {
                tokens.push(Token::RBracket);
                chars.next();
            }
            ',' => {
                tokens.push(Token::Comma);
                chars.next();
            }
            ';' => {
                tokens.push(Token::Semicolon);
                chars.next();
            }
            ':' => {
                tokens.push(Token::Colon);
                chars.next();
            }
            '.' => {
                tokens.push(Token::Dot);
                chars.next();
            }
            '?' => {
                tokens.push(Token::Question);
                chars.next();
            }
            '@' => {
                chars.next();
                if chars.peek() == Some(&'@') {
                    chars.next();
                    tokens.push(Token::AtAt);
                } else {
                    tokens.push(Token::At);
                }
            }
            '#' => {
                tokens.push(Token::Hash);
                chars.next();
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        break;
                    }
                    if c == '\\' {
                        chars.next();
                        if let Some(&esc) = chars.peek() {
                            match esc {
                                'n' => s.push('\n'),
                                'r' => s.push('\r'),
                                't' => s.push('\t'),
                                '\\' => s.push('\\'),
                                '"' => s.push('"'),
                                _ => s.push(esc),
                            }
                            chars.next();
                        }
                    } else {
                        s.push(c);
                        chars.next();
                    }
                }
                if chars.peek() == Some(&'"') {
                    chars.next();
                }
                tokens.push(Token::StringLit(s));
            }
            '\'' => {
                chars.next();
                let c = chars.next().unwrap_or('\0');
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    tokens.push(Token::CharLit(c));
                } else {
                    return Err("invalid character literal".to_string());
                }
            }
            ' ' | '\t' | '\r' | '\n' => {
                chars.next();
            }
            _ => return Err(format!("unexpected character: {}", ch)),
        }
    }
    tokens.push(Token::Eof);
    Ok(tokens)
}

fn read_ident(chars: &mut Peekable<Chars>) -> String {
    let mut s = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_alphabetic() || c == '_' || (s.len() > 0 && c.is_ascii_digit()) {
            s.push(c);
            chars.next();
        } else {
            break;
        }
    }
    s
}

fn read_number(chars: &mut Peekable<Chars>) -> i32 {
    let mut s = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            s.push(c);
            chars.next();
        } else {
            break;
        }
    }
    s.parse().unwrap()
}
