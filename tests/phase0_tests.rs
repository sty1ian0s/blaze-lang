use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn run_source(source: &str) -> (i32, String, String) {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.blz");
    fs::write(&file_path, source).unwrap();
    let output = Command::new("target/debug/blazei")
        .arg(file_path.to_str().unwrap())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    (output.status.code().unwrap_or(1), stdout, stderr)
}

#[test]
fn simple_return() {
    let source = "fn main() { 42 }";
    let (code, stdout, stderr) = run_source(source);
    assert_eq!(code, 0, "stderr: {}", stderr);
    assert_eq!(stdout.trim(), "42");
}

#[cfg(test)]
mod lexer_tests {
    use super::run_source;

    #[test]
    fn lex_basic_keywords_and_identifiers() {
        let source = "fn main() { }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Lexer should accept valid program: {}", stderr);
    }

    #[test]
    fn lex_unsupported_keyword_gives_error() {
        let source = "actor MyActor { }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("unsupported") || stderr.contains("not supported"),
            "Error missing: {}",
            stderr
        );
    }

    #[test]
    fn lex_string_literal_gives_error() {
        let source = "fn main() { let x = \"hello\"; }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("string literals are not supported"),
            "Error missing: {}",
            stderr
        );
    }

    #[test]
    fn lex_char_literal_gives_error() {
        let source = "fn main() { let x = 'a'; }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("character literals are not supported"),
            "Error missing: {}",
            stderr
        );
    }
}

#[cfg(test)]
mod parser_tests {
    use super::run_source;

    #[test]
    fn parse_simple_function() {
        let source = "fn main() { 42 }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Should parse: {}", stderr);
    }

    #[test]
    fn parse_struct_declaration() {
        let source = "struct Point { x: i32; y: i32; } fn main() { Point{ x: 10, y: 20 }; }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Should parse struct: {}", stderr);
    }

    #[test]
    fn parse_struct_fields_out_of_order_is_error() {
        let source = "struct Point { x: i32; y: i32; } fn main() { Point{ y: 20, x: 10 }; }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("order") || stderr.contains("field order"),
            "Should reject out-of-order fields: {}",
            stderr
        );
    }

    #[test]
    fn parse_if_else() {
        let source = "fn main() { if true { 1 } else { 0 } }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Should parse if-else: {}", stderr);
    }

    #[test]
    fn parse_while_loop() {
        let source = "fn main() { let x = 0; while x < 10 { x = x + 1; } }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Should parse while loop: {}", stderr);
    }

    #[test]
    fn parse_loop_break_continue() {
        let source = "fn main() { loop { break; continue; } }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Should parse loop with break/continue: {}", stderr);
    }

    #[test]
    fn parse_return_without_value() {
        let source = "fn main() { return; }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Should parse void return: {}", stderr);
    }
}

#[cfg(test)]
mod semantic_tests {
    use super::run_source;

    #[test]
    fn affine_variable_use_twice_is_error() {
        let source = "fn main() { let x = 5; let y = x + x; y }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("use of moved value") || stderr.contains("consumed"),
            "Should reject double use: {}",
            stderr
        );
    }

    #[test]
    fn affine_variable_reassignment_allowed() {
        let source = "fn main() { let x = 5; let y = x; let x = 10; y + x }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(
            code, 0,
            "Should allow reassigning dead variable: {}",
            stderr
        );
    }

    #[test]
    fn type_mismatch_bool_in_arithmetic() {
        let source = "fn main() { let x = true + 1; }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("type mismatch") || stderr.contains("type error"),
            "Expected type error: {}",
            stderr
        );
    }

    #[test]
    fn type_mismatch_int_in_condition() {
        let source = "fn main() { if 5 { 1 } else { 0 } }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("type mismatch") || stderr.contains("bool"),
            "Should expect bool: {}",
            stderr
        );
    }

    #[test]
    fn undefined_variable_error() {
        let source = "fn main() { x = 5; }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(stderr.contains("undefined"), "Error missing: {}", stderr);
    }

    #[test]
    fn struct_field_access_consumes_struct() {
        let source = "\
            struct Point { x: i32; y: i32; }\
            fn main() {\
                let p = Point{ x: 10, y: 20 };\
                let a = p.x;\
                let b = p.x;\
                a + b\
            }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("use of moved value") || stderr.contains("consumed"),
            "Should reject second field access: {}",
            stderr
        );
    }

    #[test]
    fn struct_field_access_whole_struct_moved() {
        let source = "\
            struct Point { x: i32; y: i32; }\
            fn main() {\
                let p = Point{ x: 10, y: 20 };\
                let a = p;\
                let b = p.x;\
                0\
            }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("use of moved value"),
            "Struct moved as whole should prevent field access: {}",
            stderr
        );
    }

    #[test]
    fn function_argument_consumes_variable() {
        let source = "\
            fn foo(a: i32) { a }\
            fn main() {\
                let x = 5;\
                let y = foo(x);\
                let z = x;\
                y + z\
            }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("use of moved value"),
            "Should reject use after function call: {}",
            stderr
        );
    }

    #[test]
    fn void_function_used_in_expression_error() {
        let source = "\
            fn foo() {}\
            fn main() {\
                foo() + 1\
            }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("void") || stderr.contains("cannot be used"),
            "Should reject void in expression: {}",
            stderr
        );
    }

    #[test]
    fn uninhabited_return_paths_incompatible() {
        let source = "\
            fn main() {\
                if true { 42 } else { return; }\
            }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("return type") || stderr.contains("void"),
            "Should detect conflicting return types: {}",
            stderr
        );
    }
}

#[cfg(test)]
mod interpreter_tests {
    use super::run_source;

    #[test]
    fn simple_return_value() {
        let source = "fn main() { 42 }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Simple return: {}", stderr);
    }

    #[test]
    fn arithmetic_expression() {
        let source = "fn main() { (2 + 3) * 4 }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Arithmetic ok: {}", stderr);
    }

    #[test]
    fn integer_overflow_panics() {
        let source = "fn main() { 2147483647 + 1 }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("overflow") || stderr.contains("panic"),
            "Overflow should panic: {}",
            stderr
        );
    }

    #[test]
    fn division_by_zero_panics() {
        let source = "fn main() { 10 / 0 }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("division by zero") || stderr.contains("panic"),
            "Div by zero should panic: {}",
            stderr
        );
    }

    #[test]
    fn if_expression() {
        let source = "fn main() { if true { 5 } else { 6 } }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "If works: {}", stderr);
    }

    #[test]
    fn while_loop() {
        let source = "\
            fn main() {\
                let i = 0;\
                let sum = 0;\
                while i < 10 {\
                    sum = sum + i;\
                    i = i + 1;\
                }\
                sum\
            }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "While loop works: {}", stderr);
    }

    #[test]
    fn recursion_depth_limit() {
        let source = "\
            fn recurse(n) {\
                if n <= 0 { 0 } else { recurse(n - 1) }\
            }\
            fn main() { recurse(2000) }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("recursion limit") || stderr.contains("1000"),
            "Should exceed recursion limit: {}",
            stderr
        );
    }

    #[test]
    fn struct_construction_and_field_access() {
        let source = "\
            struct Point { x: i32; y: i32; }\
            fn main() {\
                let p = Point{ x: 10, y: 20 };\
                p.x + p.y\
            }";
        let (code, _, stderr) = run_source(source);
        assert_eq!(code, 0, "Struct works: {}", stderr);
    }

    #[test]
    fn affine_consumption_in_loop() {
        let source = "\
            fn main() {\
                let x = 5;\
                loop {\
                    let y = x;\
                    break;\
                }\
                x\
            }";
        let (code, _, stderr) = run_source(source);
        assert_ne!(code, 0);
        assert!(
            stderr.contains("use of moved value"),
            "Variable consumed in loop should be dead after: {}",
            stderr
        );
    }
}
