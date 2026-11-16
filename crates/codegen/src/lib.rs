//! Rust -> Python code generation (spec section 7). The AST stays authoritative;
//! Python text is a derived artefact that can be inspected, edited and exported.
pub mod emitter;

/// Native helpers mirrored in python/scripts/aaiyyan_runtime.py.
pub const NATIVE_HELPERS: &[&str] = &[
    "sec", "csc", "cot", "asec", "acsc", "acot", "sech", "csch", "coth", "sinc",
    "frac", "clamp", "gcd", "lcm", "nroot", "log1p", "expm1",
    "geomean", "harmmean", "mode", "percentile", "corr", "cov", "rms", "skew", "kurt",
    "augment", "stack", "submatrix", "row", "col", "last", "vec", "reshape",
    "cumsum", "cumprod", "logspace", "linterp", "linfit", "expfit", "logfit", "pwrfit",
    "maximize", "newton", "runif", "rnorm", "seed", "unitsOf", "SIUnitsOf",
];

use aaiyyan_ast::{BinaryOp, Expr, Statement};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    Unknown,
    Scalar,
    Vector,
    Matrix,
    Symbolic,
    Table,
    Function,
}

impl Ty {
    pub fn is_array(self) -> bool { matches!(self, Ty::Vector | Ty::Matrix) }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub use_units: bool,
    pub capture_results: bool,
    pub comments: bool,
    pub standalone: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { use_units: true, capture_results: true, comments: true, standalone: false }
    }
}

#[derive(Clone, Debug)]
pub struct GeneratedScript {
    pub code: String,
    pub libraries: Vec<String>,
    pub definitions: Vec<String>,
}

/// Order-aware type inference used to distinguish matrix multiplication (`@`)
/// from element-wise multiplication (`*`).
pub fn infer_types(stmts: &[Statement]) -> BTreeMap<String, Ty> {
    let mut types: BTreeMap<String, Ty> = BTreeMap::new();
    for _ in 0..2 {
        for stmt in stmts {
            match stmt {
                Statement::Assign { name, value } => {
                    let t = expr_type(value, &types);
                    types.insert(name.clone(), t);
                }
                Statement::FuncDef { name, .. } => { types.insert(name.clone(), Ty::Function); }
                Statement::Expr(_) => {}
            }
        }
    }
    types
}

pub fn expr_type(expr: &Expr, types: &BTreeMap<String, Ty>) -> Ty {
    match expr {
        Expr::Number(_) | Expr::Imaginary | Expr::Bool(_) => Ty::Scalar,
        Expr::Str(_) => Ty::Unknown,
        Expr::Symbol(s) => types.get(s).copied().unwrap_or(Ty::Scalar),
        Expr::Matrix(_) => Ty::Matrix,
        Expr::Vector(_) | Expr::Range { .. } => Ty::Vector,
        Expr::Index { .. } => Ty::Scalar,
        Expr::Unary { expr, .. } => expr_type(expr, types),
        Expr::Conditional { then, .. } => expr_type(then, types),
        Expr::Binary { op, lhs, rhs } => {
            let a = expr_type(lhs, types);
            let b = expr_type(rhs, types);
            match *op {
                BinaryOp::Mul => match (a, b) {
                    (Ty::Matrix, Ty::Matrix) | (Ty::Matrix, Ty::Vector) => Ty::Matrix,
                    (Ty::Vector, Ty::Matrix) => Ty::Vector,
                    (Ty::Vector, Ty::Vector) => Ty::Scalar,
                    (Ty::Matrix, _) | (_, Ty::Matrix) => Ty::Matrix,
                    (Ty::Vector, _) | (_, Ty::Vector) => Ty::Vector,
                    _ => Ty::Scalar,
                },
                BinaryOp::ElemMul | BinaryOp::ElemDiv | BinaryOp::ElemPow => {
                    if a.is_array() || b.is_array() { Ty::Vector } else { Ty::Scalar }
                }
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Div | BinaryOp::Pow | BinaryOp::Mod => {
                    if a.is_array() || b.is_array() { Ty::Vector } else { Ty::Scalar }
                }
                _ => Ty::Scalar,
            }
        }
        Expr::Call { name, args } => match name.as_str() {
            "linspace" | "range" | "arange" | "eigenvals" | "sort" | "reverse" | "polyfit" | "column" => Ty::Vector,
            "zeros" | "ones" | "identity" | "eye" | "diag" | "transpose" | "inv" | "odesolve" | "table_matrix" => Ty::Matrix,
            "read_csv" | "read_excel" | "dataframe" => Ty::Table,
            "diff" | "integrate" | "limit" | "simplify" | "expand" | "factor" | "solve_eq" | "dsolve" | "series" => {
                if name == "integrate" && args.len() >= 3 { Ty::Scalar } else { Ty::Symbolic }
            }
            _ => {
                if args.iter().any(|a| expr_type(a, types).is_array()) { Ty::Vector } else { Ty::Scalar }
            }
        },
    }
}

pub fn generate(stmts: &[Statement], opts: &Options) -> GeneratedScript {
    let types = infer_types(stmts);
    let mut em = emitter::Emitter::new(opts.clone(), types);
    let mut body: Vec<String> = Vec::new();
    let mut definitions = Vec::new();

    for stmt in stmts {
        if opts.comments {
            body.push(format!("# {}", aaiyyan_ast::mathstr::statement_to_string(stmt)));
        }
        match stmt {
            Statement::Assign { name, value } => {
                let code = em.expr(value);
                let py_name = em.name(name);
                body.push(format!("{py_name} = {code}"));
                if opts.capture_results {
                    body.push(format!("aaiyyan.report(\"{name}\", {py_name})"));
                }
                definitions.push(py_name);
            }
            Statement::FuncDef { name, params, body: fbody } => {
                let py_name = em.name(name);
                let ps: Vec<String> = params.iter().map(|p| em.name(p)).collect();
                let code = em.expr(fbody);
                body.push(format!("def {py_name}({}):", ps.join(", ")));
                body.push(format!("    return {code}"));
                body.push(String::new());
                if opts.capture_results {
                    body.push(format!("aaiyyan.report(\"{name}\", {py_name})"));
                }
                definitions.push(py_name);
            }
            Statement::Expr(e) => {
                let code = em.expr(e);
                if opts.capture_results {
                    body.push(format!("aaiyyan.report_expr({code})"));
                } else {
                    body.push(format!("print({code})"));
                }
            }
        }
        body.push(String::new());
    }

    let body_text = body.join("\n");
    let mut libs: BTreeSet<String> = em.libraries.into_iter().collect();
    if body_text.contains("np.") { libs.insert("numpy".into()); }
    if body_text.contains("sp.") { libs.insert("sympy".into()); }
    if body_text.contains("scipy.") { libs.insert("scipy".into()); }
    if body_text.contains("pd.") { libs.insert("pandas".into()); }
    if opts.use_units && body_text.contains("ureg.") { libs.insert("pint".into()); }
    if body_text.contains("math.") { libs.insert("math".into()); }
    if body_text.contains("aaiyyan.") { libs.insert("aaiyyan_runtime".into()); }

    let mut header: Vec<String> = Vec::new();
    header.push("# Generated by AaiyyanMath (native Rust code generator).".to_string());
    header.push("# The worksheet AST is authoritative; this file is a derived artefact.".to_string());
    header.push(String::new());
    if libs.contains("math") { header.push("import math".into()); }
    if libs.contains("numpy") { header.push("import numpy as np".into()); }
    if libs.contains("scipy") {
        header.push("import scipy".into());
        header.push("import scipy.integrate".into());
        header.push("import scipy.optimize".into());
    }
    if libs.contains("sympy") { header.push("import sympy as sp".into()); }
    if libs.contains("pandas") { header.push("import pandas as pd".into()); }
    if libs.contains("pint") {
        header.push("import pint".into());
        header.push("ureg = pint.UnitRegistry()".into());
    }
    if libs.contains("aaiyyan_runtime") { header.push("import aaiyyan_runtime as aaiyyan".into()); }
    header.push(String::new());

    let symbols = em.free_symbols;
    if libs.contains("sympy") && !symbols.is_empty() {
        let list: Vec<String> = symbols.iter().cloned().collect();
        header.push(format!("{} = sp.symbols(\"{}\")", list.join(", "), list.join(" ")));
        header.push(String::new());
    }

    let mut code = header.join("\n");
    code.push_str(&body_text);
    if opts.standalone {
        code.push_str("\n\nif __name__ == \"__main__\":\n");
        if definitions.is_empty() {
            code.push_str("    pass\n");
        }
        for d in &definitions {
            code.push_str(&format!("    print(f\"{d} = {{{d}}}\")\n"));
        }
    }

    GeneratedScript { code, libraries: libs.into_iter().collect(), definitions }
}

pub fn generate_expression(expr: &Expr, opts: &Options) -> String {
    let mut em = emitter::Emitter::new(opts.clone(), BTreeMap::new());
    em.expr(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar_program() -> Vec<Statement> {
        vec![
            Statement::Assign { name: "a".into(), value: Expr::num(5.0) },
            Statement::Assign { name: "b".into(), value: Expr::num(10.0) },
            Statement::Assign {
                name: "c".into(),
                value: Expr::call("sqrt", vec![Expr::bin(BinaryOp::Add,
                    Expr::bin(BinaryOp::Pow, Expr::sym("a"), Expr::num(2.0)),
                    Expr::bin(BinaryOp::Pow, Expr::sym("b"), Expr::num(2.0)))]),
            },
        ]
    }

    #[test]
    fn scalar_example_uses_math() {
        let script = generate(&scalar_program(), &Options {
            capture_results: false, use_units: false, comments: false, standalone: false,
        });
        assert!(script.code.contains("import math"));
        assert!(script.code.contains("c = math.sqrt(a**2 + b**2)"), "{}", script.code);
    }

    #[test]
    fn matrix_product_uses_matmul_operator() {
        let stmts = vec![
            Statement::Assign {
                name: "A".into(),
                value: Expr::Matrix(vec![vec![Expr::num(1.0), Expr::num(2.0)], vec![Expr::num(3.0), Expr::num(4.0)]]),
            },
            Statement::Assign {
                name: "B".into(),
                value: Expr::Matrix(vec![vec![Expr::num(5.0), Expr::num(6.0)], vec![Expr::num(7.0), Expr::num(8.0)]]),
            },
            Statement::Assign {
                name: "C".into(),
                value: Expr::bin(BinaryOp::Mul, Expr::sym("A"), Expr::sym("B")),
            },
        ];
        let script = generate(&stmts, &Options {
            capture_results: false, use_units: false, comments: false, standalone: false,
        });
        assert!(script.code.contains("C = A @ B"), "{}", script.code);
    }

    #[test]
    fn units_emit_pint() {
        let stmts = vec![Statement::Assign {
            name: "E".into(),
            value: Expr::bin(BinaryOp::Mul, Expr::num(210.0), Expr::sym("GPa")),
        }];
        let script = generate(&stmts, &Options::default());
        assert!(script.code.contains("ureg.GPa"), "{}", script.code);
        assert!(script.code.contains("import pint"));
    }
}
