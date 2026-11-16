//! Authoritative mathematical AST. Independent of Python syntax (spec section 6).
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub mod latex;
pub mod mathstr;

pub use latex::{expr_to_latex, statement_to_latex};
pub use mathstr::{expr_to_string, statement_to_string};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
    Factorial,
    Transpose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
    ElemMul,
    ElemDiv,
    ElemPow,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl BinaryOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Pow => "^",
            BinaryOp::Mod => "mod",
            BinaryOp::ElemMul => ".*",
            BinaryOp::ElemDiv => "./",
            BinaryOp::ElemPow => ".^",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
            BinaryOp::Eq => "=",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
        }
    }

    pub fn precedence(self) -> u8 {
        match self {
            BinaryOp::Or => 2,
            BinaryOp::And => 4,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 6,
            BinaryOp::Add | BinaryOp::Sub => 10,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::ElemMul | BinaryOp::ElemDiv => 12,
            BinaryOp::Pow | BinaryOp::ElemPow => 18,
        }
    }

    pub fn is_comparison(self) -> bool {
        matches!(self, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Number(f64),
    Imaginary,
    Bool(bool),
    Str(String),
    Symbol(String),
    Unary { op: UnaryOp, expr: Box<Expr> },
    Binary { op: BinaryOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
    Matrix(Vec<Vec<Expr>>),
    Vector(Vec<Expr>),
    Range { start: Box<Expr>, step: Option<Box<Expr>>, end: Box<Expr> },
    Index { target: Box<Expr>, indices: Vec<Expr> },
    Conditional { cond: Box<Expr>, then: Box<Expr>, otherwise: Box<Expr> },
}

impl Expr {
    pub fn num(v: f64) -> Self { Expr::Number(v) }
    pub fn sym(name: impl Into<String>) -> Self { Expr::Symbol(name.into()) }
    pub fn bin(op: BinaryOp, lhs: Expr, rhs: Expr) -> Self {
        Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }
    }
    pub fn un(op: UnaryOp, expr: Expr) -> Self { Expr::Unary { op, expr: Box::new(expr) } }
    pub fn call(name: impl Into<String>, args: Vec<Expr>) -> Self { Expr::Call { name: name.into(), args } }

    pub fn precedence(&self) -> u8 {
        match self {
            Expr::Binary { op, .. } => op.precedence(),
            Expr::Unary { .. } => 16,
            Expr::Range { .. } => 8,
            Expr::Conditional { .. } => 1,
            _ => 20,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Assign { name: String, value: Expr },
    FuncDef { name: String, params: Vec<String>, body: Expr },
    Expr(Expr),
}

impl Statement {
    pub fn defined_name(&self) -> Option<&str> {
        match self {
            Statement::Assign { name, .. } => Some(name),
            Statement::FuncDef { name, .. } => Some(name),
            Statement::Expr(_) => None,
        }
    }

    pub fn value_expr(&self) -> &Expr {
        match self {
            Statement::Assign { value, .. } => value,
            Statement::FuncDef { body, .. } => body,
            Statement::Expr(e) => e,
        }
    }

    pub fn set_value(&mut self, e: Expr) {
        match self {
            Statement::Assign { value, .. } => *value = e,
            Statement::FuncDef { body, .. } => *body = e,
            Statement::Expr(inner) => *inner = e,
        }
    }

    /// Left-hand side text for worksheet display.
    pub fn display_lhs(&self) -> Option<String> {
        match self {
            Statement::Assign { name, .. } => Some(format!("{name} := ")),
            Statement::FuncDef { name, params, .. } => Some(format!("{name}({}) := ", params.join(", "))),
            Statement::Expr(_) => None,
        }
    }
}

/// Calls that must be executed by the symbolic (SymPy) backend.
pub const SYMBOLIC_FUNCTIONS: &[&str] =
    &["diff", "limit", "simplify", "expand", "factor", "solve_eq", "dsolve", "series"];

/// Calls that require the Python numerical backend (SciPy / Pandas).
pub const PYTHON_ONLY_FUNCTIONS: &[&str] =
    &["read_excel", "dataframe", "bvp_solve", "curve_fit", "minimize_constrained", "linprog"];

pub fn is_symbolic_call(name: &str, arity: usize) -> bool {
    if SYMBOLIC_FUNCTIONS.contains(&name) {
        return true;
    }
    name == "integrate" && arity == 2
}

pub fn is_python_only_call(name: &str) -> bool {
    PYTHON_ONLY_FUNCTIONS.contains(&name)
}

pub fn walk(expr: &Expr, f: &mut impl FnMut(&Expr)) {
    f(expr);
    match expr {
        Expr::Unary { expr, .. } => walk(expr, f),
        Expr::Binary { lhs, rhs, .. } => { walk(lhs, f); walk(rhs, f); }
        Expr::Call { args, .. } => { for a in args { walk(a, f); } }
        Expr::Matrix(rows) => { for r in rows { for c in r { walk(c, f); } } }
        Expr::Vector(items) => { for c in items { walk(c, f); } }
        Expr::Range { start, step, end } => {
            walk(start, f);
            if let Some(s) = step { walk(s, f); }
            walk(end, f);
        }
        Expr::Index { target, indices } => {
            walk(target, f);
            for i in indices { walk(i, f); }
        }
        Expr::Conditional { cond, then, otherwise } => { walk(cond, f); walk(then, f); walk(otherwise, f); }
        _ => {}
    }
}

pub fn free_symbols(expr: &Expr) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk(expr, &mut |e| {
        if let Expr::Symbol(s) = e {
            out.insert(s.clone());
        }
    });
    out
}

pub fn called_functions(expr: &Expr) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk(expr, &mut |e| {
        if let Expr::Call { name, .. } = e {
            out.insert(name.clone());
        }
    });
    out
}

/// Dependency set of a statement: free symbols plus called function names,
/// minus function-local parameters.
pub fn statement_dependencies(stmt: &Statement) -> BTreeSet<String> {
    let mut deps = free_symbols(stmt.value_expr());
    deps.extend(called_functions(stmt.value_expr()));
    if let Statement::FuncDef { params, .. } = stmt {
        for p in params {
            deps.remove(p);
        }
    }
    deps
}

pub fn requires_symbolic(stmt: &Statement) -> bool {
    let mut found = false;
    walk(stmt.value_expr(), &mut |e| {
        if let Expr::Call { name, args } = e {
            if is_symbolic_call(name, args.len()) || is_python_only_call(name) {
                found = true;
            }
        }
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependencies_of_assignment() {
        let stmt = Statement::Assign {
            name: "c".into(),
            value: Expr::call("sqrt", vec![Expr::bin(BinaryOp::Add,
                Expr::bin(BinaryOp::Pow, Expr::sym("a"), Expr::num(2.0)),
                Expr::bin(BinaryOp::Pow, Expr::sym("b"), Expr::num(2.0)))]),
        };
        let deps = statement_dependencies(&stmt);
        assert!(deps.contains("a"));
        assert!(deps.contains("b"));
        assert!(deps.contains("sqrt"));
    }

    #[test]
    fn function_params_are_local() {
        let stmt = Statement::FuncDef {
            name: "f".into(),
            params: vec!["x".into()],
            body: Expr::bin(BinaryOp::Mul, Expr::sym("a"), Expr::sym("x")),
        };
        let deps = statement_dependencies(&stmt);
        assert!(deps.contains("a"));
        assert!(!deps.contains("x"));
    }
}
