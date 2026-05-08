pub mod ast;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod semantic;

pub use interpreter::run_main;
pub use lexer::tokenize;
pub use parser::parse;
pub use semantic::check;
