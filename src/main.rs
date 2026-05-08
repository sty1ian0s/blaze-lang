use blazei::{check, parse, run_main, tokenize};
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

    let tokens = match tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexical error: {}", e);
            exit(1);
        }
    };

    let program = match parse(&tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            exit(1);
        }
    };

    if let Err(e) = check(&program) {
        eprintln!("Semantic error: {}", e);
        exit(1);
    }

    match run_main(&program) {
        Ok(Some(val)) => println!("{}", val),
        Ok(None) => {}
        Err(e) => {
            eprintln!("Runtime error: {}", e);
            exit(1);
        }
    }
}
