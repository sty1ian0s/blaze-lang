use crate::ast::*;
use std::collections::HashMap;

pub fn check(program: &Program) -> Result<(), String> {
    for (name, func) in &program.functions {
        check_function(program, name, func)?;
    }
    for (_name, _) in &program.structs {}
    Ok(())
}

fn check_function(program: &Program, func_name: &str, func: &Function) -> Result<(), String> {
    let mut scope = Scope::new();
    for param in &func.params {
        scope.declare(param, Type::I32);
    }
    let mut return_type = None;
    for stmt in &func.body {
        match check_stmt(program, &mut scope, stmt, func_name)? {
            Some(t) => {
                if let Stmt::Return(_) = stmt {
                    if let Some(rt) = &return_type {
                        if *rt != t {
                            return Err(format!(
                                "function returns {:?} but return statement gives {:?}",
                                func.return_type, t
                            ));
                        }
                    }
                    return_type = Some(t.clone());
                }
            }
            None => {}
        }
    }
    if let Some(rt) = return_type {
        if rt != func.return_type {
            return Err(format!(
                "function returns {:?} but return statement gives {:?}",
                func.return_type, rt
            ));
        }
    } else if func.return_type != Type::Void {
        return Err(format!(
            "function should return {:?} but has no return statement",
            func.return_type
        ));
    }
    for (name, state) in scope.vars() {
        if state.is_live && !state.is_copy {
            return Err(format!("linear variable `{}` not consumed", name));
        }
    }
    Ok(())
}

fn check_stmt(
    program: &Program,
    scope: &mut Scope,
    stmt: &Stmt,
    current_func: &str,
) -> Result<Option<Type>, String> {
    match stmt {
        Stmt::Let { name, init } => {
            if let Some(init_expr) = init {
                let t = check_expr(program, scope, init_expr, current_func)?;
                scope.declare(name, t);
                scope.set_live(name, true);
            } else {
                return Err("let statement requires initialiser".to_string());
            }
            Ok(None)
        }
        Stmt::Expr(expr) => {
            check_expr(program, scope, expr, current_func)?;
            Ok(None)
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            let cond_type = check_expr(program, scope, cond, current_func)?;
            if cond_type != Type::Bool {
                return Err("if condition must be bool".to_string());
            }
            let mut then_scope = scope.clone();
            let mut else_scope = scope.clone();
            for stmt in then_block {
                check_stmt(program, &mut then_scope, stmt, current_func)?;
            }
            if let Some(else_blk) = else_block {
                for stmt in else_blk {
                    check_stmt(program, &mut else_scope, stmt, current_func)?;
                }
            }
            scope.merge(&then_scope, &else_scope);
            Ok(None)
        }
        Stmt::While { cond, body } => {
            let cond_type = check_expr(program, scope, cond, current_func)?;
            if cond_type != Type::Bool {
                return Err("while condition must be bool".to_string());
            }
            let mut loop_scope = scope.clone();
            for stmt in body {
                check_stmt(program, &mut loop_scope, stmt, current_func)?;
            }
            Ok(None)
        }
        Stmt::Loop { body } => {
            let mut loop_scope = scope.clone();
            for stmt in body {
                check_stmt(program, &mut loop_scope, stmt, current_func)?;
            }
            Ok(None)
        }
        Stmt::Break | Stmt::Continue => Ok(None),
        Stmt::Return(expr) => {
            let t = if let Some(e) = expr {
                check_expr(program, scope, e, current_func)?
            } else {
                Type::Void
            };
            Ok(Some(t))
        }
    }
}

fn check_expr(
    program: &Program,
    scope: &mut Scope,
    expr: &Expr,
    current_func: &str,
) -> Result<Type, String> {
    match expr {
        Expr::LiteralInt(_) => Ok(Type::I32),
        Expr::LiteralBool(_) => Ok(Type::Bool),
        Expr::Variable(name) => {
            scope.use_var(name)?;
            let t = scope.get_type(name)?;
            scope.set_live(name, false);
            Ok(t)
        }
        Expr::BinaryOp { op, left, right } => {
            // Assignment special case
            if *op == BinOp::Eq {
                let var_name = match &**left {
                    Expr::Variable(name) => name.clone(),
                    _ => return Err("left side of assignment must be a variable".to_string()),
                };
                let rhs_type = check_expr(program, scope, right, current_func)?;
                // After assignment, the variable becomes live again
                scope.set_live(&var_name, true);
                return Ok(rhs_type);
            }

            let lt = check_expr(program, scope, left, current_func)?;
            let rt = check_expr(program, scope, right, current_func)?;
            if lt != rt {
                return Err(format!("type mismatch: {:?} vs {:?}", lt, rt));
            }
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                    if lt == Type::I32 {
                        Ok(lt)
                    } else {
                        Err("arithmetic requires integer".to_string())
                    }
                }
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                    Ok(Type::Bool)
                }
                BinOp::And | BinOp::Or => {
                    if lt == Type::Bool {
                        Ok(Type::Bool)
                    } else {
                        Err("logical operator requires bool".to_string())
                    }
                }
            }
        }
        Expr::UnaryOp { op, expr: e } => {
            let t = check_expr(program, scope, e, current_func)?;
            match op {
                UnaryOp::Neg => {
                    if t == Type::I32 {
                        Ok(Type::I32)
                    } else {
                        Err("negation requires integer".to_string())
                    }
                }
                UnaryOp::Not => {
                    if t == Type::Bool {
                        Ok(Type::Bool)
                    } else {
                        Err("not requires bool".to_string())
                    }
                }
            }
        }
        Expr::Call { func, args } => {
            let callee = program
                .functions
                .get(func)
                .ok_or_else(|| format!("undefined function: {}", func))?;
            if args.len() != callee.params.len() {
                return Err(format!(
                    "function {} expects {} arguments, got {}",
                    func,
                    callee.params.len(),
                    args.len()
                ));
            }
            for (i, arg) in args.iter().enumerate() {
                let arg_type = check_expr(program, scope, arg, current_func)?;
                if arg_type != Type::I32 {
                    return Err(format!("argument {} must be i32, got {:?}", i, arg_type));
                }
            }
            Ok(callee.return_type.clone())
        }
        Expr::StructInit { name, fields } => {
            let struct_def = program
                .structs
                .get(name)
                .ok_or_else(|| format!("undefined struct: {}", name))?;
            if fields.len() != struct_def.fields.len() {
                return Err(format!(
                    "struct {} expects {} fields, got {}",
                    name,
                    struct_def.fields.len(),
                    fields.len()
                ));
            }
            for (i, (field_name, field_type)) in struct_def.fields.iter().enumerate() {
                let expr_type = check_expr(program, scope, &fields[i], current_func)?;
                if expr_type != *field_type {
                    return Err(format!(
                        "field {} expects {:?}, got {:?}",
                        field_name, field_type, expr_type
                    ));
                }
            }
            Ok(Type::I32)
        }
        Expr::FieldAccess { struct_expr, field } => match &**struct_expr {
            Expr::Variable(var_name) => {
                let struct_def = program
                    .structs
                    .get(var_name)
                    .ok_or_else(|| format!("variable {} is not a struct", var_name))?;
                let field_types: HashMap<_, _> = struct_def.fields.iter().cloned().collect();
                let field_type = field_types
                    .get(field)
                    .ok_or_else(|| format!("struct {} has no field {}", var_name, field))?;
                scope.use_var(var_name)?;
                scope.set_live(var_name, false);
                Ok(field_type.clone())
            }
            _ => Err("field access only on variable".to_string()),
        },
    }
}

#[derive(Debug, Clone)]
struct VarState {
    typ: Type,
    is_live: bool,
    is_copy: bool,
}

#[derive(Clone)]
struct Scope {
    vars: HashMap<String, VarState>,
}

impl Scope {
    fn new() -> Self {
        Scope {
            vars: HashMap::new(),
        }
    }

    fn declare(&mut self, name: &str, typ: Type) {
        self.vars.insert(
            name.to_string(),
            VarState {
                typ,
                is_live: true,
                is_copy: false,
            },
        );
    }

    fn get_type(&self, name: &str) -> Result<Type, String> {
        if let Some(state) = self.vars.get(name) {
            if !state.is_live {
                return Err(format!("use of moved value: {}", name));
            }
            Ok(state.typ.clone())
        } else {
            Err(format!("undefined variable: {}", name))
        }
    }

    fn use_var(&mut self, name: &str) -> Result<(), String> {
        if let Some(state) = self.vars.get_mut(name) {
            if !state.is_live {
                return Err(format!("use of moved value: {}", name));
            }
            Ok(())
        } else {
            Err(format!("undefined variable: {}", name))
        }
    }

    fn set_live(&mut self, name: &str, live: bool) {
        if let Some(state) = self.vars.get_mut(name) {
            state.is_live = live;
        }
    }

    fn vars(&self) -> Vec<(String, VarState)> {
        self.vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn merge(&mut self, then_scope: &Scope, else_scope: &Scope) {
        for (name, state) in self.vars.iter_mut() {
            let then_live = then_scope
                .vars
                .get(name)
                .map(|s| s.is_live)
                .unwrap_or(false);
            let else_live = else_scope
                .vars
                .get(name)
                .map(|s| s.is_live)
                .unwrap_or(false);
            state.is_live = then_live && else_live;
        }
    }
}
