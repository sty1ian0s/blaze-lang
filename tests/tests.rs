use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn parse_ok(source: &str) -> bool {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.blz");
    fs::write(&file_path, source).unwrap();
    let output = Command::new("target/debug/blazei")
        .arg(file_path.to_str().unwrap())
        .output()
        .unwrap();
    output.status.success()
}

#[test]
fn parse_empty() {
    assert!(parse_ok(""));
}

#[test]
fn parse_function_no_params() {
    assert!(parse_ok("fn main() { 42 }"));
}

#[test]
fn parse_function_with_params() {
    assert!(parse_ok("fn add(a: i32, b: i32) -> i32 { a + b }"));
}

#[test]
fn parse_generic_function() {
    // Generic parameter list [T], typed parameter, return type
    assert!(parse_ok("fn identity[T](x: T) -> T { x }"));
}

#[test]
fn parse_struct_decl() {
    assert!(parse_ok("struct Point { x: i32; y: i32; }"));
}

#[test]
fn parse_enum_decl() {
    // Enum with generic parameter and simple variants
    assert!(parse_ok("enum Option[T] { Some, None }"));
}

#[test]
fn parse_trait_decl() {
    assert!(parse_ok("trait Clone { fn clone(&self) -> Self; }"));
}

#[test]
fn parse_impl_inherent() {
    assert!(parse_ok(
        "struct Point { x: i32; } impl Point { fn new(x: i32) -> Point { Point { x } } }"
    ));
}

#[test]
fn parse_impl_trait() {
    assert!(parse_ok(
        "trait Foo { fn foo(); } struct Bar; impl Foo for Bar { fn foo() {} }"
    ));
}

#[test]
fn parse_use_decl() {
    assert!(parse_ok("use std::collections::HashMap;"));
}

#[test]
fn parse_mod_decl() {
    assert!(parse_ok("mod foo;"));
}

#[test]
fn parse_actor_decl() {
    assert!(parse_ok(
        "actor Counter { value: i32; fn inc(&mut self) { self.value += 1; } }"
    ));
}

#[test]
fn parse_expr_arithmetic() {
    assert!(parse_ok("fn main() { let x = 1 + 2 * 3; }"));
}

#[test]
fn parse_expr_comparison() {
    assert!(parse_ok("fn main() { let b = 5 > 3 && 2 <= 1; }"));
}

#[test]
fn parse_expr_call() {
    assert!(parse_ok(
        "fn foo() -> i32 { 0 } fn main() { let x = foo(); }"
    ));
}

#[test]
fn parse_struct_init() {
    assert!(parse_ok(
        "struct S { a: i32; } fn main() { let s = S { a: 42 }; }"
    ));
}

#[test]
fn parse_field_access() {
    assert!(parse_ok(
        "struct S { a: i32; } fn main() { let s = S { a: 42 }; let x = s.a; }"
    ));
}

#[test]
fn parse_if_else() {
    assert!(parse_ok("fn main() { if true { 1 } else { 0 } }"));
}

#[test]
fn parse_while_loop() {
    assert!(parse_ok(
        "fn main() { let i = 0; while i < 10 { i = i + 1; } }"
    ));
}

#[test]
fn parse_loop() {
    assert!(parse_ok("fn main() { loop { break; } }"));
}

#[test]
fn parse_match() {
    assert!(parse_ok(
        "fn main() { let x = Some(5); match x { Some(y) => y, None => 0 } }"
    ));
}
