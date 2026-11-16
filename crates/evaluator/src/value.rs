use aaiyyan_ast::Expr;
use aaiyyan_units::{format_number, Dimension, Quantity, Unit};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize, data: Vec<f64>) -> Matrix { Matrix { rows, cols, data } }

    pub fn identity(n: usize) -> Matrix {
        let mut m = Matrix::new(n, n, vec![0.0; n * n]);
        for i in 0..n { m.data[i * n + i] = 1.0; }
        m
    }

    pub fn get(&self, r: usize, c: usize) -> f64 { self.data[r * self.cols + c] }
    pub fn shape_matches(&self, other: &Matrix) -> bool { self.rows == other.rows && self.cols == other.cols }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FunctionBody {
    Ast(Box<Expr>),
    Builtin(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: FunctionBody,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Cell {
    Num(f64),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Scalar(f64),
    Quantity(Quantity),
    Complex { re: f64, im: f64 },
    Bool(bool),
    Str(String),
    Vector(Vec<f64>),
    Matrix(Matrix),
    Unit(Unit),
    Function(Function),
    Symbolic { expr: String, latex: String },
    Table(Table),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Scalar(_) => "scalar",
            Value::Quantity(_) => "quantity",
            Value::Complex { .. } => "complex",
            Value::Bool(_) => "boolean",
            Value::Str(_) => "text",
            Value::Vector(_) => "vector",
            Value::Matrix(_) => "matrix",
            Value::Unit(_) => "unit",
            Value::Function(_) => "function",
            Value::Symbolic { .. } => "symbolic",
            Value::Table(_) => "table",
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Scalar(v) => Some(*v),
            Value::Quantity(q) if q.is_dimensionless() => Some(q.si),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_quantity(&self) -> Option<Quantity> {
        match self {
            Value::Scalar(v) => Some(Quantity::dimensionless(*v)),
            Value::Quantity(q) => Some(q.clone()),
            _ => None,
        }
    }

    pub fn dimension(&self) -> Option<Dimension> {
        match self {
            Value::Scalar(_) => Some(Dimension::DIMENSIONLESS),
            Value::Quantity(q) => Some(q.dim()),
            Value::Unit(u) => Some(u.dim),
            _ => None,
        }
    }

    pub fn display(&self) -> String {
        match self {
            Value::Scalar(v) => format_number(*v),
            Value::Quantity(q) => q.display(),
            Value::Complex { re, im } => {
                let sign = if *im < 0.0 { "-" } else { "+" };
                format!("{} {} {}i", format_number(*re), sign, format_number(im.abs()))
            }
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.clone(),
            Value::Vector(v) => {
                let items: Vec<String> = v.iter().take(12).map(|x| format_number(*x)).collect();
                let more = if v.len() > 12 { ", ..." } else { "" };
                format!("[{}{more}]", items.join(", "))
            }
            Value::Matrix(m) => format!("{}x{} matrix", m.rows, m.cols),
            Value::Unit(u) => format!("unit {}", u.name),
            Value::Function(f) => format!("{}({})", f.name, f.params.join(", ")),
            Value::Symbolic { expr, .. } => expr.clone(),
            Value::Table(t) => format!("table {} rows x {} columns", t.rows.len(), t.columns.len()),
        }
    }

    pub fn latex(&self) -> String {
        match self {
            Value::Scalar(v) => format_number(*v),
            Value::Quantity(q) => {
                if q.unit.name.is_empty() {
                    format_number(q.value())
                } else {
                    format!("{}\\,\\mathrm{{{}}}", format_number(q.value()), q.unit.name.replace('*', " \\cdot "))
                }
            }
            Value::Matrix(m) => {
                let rows: Vec<String> = (0..m.rows)
                    .map(|r| (0..m.cols).map(|c| format_number(m.get(r, c))).collect::<Vec<_>>().join(" & "))
                    .collect();
                format!("\\begin{{bmatrix}}{}\\end{{bmatrix}}", rows.join(" \\\\ "))
            }
            Value::Vector(v) => {
                let items: Vec<String> = v.iter().map(|x| format_number(*x)).collect();
                format!("\\begin{{bmatrix}}{}\\end{{bmatrix}}", items.join(" \\\\ "))
            }
            Value::Symbolic { latex, expr } => {
                if latex.is_empty() { expr.clone() } else { latex.clone() }
            }
            other => other.display(),
        }
    }
}
