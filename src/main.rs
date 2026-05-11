mod ast;
mod lexer;
mod parser;
mod semantic;

use std::env;
use std::fs;
use std::process::exit;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: blazei <source.blz>");
        exit(1);
    }
    let source = fs::read_to_string(&args[1]).unwrap_or_else(|e| {
        eprintln!("Error reading file: {}", e);
        exit(1);
    });

    let tokens = match lexer::tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexical error: {}", e);
            exit(1);
        }
    };

    let parse_result = match parser::parse(&tokens) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            exit(1);
        }
    };

    if let Err(e) = semantic::check(parse_result.nodes, parse_result.strings, parse_result.root) {
        eprintln!("Semantic error: {}", e);
        exit(1);
    }

    println!("OK");
}
