use crate::ast::*;
use std::collections::HashMap;

pub struct Interpreter {
    functions: HashMap<String, Function>,
    structs: HashMap<String, Struct>,
    call_stack: Vec<Frame>,
    recursion_depth: usize,
}

#[derive(Debug, Clone)]
struct Frame {
    locals: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Int(i32),
    Bool(bool),
    Struct { name: String, fields: Vec<Value> },
    Void,
}

impl Interpreter {
    fn new(program: &Program) -> Self {
        Interpreter {
            functions: program.functions.clone(),
            structs: program.structs.clone(),
            call_stack: Vec::new(),
            recursion_depth: 0,
        }
    }

    pub fn run_main(&mut self) -> Result<Option<i32>, String> {
        let main_func = self.functions.get("main").ok_or("no main function")?;
        if !main_func.params.is_empty() {
            return Err("main function must have no parameters".to_string());
        }
        let result = self.call_function("main", &[])?;
        match result {
            Value::Int(i) => Ok(Some(i)),
            Value::Void => Ok(None),
            _ => Err("main returned non-integer value".to_string()),
        }
    }

    fn call_function(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        if self.recursion_depth > 1000 {
            return Err("recursion limit exceeded (1000)".to_string());
        }
        let func = self
            .functions
            .get(name)
            .ok_or_else(|| format!("undefined function: {}", name))?
            .clone();
        if args.len() != func.params.len() {
            return Err(format!(
                "function {} expects {} arguments",
                name,
                func.params.len()
            ));
        }
        self.recursion_depth += 1;
        let mut frame = Frame {
            locals: HashMap::new(),
        };
        for (param, arg) in func.params.iter().zip(args) {
            frame.locals.insert(param.clone(), arg.clone());
        }
        self.call_stack.push(frame);
        let result = self.evaluate_block(&func.body)?;
        self.call_stack.pop();
        self.recursion_depth -= 1;
        Ok(result)
    }

    fn evaluate_block(&mut self, stmts: &[Stmt]) -> Result<Value, String> {
        let mut last_value = Value::Void;
        for stmt in stmts {
            match stmt {
                Stmt::Let { name, init } => {
                    let val = if let Some(init_expr) = init {
                        self.evaluate_expr(init_expr)?
                    } else {
                        return Err("let without initialiser".to_string());
                    };
                    self.current_frame_mut().locals.insert(name.clone(), val);
                }
                Stmt::Expr(expr) => {
                    last_value = self.evaluate_expr(expr)?;
                }
                Stmt::If {
                    cond,
                    then_block,
                    else_block,
                } => {
                    let cond_val = self.evaluate_expr(cond)?;
                    let b = match cond_val {
                        Value::Bool(b) => b,
                        _ => return Err("if condition must be bool".to_string()),
                    };
                    if b {
                        last_value = self.evaluate_block(then_block)?;
                    } else if let Some(else_blk) = else_block {
                        last_value = self.evaluate_block(else_blk)?;
                    }
                }
                Stmt::While { cond, body } => loop {
                    let cond_val = self.evaluate_expr(cond)?;
                    let b = match cond_val {
                        Value::Bool(b) => b,
                        _ => return Err("while condition must be bool".to_string()),
                    };
                    if !b {
                        break;
                    }
                    match self.evaluate_block(body) {
                        Ok(Value::Void) => {}
                        Ok(_) => return Err("while body should not return a value".to_string()),
                        Err(e) => return Err(e),
                    }
                },
                Stmt::Loop { body } => loop {
                    match self.evaluate_block(body) {
                        Ok(Value::Void) => {}
                        Ok(_) => return Err("loop body should not return a value".to_string()),
                        Err(e) => return Err(e),
                    }
                },
                Stmt::Break => {
                    return Ok(Value::Void);
                }
                Stmt::Continue => {
                    continue;
                }
                Stmt::Return(expr) => {
                    let val = if let Some(e) = expr {
                        self.evaluate_expr(e)?
                    } else {
                        Value::Void
                    };
                    return Ok(val);
                }
            }
        }
        Ok(last_value)
    }

    fn evaluate_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::LiteralInt(i) => Ok(Value::Int(*i)),
            Expr::LiteralBool(b) => Ok(Value::Bool(*b)),
            Expr::Variable(name) => {
                let val = self
                    .current_frame()
                    .locals
                    .get(name)
                    .ok_or_else(|| format!("undefined variable: {}", name))?
                    .clone();
                Ok(val)
            }
            Expr::BinaryOp { op, left, right } => {
                if *op == BinOp::Eq {
                    // assignment
                    let lhs = match &**left {
                        Expr::Variable(name) => name.clone(),
                        _ => return Err("left side of assignment must be a variable".to_string()),
                    };
                    let rhs = self.evaluate_expr(right)?;
                    self.current_frame_mut().locals.insert(lhs, rhs.clone());
                    return Ok(rhs);
                }
                let l = self.evaluate_expr(left)?;
                let r = self.evaluate_expr(right)?;
                self.apply_binop(*op, l, r)
            }
            Expr::UnaryOp { op, expr: e } => {
                let v = self.evaluate_expr(e)?;
                match op {
                    UnaryOp::Neg => {
                        if let Value::Int(i) = v {
                            Ok(Value::Int(-i))
                        } else {
                            Err("negation requires integer".to_string())
                        }
                    }
                    UnaryOp::Not => {
                        if let Value::Bool(b) = v {
                            Ok(Value::Bool(!b))
                        } else {
                            Err("not requires bool".to_string())
                        }
                    }
                }
            }
            Expr::Call { func, args } => {
                let arg_vals: Result<Vec<Value>, _> =
                    args.iter().map(|a| self.evaluate_expr(a)).collect();
                let arg_vals = arg_vals?;
                self.call_function(func, &arg_vals)
            }
            Expr::StructInit { name, fields } => {
                let field_vals: Result<Vec<Value>, _> =
                    fields.iter().map(|f| self.evaluate_expr(f)).collect();
                let field_vals = field_vals?;
                Ok(Value::Struct {
                    name: name.clone(),
                    fields: field_vals,
                })
            }
            Expr::FieldAccess { struct_expr, field } => {
                let struct_val = self.evaluate_expr(struct_expr)?;
                match struct_val {
                    Value::Struct {
                        name: sname,
                        fields,
                    } => {
                        let struct_def = self
                            .structs
                            .get(&sname)
                            .ok_or_else(|| format!("undefined struct: {}", sname))?;
                        let idx = struct_def
                            .fields
                            .iter()
                            .position(|(f, _)| f == field)
                            .ok_or_else(|| format!("struct {} has no field {}", sname, field))?;
                        Ok(fields[idx].clone())
                    }
                    _ => Err("field access on non-struct".to_string()),
                }
            }
        }
    }

    fn apply_binop(&mut self, op: BinOp, left: Value, right: Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Int(l), Value::Int(r)) => {
                let result = match op {
                    BinOp::Add => l.checked_add(r).ok_or("integer overflow")?,
                    BinOp::Sub => l.checked_sub(r).ok_or("integer overflow")?,
                    BinOp::Mul => l.checked_mul(r).ok_or("integer overflow")?,
                    BinOp::Div => {
                        if r == 0 {
                            return Err("division by zero".to_string());
                        }
                        l / r
                    }
                    BinOp::Rem => {
                        if r == 0 {
                            return Err("division by zero".to_string());
                        }
                        l % r
                    }
                    BinOp::Eq => (l == r) as i32,
                    BinOp::Ne => (l != r) as i32,
                    BinOp::Lt => (l < r) as i32,
                    BinOp::Gt => (l > r) as i32,
                    BinOp::Le => (l <= r) as i32,
                    BinOp::Ge => (l >= r) as i32,
                    _ => return Err("invalid binary operator for integers".to_string()),
                };
                Ok(Value::Int(result))
            }
            (Value::Bool(l), Value::Bool(r)) => {
                let result = match op {
                    BinOp::And => l && r,
                    BinOp::Or => l || r,
                    BinOp::Eq => l == r,
                    BinOp::Ne => l != r,
                    _ => return Err("invalid binary operator for bool".to_string()),
                };
                Ok(Value::Bool(result))
            }
            _ => Err("type mismatch in binary operation".to_string()),
        }
    }

    fn current_frame(&self) -> &Frame {
        self.call_stack.last().unwrap()
    }

    fn current_frame_mut(&mut self) -> &mut Frame {
        self.call_stack.last_mut().unwrap()
    }
}

pub fn run_main(program: &Program) -> Result<Option<i32>, String> {
    let mut interpreter = Interpreter::new(program);
    interpreter.run_main()
}
