#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeIdx(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTag {
    Program,
    FnDecl,
    StructDecl,
    EnumDecl,
    UnionDecl,
    TraitDecl,
    ImplDecl,
    ModDecl,
    UseDecl,
    ConstDecl,
    StaticDecl,
    ExternDecl,
    ActorDecl,
    Block,
    LetStmt,
    IfStmt,
    WhileStmt,
    ForStmt,
    LoopStmt,
    MatchStmt,
    ReturnStmt,
    ExprStmt,
    BinaryOp,
    UnaryOp,
    Call,
    Ident,
    LiteralInt,
    LiteralBool,
    LiteralString,
    LiteralChar,
    StructInit,
    FieldAccess,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub tag: NodeTag,
    pub span_start: u32,
    pub span_end: u32,
    pub payload: [u32; 4],
}

pub struct StringTable {
    strings: Vec<String>,
}

impl StringTable {
    pub fn new() -> Self {
        StringTable {
            strings: Vec::new(),
        }
    }
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(pos) = self.strings.iter().position(|x| x == s) {
            pos as u32
        } else {
            self.strings.push(s.to_string());
            (self.strings.len() - 1) as u32
        }
    }
    pub fn get(&self, idx: u32) -> &str {
        &self.strings[idx as usize]
    }
}
