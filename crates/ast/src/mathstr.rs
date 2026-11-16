//! Plain mathematical text rendering (console output, code comments,
//! change detection and document export).
use crate::{BinaryOp, Expr, Statement, UnaryOp};

pub fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Number(v) => crate::latex::format_number(*v),
        Expr::Imaginary => "i".into(),
        Expr::Bool(b) => b.to_string(),
        Expr::Str(s) => format!("\"{s}\""),
        Expr::Symbol(s) => s.clone(),
        Expr::Unary { op, expr } => match op {
            UnaryOp::Neg => format!("-{}", wrap(expr, 16)),
            UnaryOp::Pos => format!("+{}", wrap(expr, 16)),
            UnaryOp::Not => format!("not {}", wrap(expr, 6)),
            UnaryOp::Factorial => format!("{}!", wrap(expr, 20)),
            UnaryOp::Transpose => format!("{}'", wrap(expr, 20)),
        },
        Expr::Binary { op, lhs, rhs } => {
            let p = op.precedence();
            let l = wrap(lhs, p);
            let r = wrap(rhs, p + 1);
            if *op == BinaryOp::Mul {
                format!("{l} {r}")
            } else {
                format!("{l} {} {r}", op.symbol())
            }
        }
        Expr::Call { name, args } => {
            let inner: Vec<String> = args.iter().map(expr_to_string).collect();
            format!("{name}({})", inner.join(", "))
        }
        Expr::Matrix(rows) => {
            let rs: Vec<String> = rows.iter()
                .map(|r| format!("[{}]", r.iter().map(expr_to_string).collect::<Vec<_>>().join(", ")))
                .collect();
            format!("[{}]", rs.join(", "))
        }
        Expr::Vector(items) => format!("[{}]", items.iter().map(expr_to_string).collect::<Vec<_>>().join(", ")),
        Expr::Range { start, step, end } => match step {
            Some(s) => format!("{}..{}..{}", expr_to_string(start), expr_to_string(s), expr_to_string(end)),
            None => format!("{}..{}", expr_to_string(start), expr_to_string(end)),
        },
        Expr::Index { target, indices } => {
            format!("{}[{}]", wrap(target, 20), indices.iter().map(expr_to_string).collect::<Vec<_>>().join(", "))
        }
        Expr::Conditional { cond, then, otherwise } => format!(
            "if {} then {} else {}",
            expr_to_string(cond), expr_to_string(then), expr_to_string(otherwise)
        ),
    }
}

fn wrap(expr: &Expr, parent_prec: u8) -> String {
    let s = expr_to_string(expr);
    if expr.precedence() < parent_prec { format!("({s})") } else { s }
}

pub fn statement_to_string(stmt: &Statement) -> String {
    match stmt {
        Statement::Assign { name, value } => format!("{name} := {}", expr_to_string(value)),
        Statement::FuncDef { name, params, body } => format!("{name}({}) := {}", params.join(", "), expr_to_string(body)),
        Statement::Expr(e) => expr_to_string(e),
    }
}
