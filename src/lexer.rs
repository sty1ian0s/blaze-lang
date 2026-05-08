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

    // Identifiers and literals
    Ident(String),
    Integer(i32),

    // Operators and punctuation
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Percent,   // %
    Eq,        // =
    EqEq,      // ==
    NotEq,     // !=
    Lt,        // <
    Gt,        // >
    Le,        // <=
    Ge,        // >=
    And,       // &&
    Or,        // ||
    Not,       // !
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    Comma,     // ,
    Semicolon, // ;
    Colon,     // :
    Dot,       // .
    Arrow,     // ->

    // End of file
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
                    // skip comment line
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        chars.next();
                    }
                } else if chars.peek() == Some(&'*') {
                    // Block comment – not required, but we implement for robustness
                    chars.next(); // skip '*'
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
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::EqEq);
                } else {
                    tokens.push(Token::Eq);
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
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Le);
                } else {
                    tokens.push(Token::Lt);
                }
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Ge);
                } else {
                    tokens.push(Token::Gt);
                }
            }
            '&' => {
                chars.next();
                if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(Token::And);
                } else {
                    return Err("Unexpected '&'".to_string());
                }
            }
            '|' => {
                chars.next();
                if chars.peek() == Some(&'|') {
                    chars.next();
                    tokens.push(Token::Or);
                } else {
                    return Err("Unexpected '|'".to_string());
                }
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
            ' ' | '\t' | '\r' | '\n' => {
                chars.next();
            }
            _ => {
                // String/char literals not supported
                if ch == '"' {
                    return Err("string literals are not supported in Phase 0".to_string());
                } else if ch == '\'' {
                    return Err("character literals are not supported in Phase 0".to_string());
                } else {
                    return Err(format!("unexpected character: {}", ch));
                }
            }
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
