// amber-core/src/optimizer.rs
// Local constant folding: AST -> AST rewrite that evaluates literal-only
// expressions at compile time. Runs after import merging, before emission.
//
// Rules (each mirrors exact VM semantics; anything else is left unfolded so
// runtime errors and type errors behave identically with or without --no-opt):
// - int Binary over two Integer literals (wrapping arithmetic). Div by zero is
//   NEVER folded — it must stay a runtime error, not a compile-time panic.
// - float Binary over two Float literals, evaluated as f32 like the emitter.
// - comparisons over same-type literals -> Boolean (int/float/char ordering;
//   == / != also for bool and strings, compared by content like the VM).
// - String + String over two literals -> one concatenated literal (also shrinks
//   the constant pool and deletes a runtime allocation plus its GC nudge).
// Folding is bottom-up, so nested constants collapse fully.

use crate::ast::{Expr, Op, Stmt};

fn fold_binary(l: Expr, op: &Op, r: Expr) -> Expr {
    match (l, r) {
        (Expr::Integer(a), Expr::Integer(b)) => match op {
            Op::Add => Expr::Integer(a.wrapping_add(b)),
            Op::Sub => Expr::Integer(a.wrapping_sub(b)),
            Op::Mul => Expr::Integer(a.wrapping_mul(b)),
            Op::Div => {
                if b == 0 {
                    Expr::Binary(Box::new(Expr::Integer(a)), op.clone(), Box::new(Expr::Integer(b)))
                } else {
                    Expr::Integer(a.wrapping_div(b))
                }
            }
            Op::LessThan => Expr::Boolean(a < b),
            Op::GreaterThan => Expr::Boolean(a > b),
            Op::Equals => Expr::Boolean(a == b),
            Op::NotEquals => Expr::Boolean(a != b),
            Op::LessEquals => Expr::Boolean(a <= b),
            Op::GreaterEquals => Expr::Boolean(a >= b),
        },
        (Expr::Float(a), Expr::Float(b)) => {
            let (x, y) = (a as f32, b as f32);
            match op {
                Op::Add => Expr::Float((x + y) as f64),
                Op::Sub => Expr::Float((x - y) as f64),
                Op::Mul => Expr::Float((x * y) as f64),
                // f32 division by zero yields inf/nan exactly like the VM's
                // C++ float division (no trap), so folding is faithful.
                Op::Div => Expr::Float((x / y) as f64),
                Op::LessThan => Expr::Boolean(x < y),
                Op::GreaterThan => Expr::Boolean(x > y),
                Op::Equals => Expr::Boolean(x == y),
                Op::NotEquals => Expr::Boolean(x != y),
                Op::LessEquals => Expr::Boolean(x <= y),
                Op::GreaterEquals => Expr::Boolean(x >= y),
            }
        }
        (Expr::Char(a), Expr::Char(b)) => match op {
            Op::LessThan => Expr::Boolean(a < b),
            Op::GreaterThan => Expr::Boolean(a > b),
            Op::Equals => Expr::Boolean(a == b),
            Op::NotEquals => Expr::Boolean(a != b),
            Op::LessEquals => Expr::Boolean(a <= b),
            Op::GreaterEquals => Expr::Boolean(a >= b),
            _ => Expr::Binary(Box::new(Expr::Char(a)), op.clone(), Box::new(Expr::Char(b))),
        },
        (Expr::Boolean(a), Expr::Boolean(b)) => match op {
            Op::Equals => Expr::Boolean(a == b),
            Op::NotEquals => Expr::Boolean(a != b),
            _ => Expr::Binary(Box::new(Expr::Boolean(a)), op.clone(), Box::new(Expr::Boolean(b))),
        },
        (Expr::StringLiteral(a), Expr::StringLiteral(b)) => match op {
            Op::Add => Expr::StringLiteral(a + &b),
            Op::Equals => Expr::Boolean(a == b),
            Op::NotEquals => Expr::Boolean(a != b),
            _ => Expr::Binary(
                Box::new(Expr::StringLiteral(a)),
                op.clone(),
                Box::new(Expr::StringLiteral(b)),
            ),
        },
        (l, r) => Expr::Binary(Box::new(l), op.clone(), Box::new(r)),
    }
}

fn fold_expr_list(exprs: Vec<Expr>) -> Vec<Expr> {
    exprs.into_iter().map(fold_expr).collect()
}

pub fn fold_expr(expr: Expr) -> Expr {
    match expr {
        Expr::Binary(l, op, r) => {
            let l = fold_expr(*l);
            let r = fold_expr(*r);
            fold_binary(l, &op, r)
        }
        Expr::NewArray(size) => Expr::NewArray(Box::new(fold_expr(*size))),
        Expr::ArrayAccess(name, index) => Expr::ArrayAccess(name, Box::new(fold_expr(*index))),
        Expr::Call(name, args) => Expr::Call(name, fold_expr_list(args)),
        Expr::Spawn(name, args) => Expr::Spawn(name, fold_expr_list(args)),
        Expr::MethodCall(obj, name, args) => {
            Expr::MethodCall(Box::new(fold_expr(*obj)), name, fold_expr_list(args))
        }
        Expr::NewInstance(name, args) => Expr::NewInstance(name, fold_expr_list(args)),
        Expr::GetField(obj, field) => Expr::GetField(Box::new(fold_expr(*obj)), field),
        Expr::ListGet(list, index) => {
            Expr::ListGet(Box::new(fold_expr(*list)), Box::new(fold_expr(*index)))
        }
        Expr::ListSize(list) => Expr::ListSize(Box::new(fold_expr(*list))),
        other => other,
    }
}

fn fold_stmt_list(stmts: Vec<Stmt>) -> Vec<Stmt> {
    stmts.into_iter().map(fold_stmt).collect()
}

pub fn fold_stmt(stmt: Stmt) -> Stmt {
    match stmt {
        Stmt::VarDecl(name, ty, init) => Stmt::VarDecl(name, ty, fold_expr(init)),
        Stmt::Assign(name, value) => Stmt::Assign(name, fold_expr(value)),
        Stmt::Return(value) => Stmt::Return(fold_expr(value)),
        Stmt::ArraySet(name, index, value) => {
            Stmt::ArraySet(name, fold_expr(index), fold_expr(value))
        }
        Stmt::Print(value) => Stmt::Print(fold_expr(value)),
        Stmt::Block(stmts) => Stmt::Block(fold_stmt_list(stmts)),
        Stmt::If(cond, then_b, else_b) => Stmt::If(
            fold_expr(cond),
            Box::new(fold_stmt(*then_b)),
            else_b.map(|b| Box::new(fold_stmt(*b))),
        ),
        Stmt::While(cond, body) => {
            Stmt::While(fold_expr(cond), Box::new(fold_stmt(*body)))
        }
        Stmt::Expression(expr) => Stmt::Expression(fold_expr(expr)),
        Stmt::Function(name, params, body, vis, is_static) => {
            Stmt::Function(name, params, fold_stmt_list(body), vis, is_static)
        }
        Stmt::Class(name, parent, fields, methods, implements) => Stmt::Class(
            name,
            parent,
            fields,
            fold_stmt_list(methods),
            implements,
        ),
        Stmt::FieldSet(obj, field, value) => {
            Stmt::FieldSet(Box::new(fold_expr(*obj)), field, fold_expr(value))
        }
        Stmt::ListAdd(list, value) => {
            Stmt::ListAdd(Box::new(fold_expr(*list)), fold_expr(value))
        }
        Stmt::ListSet(list, index, value) => Stmt::ListSet(
            Box::new(fold_expr(*list)),
            Box::new(fold_expr(*index)),
            fold_expr(value),
        ),
        other => other,
    }
}

/// Folds a whole merged program (all modules' statements).
pub fn fold_program(ast: Vec<Stmt>) -> Vec<Stmt> {
    fold_stmt_list(ast)
}
