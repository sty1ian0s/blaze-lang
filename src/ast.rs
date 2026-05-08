use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    Bool,
    Void,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: HashMap<String, Function>,
    pub structs: HashMap<String, Struct>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub return_type: Type,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        init: Option<Expr>,
    },
    Expr(Expr),
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Loop {
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    LiteralInt(i32),
    LiteralBool(bool),
    Variable(String),
    BinaryOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        func: String,
        args: Vec<Expr>,
    },
    StructInit {
        name: String,
        fields: Vec<Expr>,
    },
    FieldAccess {
        struct_expr: Box<Expr>,
        field: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}
