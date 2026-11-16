//! LaTeX generation from the AST (spec sections 6 and 22).
use crate::{BinaryOp, Expr, Statement, UnaryOp};

pub fn format_number(v: f64) -> String {
    if !v.is_finite() {
        return format!("{v}");
    }
    if v == 0.0 {
        return "0".to_string();
    }
    let a = v.abs();
    if a >= 1e6 || a < 1e-4 {
        let s = format!("{:.5e}", v);
        let mut parts = s.split('e');
        let mant = parts.next().unwrap_or("0").trim_end_matches('0').trim_end_matches('.').to_string();
        let exp = parts.next().unwrap_or("0");
        let exp_val: i32 = exp.parse().unwrap_or(0);
        return format!("{mant}e{exp_val}");
    }
    let decimals = (6.0 - a.log10().floor() - 1.0).max(0.0).min(12.0) as usize;
    let s = format!("{:.*}", decimals, v);
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

pub fn greek(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "\\alpha", "beta" => "\\beta", "gamma" => "\\gamma", "delta" => "\\delta",
        "epsilon" => "\\epsilon", "zeta" => "\\zeta", "eta" => "\\eta", "theta" => "\\theta",
        "iota" => "\\iota", "kappa" => "\\kappa", "lambda" => "\\lambda", "mu" => "\\mu",
        "nu" => "\\nu", "xi" => "\\xi", "rho" => "\\rho", "sigma" => "\\sigma",
        "tau" => "\\tau", "upsilon" => "\\upsilon", "phi" => "\\phi", "chi" => "\\chi",
        "psi" => "\\psi", "omega" => "\\omega", "Gamma" => "\\Gamma", "Delta" => "\\Delta",
        "Theta" => "\\Theta", "Lambda" => "\\Lambda", "Xi" => "\\Xi", "Pi" => "\\Pi",
        "Sigma" => "\\Sigma", "Phi" => "\\Phi", "Psi" => "\\Psi", "Omega" => "\\Omega",
        "pi" => "\\pi", "infty" => "\\infty",
        _ => return None,
    })
}

fn symbol(name: &str) -> String {
    if let Some(g) = greek(name) {
        return g.to_string();
    }
    if let Some((base, sub)) = name.split_once('_') {
        let base_tex = greek(base).map(|s| s.to_string()).unwrap_or_else(|| escape(base));
        return format!("{base_tex}_{{{}}}", escape(sub));
    }
    escape(name)
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\backslash ")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('%', "\\%")
        .replace('&', "\\&")
        .replace('#', "\\#")
        .replace('_', "\\_")
}

pub fn expr_to_latex(expr: &Expr) -> String {
    match expr {
        Expr::Number(v) => format_number(*v),
        Expr::Imaginary => "i".into(),
        Expr::Bool(b) => (if *b { "\\mathrm{true}" } else { "\\mathrm{false}" }).into(),
        Expr::Str(s) => format!("\\text{{{s}}}"),
        Expr::Symbol(s) => symbol(s),
        Expr::Unary { op, expr } => match op {
            UnaryOp::Neg => format!("-{}", paren(expr, 16)),
            UnaryOp::Pos => format!("+{}", paren(expr, 16)),
            UnaryOp::Not => format!("\\neg {}", paren(expr, 6)),
            UnaryOp::Factorial => format!("{}!", paren(expr, 20)),
            UnaryOp::Transpose => format!("{}^{{T}}", paren(expr, 20)),
        },
        Expr::Binary { op, lhs, rhs } => match op {
            BinaryOp::Div => format!("\\frac{{{}}}{{{}}}", expr_to_latex(lhs), expr_to_latex(rhs)),
            BinaryOp::Pow | BinaryOp::ElemPow => format!("{}^{{{}}}", paren(lhs, 19), expr_to_latex(rhs)),
            BinaryOp::Mul => format!("{} \\cdot {}", paren(lhs, 12), paren(rhs, 13)),
            BinaryOp::ElemMul => format!("{} \\odot {}", paren(lhs, 12), paren(rhs, 13)),
            BinaryOp::ElemDiv => format!("{} \\oslash {}", paren(lhs, 12), paren(rhs, 13)),
            BinaryOp::Add | BinaryOp::Sub => format!("{} {} {}", paren(lhs, 10), op.symbol(), paren(rhs, 11)),
            BinaryOp::Mod => format!("{} \\bmod {}", paren(lhs, 12), paren(rhs, 13)),
            BinaryOp::And => format!("{} \\wedge {}", paren(lhs, 4), paren(rhs, 5)),
            BinaryOp::Or => format!("{} \\vee {}", paren(lhs, 2), paren(rhs, 3)),
            BinaryOp::Eq => format!("{} = {}", paren(lhs, 6), paren(rhs, 7)),
            BinaryOp::Ne => format!("{} \\neq {}", paren(lhs, 6), paren(rhs, 7)),
            BinaryOp::Lt => format!("{} < {}", paren(lhs, 6), paren(rhs, 7)),
            BinaryOp::Le => format!("{} \\leq {}", paren(lhs, 6), paren(rhs, 7)),
            BinaryOp::Gt => format!("{} > {}", paren(lhs, 6), paren(rhs, 7)),
            BinaryOp::Ge => format!("{} \\geq {}", paren(lhs, 6), paren(rhs, 7)),
        },
        Expr::Call { name, args } => call_latex(name, args),
        Expr::Matrix(rows) => {
            let body: Vec<String> = rows.iter()
                .map(|r| r.iter().map(expr_to_latex).collect::<Vec<_>>().join(" & "))
                .collect();
            format!("\\begin{{bmatrix}}{}\\end{{bmatrix}}", body.join(" \\\\ "))
        }
        Expr::Vector(items) => {
            let body: Vec<String> = items.iter().map(expr_to_latex).collect();
            format!("\\begin{{bmatrix}}{}\\end{{bmatrix}}", body.join(" \\\\ "))
        }
        Expr::Range { start, step, end } => match step {
            Some(s) => format!("{}, {} \\ldots {}", expr_to_latex(start), expr_to_latex(s), expr_to_latex(end)),
            None => format!("{} \\ldots {}", expr_to_latex(start), expr_to_latex(end)),
        },
        Expr::Index { target, indices } => format!(
            "{}_{{{}}}", paren(target, 20),
            indices.iter().map(expr_to_latex).collect::<Vec<_>>().join(",")
        ),
        Expr::Conditional { cond, then, otherwise } => format!(
            "\\begin{{cases}} {} & \\text{{if }} {} \\\\ {} & \\text{{otherwise}} \\end{{cases}}",
            expr_to_latex(then), expr_to_latex(cond), expr_to_latex(otherwise)
        ),
    }
}

fn call_latex(name: &str, args: &[Expr]) -> String {
    let a: Vec<String> = args.iter().map(expr_to_latex).collect();
    match name {
        "sqrt" => format!("\\sqrt{{{}}}", a.first().cloned().unwrap_or_default()),
        "abs" => format!("\\left|{}\\right|", a.first().cloned().unwrap_or_default()),
        "diff" if a.len() >= 2 => format!("\\frac{{d}}{{d {}}}\\left({}\\right)", a[1], a[0]),
        "integrate" if a.len() >= 3 => format!(
            "\\int_{{{}}}^{{{}}} {} \\, d{}", a[1], a[2], a[0],
            a.get(3).cloned().unwrap_or_else(|| "x".into())
        ),
        "integrate" if a.len() == 2 => format!("\\int {} \\, d{}", a[0], a[1]),
        "sum" => format!("\\sum {}", a.join(", ")),
        "prod" => format!("\\prod {}", a.join(", ")),
        "limit" => format!(
            "\\lim_{{{} \\to {}}} {}",
            a.get(1).cloned().unwrap_or_default(),
            a.get(2).cloned().unwrap_or_default(),
            a.first().cloned().unwrap_or_default()
        ),
        "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "exp" | "ln" | "log" | "min" | "max" => {
            format!("\\{}\\left({}\\right)", name, a.join(", "))
        }
        _ => format!("\\mathrm{{{}}}\\left({}\\right)", escape(name), a.join(", ")),
    }
}

fn paren(expr: &Expr, parent_prec: u8) -> String {
    let s = expr_to_latex(expr);
    if expr.precedence() < parent_prec { format!("\\left({s}\\right)") } else { s }
}

pub fn statement_to_latex(stmt: &Statement) -> String {
    match stmt {
        Statement::Assign { name, value } => format!("{} := {}", symbol(name), expr_to_latex(value)),
        Statement::FuncDef { name, params, body } => format!(
            "{}\\left({}\\right) := {}", symbol(name),
            params.iter().map(|p| symbol(p)).collect::<Vec<_>>().join(", "),
            expr_to_latex(body)
        ),
        Statement::Expr(e) => expr_to_latex(e),
    }
}
