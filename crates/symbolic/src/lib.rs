//! Symbolic mathematics bridge (spec section 12). SymPy is the backend; the
//! worksheet AST remains authoritative and results carry LaTeX for rendering.
use aaiyyan_ast::Statement;
use aaiyyan_codegen::{generate, Options};
use aaiyyan_python_runtime::{ExecOutcome, ValueDto};

pub fn needs_python(stmt: &Statement) -> bool { aaiyyan_ast::requires_symbolic(stmt) }

pub fn needs_python_any(stmts: &[Statement]) -> bool { stmts.iter().any(needs_python) }

pub fn symbolic_script(stmts: &[Statement]) -> String {
    generate(stmts, &Options { use_units: true, capture_results: true, comments: true, standalone: false }).code
}

#[derive(Clone, Debug)]
pub struct SymbolicResult {
    pub name: String,
    pub expr: String,
    pub latex: String,
}

pub fn collect(outcome: &ExecOutcome) -> Vec<SymbolicResult> {
    outcome.values.iter()
        .filter(|v| v.kind == "symbolic")
        .map(|v: &ValueDto| SymbolicResult {
            name: v.name.clone(),
            expr: v.text.clone().unwrap_or_else(|| v.display.clone()),
            latex: v.latex.clone().unwrap_or_default(),
        })
        .collect()
}

/// Convenience script for a single symbolic operation on worksheet expressions.
pub fn operation_script(op: &str, args: &[String], result_name: &str) -> String {
    format!(
        "import sympy as sp\nimport aaiyyan_runtime as aaiyyan\n{result_name} = sp.{op}({args})\naaiyyan.report(\"{result_name}\", {result_name})\n",
        op = op, args = args.join(", "), result_name = result_name
    )
}
