use crate::builtins;
use crate::env::Environment;
use crate::value::{Function, FunctionBody, Matrix, Value};
use crate::EvalError;
use aaiyyan_ast::{is_symbolic_call, BinaryOp, Expr, Statement, UnaryOp};
use aaiyyan_units::{lookup, Dimension, Quantity, Unit};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct EvalLimits {
    pub max_steps: u64,
    pub timeout: Option<Duration>,
    pub cancel: Option<Arc<AtomicBool>>,
}

impl Default for EvalLimits {
    fn default() -> Self {
        EvalLimits { max_steps: 20_000_000, timeout: Some(Duration::from_secs(15)), cancel: None }
    }
}

pub struct Evaluator {
    pub limits: EvalLimits,
    pub output: Vec<String>,
    steps: u64,
    started: Instant,
    depth: usize,
}

impl Evaluator {
    pub fn new(limits: EvalLimits) -> Evaluator {
        Evaluator { limits, output: Vec::new(), steps: 0, started: Instant::now(), depth: 0 }
    }

    fn tick(&mut self) -> Result<(), EvalError> {
        self.steps += 1;
        if self.steps > self.limits.max_steps { return Err(EvalError::StepLimit); }
        if self.steps % 512 == 0 {
            if let Some(c) = &self.limits.cancel {
                if c.load(Ordering::Relaxed) { return Err(EvalError::Cancelled); }
            }
            if let Some(t) = self.limits.timeout {
                if self.started.elapsed() > t { return Err(EvalError::Timeout); }
            }
        }
        Ok(())
    }

    pub fn eval_statement(&mut self, stmt: &Statement, env: &mut Environment) -> Result<(String, Value), EvalError> {
        match stmt {
            Statement::Assign { name, value } => {
                let v = self.eval(value, env)?;
                env.define_global(name.clone(), v.clone());
                Ok((name.clone(), v))
            }
            Statement::FuncDef { name, params, body } => {
                let f = Value::Function(Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: FunctionBody::Ast(Box::new(body.clone())),
                });
                env.define_global(name.clone(), f.clone());
                Ok((name.clone(), f))
            }
            Statement::Expr(e) => Ok(("_".to_string(), self.eval(e, env)?)),
        }
    }

    pub fn call_value(&mut self, f: &Value, args: Vec<Value>, env: &mut Environment) -> Result<Value, EvalError> {
        match f {
            Value::Function(fun) => match &fun.body {
                FunctionBody::Builtin(name) => builtins::call(name, &args, self, env),
                FunctionBody::Ast(body) => {
                    if args.len() != fun.params.len() {
                        return Err(EvalError::Args(format!(
                            "{} expects {} argument(s), got {}", fun.name, fun.params.len(), args.len()
                        )));
                    }
                    if self.depth > 128 { return Err(EvalError::Math("recursion limit exceeded".into())); }
                    let mut scope = HashMap::new();
                    for (p, a) in fun.params.iter().zip(args.into_iter()) { scope.insert(p.clone(), a); }
                    env.push_scope(scope);
                    self.depth += 1;
                    let result = self.eval(body, env);
                    self.depth -= 1;
                    env.pop_scope();
                    result
                }
            },
            other => Err(EvalError::Type(format!("{} is not callable", other.type_name()))),
        }
    }

    pub fn eval(&mut self, expr: &Expr, env: &mut Environment) -> Result<Value, EvalError> {
        self.tick()?;
        match expr {
            Expr::Number(v) => Ok(Value::Scalar(*v)),
            Expr::Imaginary => Ok(Value::Complex { re: 0.0, im: 1.0 }),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Symbol(name) => {
                if let Some(v) = env.get(name) { return Ok(v.clone()); }
                if let Some(u) = lookup(name) { return Ok(Value::Unit(u)); }
                Err(EvalError::Undefined(name.clone()))
            }
            Expr::Unary { op, expr } => {
                let v = self.eval(expr, env)?;
                unary(*op, &v)
            }
            Expr::Binary { op, lhs, rhs } => {
                if *op == BinaryOp::And || *op == BinaryOp::Or {
                    let a = truthy(&self.eval(lhs, env)?)?;
                    if *op == BinaryOp::And && !a { return Ok(Value::Bool(false)); }
                    if *op == BinaryOp::Or && a { return Ok(Value::Bool(true)); }
                    let b = truthy(&self.eval(rhs, env)?)?;
                    return Ok(Value::Bool(b));
                }
                let a = self.eval(lhs, env)?;
                let b = self.eval(rhs, env)?;
                binary(*op, &a, &b)
            }
            Expr::Call { name, args } => {
                if is_symbolic_call(name, args.len()) {
                    return Err(EvalError::RequiresPython(format!("{name}()")));
                }
                if aaiyyan_ast::is_python_only_call(name) {
                    return Err(EvalError::RequiresPython(format!("{name}()")));
                }
                let mut vals = Vec::with_capacity(args.len());
                for a in args { vals.push(self.eval(a, env)?); }
                if let Some(f) = env.get(name).cloned() {
                    return self.call_value(&f, vals, env);
                }
                if builtins::is_builtin(name) {
                    return builtins::call(name, &vals, self, env);
                }
                Err(EvalError::UnknownFunction(name.clone()))
            }
            Expr::Matrix(rows) => {
                let nr = rows.len();
                let nc = rows.first().map(|r| r.len()).unwrap_or(0);
                let mut data = Vec::with_capacity(nr * nc);
                for row in rows {
                    for cell in row {
                        let v = self.eval(cell, env)?;
                        data.push(dimensionless(&v)?);
                    }
                }
                Ok(Value::Matrix(Matrix::new(nr, nc, data)))
            }
            Expr::Vector(items) => {
                let mut data = Vec::with_capacity(items.len());
                for item in items {
                    let v = self.eval(item, env)?;
                    data.push(dimensionless(&v)?);
                }
                Ok(Value::Vector(data))
            }
            Expr::Range { start, step, end } => {
                let a = as_quantity(&self.eval(start, env)?)?;
                let b = as_quantity(&self.eval(end, env)?)?;
                if !a.dim().compatible(&b.dim()) {
                    return Err(EvalError::Dimension("range bounds have different dimensions".into()));
                }
                let s = match step {
                    Some(st) => {
                        let q = as_quantity(&self.eval(st, env)?)?;
                        if !a.dim().compatible(&q.dim()) {
                            return Err(EvalError::Dimension("range step has a different dimension".into()));
                        }
                        q.value()
                    }
                    None => 1.0,
                };
                if s == 0.0 { return Err(EvalError::Args("range step cannot be zero".into())); }
                let (lo, hi) = (a.value(), b.value());
                let count = ((hi - lo) / s).floor() as i64 + 1;
                if count < 0 || count > 1_000_000 {
                    return Err(EvalError::Args("range produces too many elements".into()));
                }
                Ok(Value::Vector((0..count).map(|k| lo + (k as f64) * s).collect()))
            }
            Expr::Index { target, indices } => {
                let t = self.eval(target, env)?;
                let mut idx = Vec::new();
                for i in indices { idx.push(dimensionless(&self.eval(i, env)?)?); }
                index(&t, &idx)
            }
            Expr::Conditional { cond, then, otherwise } => {
                let c = truthy(&self.eval(cond, env)?)?;
                if c { self.eval(then, env) } else { self.eval(otherwise, env) }
            }
        }
    }
}

pub fn truthy(v: &Value) -> Result<bool, EvalError> {
    match v {
        Value::Bool(b) => Ok(*b),
        Value::Scalar(x) => Ok(*x != 0.0),
        Value::Quantity(q) => Ok(q.si != 0.0),
        other => Err(EvalError::Type(format!("{} is not a condition", other.type_name()))),
    }
}

pub fn dimensionless(v: &Value) -> Result<f64, EvalError> {
    match v {
        Value::Scalar(x) => Ok(*x),
        Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        Value::Quantity(q) if q.is_dimensionless() => Ok(q.si),
        Value::Quantity(q) => Err(EvalError::Dimension(format!(
            "expected a dimensionless value, found {}", q.dim().quantity_name()
        ))),
        other => Err(EvalError::Type(format!("expected a number, found {}", other.type_name()))),
    }
}

pub fn as_quantity(v: &Value) -> Result<Quantity, EvalError> {
    match v {
        Value::Scalar(x) => Ok(Quantity::dimensionless(*x)),
        Value::Bool(b) => Ok(Quantity::dimensionless(if *b { 1.0 } else { 0.0 })),
        Value::Quantity(q) => Ok(q.clone()),
        other => Err(EvalError::Type(format!("expected a scalar or quantity, found {}", other.type_name()))),
    }
}

fn as_complex(v: &Value) -> Option<(f64, f64)> {
    match v {
        Value::Complex { re, im } => Some((*re, *im)),
        Value::Scalar(x) => Some((*x, 0.0)),
        Value::Quantity(q) if q.is_dimensionless() => Some((q.si, 0.0)),
        _ => None,
    }
}

fn unary(op: UnaryOp, v: &Value) -> Result<Value, EvalError> {
    match op {
        UnaryOp::Pos => Ok(v.clone()),
        UnaryOp::Not => Ok(Value::Bool(!truthy(v)?)),
        UnaryOp::Neg => match v {
            Value::Scalar(x) => Ok(Value::Scalar(-x)),
            Value::Quantity(q) => Ok(Value::Quantity(Quantity { si: -q.si, unit: q.unit.clone() })),
            Value::Complex { re, im } => Ok(Value::Complex { re: -re, im: -im }),
            Value::Vector(a) => Ok(Value::Vector(a.iter().map(|x| -x).collect())),
            Value::Matrix(m) => Ok(Value::Matrix(Matrix::new(m.rows, m.cols, m.data.iter().map(|x| -x).collect()))),
            other => Err(EvalError::Type(format!("cannot negate {}", other.type_name()))),
        },
        UnaryOp::Factorial => {
            let x = dimensionless(v)?;
            if x < 0.0 || x.fract().abs() > 1e-9 || x > 170.0 {
                return Err(EvalError::Args("factorial requires an integer in 0..170".into()));
            }
            let n = x.round() as u64;
            let mut acc = 1.0f64;
            for k in 2..=n { acc *= k as f64; }
            Ok(Value::Scalar(acc))
        }
        UnaryOp::Transpose => match v {
            Value::Matrix(m) => Ok(Value::Matrix(crate::linalg::transpose(m))),
            Value::Vector(a) => Ok(Value::Matrix(Matrix::new(1, a.len(), a.clone()))),
            other => Err(EvalError::Type(format!("cannot transpose {}", other.type_name()))),
        },
    }
}

fn index(target: &Value, idx: &[f64]) -> Result<Value, EvalError> {
    let to_usize = |x: f64| -> Result<usize, EvalError> {
        if x < 0.0 || x.fract().abs() > 1e-9 {
            return Err(EvalError::Index("indices must be non-negative integers (0-based)".into()));
        }
        Ok(x.round() as usize)
    };
    match target {
        Value::Vector(v) => {
            if idx.len() != 1 { return Err(EvalError::Index("a vector takes one index".into())); }
            let i = to_usize(idx[0])?;
            v.get(i).copied().map(Value::Scalar)
                .ok_or_else(|| EvalError::Index(format!("index {i} out of range (len {})", v.len())))
        }
        Value::Matrix(m) => {
            if idx.len() == 1 {
                let r = to_usize(idx[0])?;
                if r >= m.rows { return Err(EvalError::Index(format!("row {r} out of range"))); }
                return Ok(Value::Vector((0..m.cols).map(|c| m.get(r, c)).collect()));
            }
            if idx.len() != 2 { return Err(EvalError::Index("a matrix takes one or two indices".into())); }
            let r = to_usize(idx[0])?;
            let c = to_usize(idx[1])?;
            if r >= m.rows || c >= m.cols {
                return Err(EvalError::Index(format!("index ({r},{c}) out of range for {}x{}", m.rows, m.cols)));
            }
            Ok(Value::Scalar(m.get(r, c)))
        }
        Value::Str(s) => {
            let i = to_usize(idx[0])?;
            s.chars().nth(i).map(|c| Value::Str(c.to_string()))
                .ok_or_else(|| EvalError::Index("string index out of range".into()))
        }
        other => Err(EvalError::Type(format!("cannot index {}", other.type_name()))),
    }
}

pub fn binary(op: BinaryOp, a: &Value, b: &Value) -> Result<Value, EvalError> {
    match op {
        BinaryOp::Add => add(a, b),
        BinaryOp::Sub => sub(a, b),
        BinaryOp::Mul => mul(a, b),
        BinaryOp::Div => div(a, b),
        BinaryOp::Pow => pow(a, b),
        BinaryOp::Mod => {
            let x = as_quantity(a)?;
            let y = as_quantity(b)?;
            if !x.dim().compatible(&y.dim()) {
                return Err(EvalError::Dimension("modulo operands have different dimensions".into()));
            }
            if y.si == 0.0 { return Err(EvalError::Math("modulo by zero".into())); }
            Ok(Value::Quantity(Quantity { si: x.si % y.si, unit: x.unit.clone() }))
        }
        BinaryOp::ElemMul => elementwise(a, b, BinaryOp::Mul),
        BinaryOp::ElemDiv => elementwise(a, b, BinaryOp::Div),
        BinaryOp::ElemPow => elementwise(a, b, BinaryOp::Pow),
        BinaryOp::And => Ok(Value::Bool(truthy(a)? && truthy(b)?)),
        BinaryOp::Or => Ok(Value::Bool(truthy(a)? || truthy(b)?)),
        BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => compare(op, a, b),
    }
}

fn add(a: &Value, b: &Value) -> Result<Value, EvalError> {
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => Ok(Value::Str(format!("{x}{y}"))),
        (Value::Vector(x), Value::Vector(y)) => {
            if x.len() != y.len() { return Err(EvalError::Args("vector lengths differ".into())); }
            Ok(Value::Vector(x.iter().zip(y).map(|(p, q)| p + q).collect()))
        }
        (Value::Matrix(x), Value::Matrix(y)) => {
            if !x.shape_matches(y) { return Err(EvalError::Args("matrix shapes differ".into())); }
            Ok(Value::Matrix(Matrix::new(x.rows, x.cols, x.data.iter().zip(&y.data).map(|(p, q)| p + q).collect())))
        }
        (Value::Matrix(m), Value::Scalar(s)) | (Value::Scalar(s), Value::Matrix(m)) => {
            Ok(Value::Matrix(Matrix::new(m.rows, m.cols, m.data.iter().map(|v| v + s).collect())))
        }
        (Value::Vector(v), Value::Scalar(s)) | (Value::Scalar(s), Value::Vector(v)) => {
            Ok(Value::Vector(v.iter().map(|x| x + s).collect()))
        }
        _ => {
            if is_complex(a) || is_complex(b) {
                let (ar, ai) = as_complex(a).ok_or_else(|| EvalError::Type("complex addition".into()))?;
                let (br, bi) = as_complex(b).ok_or_else(|| EvalError::Type("complex addition".into()))?;
                return Ok(Value::Complex { re: ar + br, im: ai + bi });
            }
            let qa = as_quantity(a)?;
            let qb = as_quantity(b)?;
            let mut r = qa.add(&qb)?;
            if matches!(a, Value::Scalar(_)) { r.unit = qb.unit.clone(); }
            Ok(Value::Quantity(r))
        }
    }
}

fn sub(a: &Value, b: &Value) -> Result<Value, EvalError> {
    match (a, b) {
        (Value::Vector(x), Value::Vector(y)) => {
            if x.len() != y.len() { return Err(EvalError::Args("vector lengths differ".into())); }
            Ok(Value::Vector(x.iter().zip(y).map(|(p, q)| p - q).collect()))
        }
        (Value::Matrix(x), Value::Matrix(y)) => {
            if !x.shape_matches(y) { return Err(EvalError::Args("matrix shapes differ".into())); }
            Ok(Value::Matrix(Matrix::new(x.rows, x.cols, x.data.iter().zip(&y.data).map(|(p, q)| p - q).collect())))
        }
        (Value::Matrix(m), Value::Scalar(s)) => {
            Ok(Value::Matrix(Matrix::new(m.rows, m.cols, m.data.iter().map(|v| v - s).collect())))
        }
        (Value::Scalar(s), Value::Matrix(m)) => {
            Ok(Value::Matrix(Matrix::new(m.rows, m.cols, m.data.iter().map(|v| s - v).collect())))
        }
        (Value::Vector(v), Value::Scalar(s)) => Ok(Value::Vector(v.iter().map(|x| x - s).collect())),
        (Value::Scalar(s), Value::Vector(v)) => Ok(Value::Vector(v.iter().map(|x| s - x).collect())),
        _ => {
            if is_complex(a) || is_complex(b) {
                let (ar, ai) = as_complex(a).ok_or_else(|| EvalError::Type("complex subtraction".into()))?;
                let (br, bi) = as_complex(b).ok_or_else(|| EvalError::Type("complex subtraction".into()))?;
                return Ok(Value::Complex { re: ar - br, im: ai - bi });
            }
            let qa = as_quantity(a)?;
            let qb = as_quantity(b)?;
            let mut r = qa.sub(&qb)?;
            if matches!(a, Value::Scalar(_)) { r.unit = qb.unit.clone(); }
            Ok(Value::Quantity(r))
        }
    }
}

fn mul(a: &Value, b: &Value) -> Result<Value, EvalError> {
    match (a, b) {
        (Value::Unit(x), Value::Unit(y)) => Ok(Value::Unit(x.mul_unit(y))),
        (Value::Scalar(x), Value::Unit(u)) | (Value::Unit(u), Value::Scalar(x)) => {
            Ok(Value::Quantity(Quantity::new(*x, u.clone())))
        }
        (Value::Quantity(q), Value::Unit(u)) => Ok(Value::Quantity(q.mul_unit(u)?)),
        (Value::Unit(u), Value::Quantity(q)) => Ok(Value::Quantity(q.mul_unit(u)?)),
        (Value::Matrix(x), Value::Matrix(y)) => Ok(Value::Matrix(crate::linalg::matmul(x, y)?)),
        (Value::Matrix(m), Value::Vector(v)) => Ok(Value::Vector(crate::linalg::matvec(m, v)?)),
        (Value::Vector(v), Value::Matrix(m)) => {
            let row = Matrix::new(1, v.len(), v.clone());
            let r = crate::linalg::matmul(&row, m)?;
            Ok(Value::Vector(r.data))
        }
        (Value::Vector(x), Value::Vector(y)) => {
            if x.len() != y.len() { return Err(EvalError::Args("dot product needs equal lengths".into())); }
            Ok(Value::Scalar(x.iter().zip(y).map(|(p, q)| p * q).sum()))
        }
        (Value::Matrix(m), Value::Scalar(s)) | (Value::Scalar(s), Value::Matrix(m)) => {
            Ok(Value::Matrix(Matrix::new(m.rows, m.cols, m.data.iter().map(|v| v * s).collect())))
        }
        (Value::Vector(v), Value::Scalar(s)) | (Value::Scalar(s), Value::Vector(v)) => {
            Ok(Value::Vector(v.iter().map(|x| x * s).collect()))
        }
        _ => {
            if is_complex(a) || is_complex(b) {
                let (ar, ai) = as_complex(a).ok_or_else(|| EvalError::Type("complex multiplication".into()))?;
                let (br, bi) = as_complex(b).ok_or_else(|| EvalError::Type("complex multiplication".into()))?;
                return Ok(Value::Complex { re: ar * br - ai * bi, im: ar * bi + ai * br });
            }
            let qa = as_quantity(a)?;
            let qb = as_quantity(b)?;
            if matches!(a, Value::Scalar(_)) { return Ok(Value::Quantity(qb.scale(qa.si)?)); }
            if matches!(b, Value::Scalar(_)) { return Ok(Value::Quantity(qa.scale(qb.si)?)); }
            Ok(Value::Quantity(qa.mul(&qb)?))
        }
    }
}

fn div(a: &Value, b: &Value) -> Result<Value, EvalError> {
    match (a, b) {
        (Value::Unit(x), Value::Unit(y)) => Ok(Value::Unit(x.div_unit(y))),
        (Value::Quantity(q), Value::Unit(u)) => Ok(Value::Quantity(q.div_unit(u)?)),
        (Value::Scalar(x), Value::Unit(u)) => {
            let q = Quantity::dimensionless(*x);
            Ok(Value::Quantity(q.div_unit(u)?))
        }
        (Value::Unit(u), Value::Scalar(x)) => {
            if *x == 0.0 { return Err(EvalError::Math("division by zero".into())); }
            Ok(Value::Unit(Unit {
                name: format!("1/{}", u.name),
                dim: Dimension::DIMENSIONLESS / u.dim,
                factor: 1.0 / u.factor,
                offset: 0.0,
            }))
        }
        (Value::Matrix(m), Value::Scalar(s)) => {
            if *s == 0.0 { return Err(EvalError::Math("division by zero".into())); }
            Ok(Value::Matrix(Matrix::new(m.rows, m.cols, m.data.iter().map(|v| v / s).collect())))
        }
        (Value::Vector(v), Value::Scalar(s)) => {
            if *s == 0.0 { return Err(EvalError::Math("division by zero".into())); }
            Ok(Value::Vector(v.iter().map(|x| x / s).collect()))
        }
        _ => {
            if is_complex(a) || is_complex(b) {
                let (ar, ai) = as_complex(a).ok_or_else(|| EvalError::Type("complex division".into()))?;
                let (br, bi) = as_complex(b).ok_or_else(|| EvalError::Type("complex division".into()))?;
                let d = br * br + bi * bi;
                if d == 0.0 { return Err(EvalError::Math("complex division by zero".into())); }
                return Ok(Value::Complex { re: (ar * br + ai * bi) / d, im: (ai * br - ar * bi) / d });
            }
            let qa = as_quantity(a)?;
            let qb = as_quantity(b)?;
            if qb.si == 0.0 { return Err(EvalError::Math("division by zero".into())); }
            if matches!(b, Value::Scalar(_)) { return Ok(Value::Quantity(qa.scale(1.0 / qb.si)?)); }
            Ok(Value::Quantity(qa.div(&qb)?))
        }
    }
}

fn pow(a: &Value, b: &Value) -> Result<Value, EvalError> {
    match (a, b) {
        (Value::Unit(u), Value::Scalar(e)) => Ok(Value::Unit(u.pow_unit(*e))),
        (Value::Quantity(q), Value::Scalar(e)) => Ok(Value::Quantity(q.powf(*e)?)),
        (Value::Matrix(m), Value::Scalar(e)) => {
            if e.fract().abs() > 1e-9 || e.abs() > 64.0 {
                return Err(EvalError::Args("matrix power requires a small integer exponent".into()));
            }
            let n = e.abs().round() as i32;
            let base = if *e < 0.0 { crate::linalg::inv(m)? } else { m.clone() };
            let mut acc = Matrix::identity(m.rows);
            for _ in 0..n { acc = crate::linalg::matmul(&acc, &base)?; }
            Ok(Value::Matrix(acc))
        }
        (Value::Vector(v), Value::Scalar(e)) => Ok(Value::Vector(v.iter().map(|x| x.powf(*e)).collect())),
        (Value::Complex { re, im }, Value::Scalar(e)) => {
            let r = (re * re + im * im).sqrt();
            let theta = im.atan2(*re);
            let mag = r.powf(*e);
            let ang = theta * e;
            Ok(Value::Complex { re: mag * ang.cos(), im: mag * ang.sin() })
        }
        _ => {
            let x = as_quantity(a)?;
            let y = as_quantity(b)?;
            if !y.is_dimensionless() {
                return Err(EvalError::Dimension("exponent must be dimensionless".into()));
            }
            if !x.is_dimensionless() {
                return Ok(Value::Quantity(Quantity::from_si(x.si, x.dim()).powf(y.si)?));
            }
            if x.si < 0.0 && y.si.fract().abs() > 1e-12 {
                let r = (-x.si).powf(y.si);
                let ang = std::f64::consts::PI * y.si;
                return Ok(Value::Complex { re: r * ang.cos(), im: r * ang.sin() });
            }
            Ok(Value::Scalar(x.si.powf(y.si)))
        }
    }
}

fn elementwise(a: &Value, b: &Value, op: BinaryOp) -> Result<Value, EvalError> {
    match (a, b) {
        (Value::Vector(x), Value::Vector(y)) => {
            if x.len() != y.len() { return Err(EvalError::Args("vector lengths differ".into())); }
            let mut out = Vec::with_capacity(x.len());
            for (p, q) in x.iter().zip(y) {
                out.push(dimensionless(&binary(op, &Value::Scalar(*p), &Value::Scalar(*q))?)?);
            }
            Ok(Value::Vector(out))
        }
        (Value::Matrix(x), Value::Matrix(y)) => {
            if !x.shape_matches(y) { return Err(EvalError::Args("matrix shapes differ".into())); }
            let mut out = Vec::with_capacity(x.data.len());
            for (p, q) in x.data.iter().zip(&y.data) {
                out.push(dimensionless(&binary(op, &Value::Scalar(*p), &Value::Scalar(*q))?)?);
            }
            Ok(Value::Matrix(Matrix::new(x.rows, x.cols, out)))
        }
        (Value::Vector(v), Value::Scalar(s)) => {
            let mut out = Vec::with_capacity(v.len());
            for p in v { out.push(dimensionless(&binary(op, &Value::Scalar(*p), &Value::Scalar(*s))?)?); }
            Ok(Value::Vector(out))
        }
        (Value::Scalar(s), Value::Vector(v)) => {
            let mut out = Vec::with_capacity(v.len());
            for p in v { out.push(dimensionless(&binary(op, &Value::Scalar(*s), &Value::Scalar(*p))?)?); }
            Ok(Value::Vector(out))
        }
        (Value::Matrix(m), Value::Scalar(s)) => {
            let mut out = Vec::with_capacity(m.data.len());
            for p in &m.data { out.push(dimensionless(&binary(op, &Value::Scalar(*p), &Value::Scalar(*s))?)?); }
            Ok(Value::Matrix(Matrix::new(m.rows, m.cols, out)))
        }
        _ => binary(op, a, b),
    }
}

fn is_complex(v: &Value) -> bool { matches!(v, Value::Complex { .. }) }

fn compare(op: BinaryOp, a: &Value, b: &Value) -> Result<Value, EvalError> {
    let result = match (a, b) {
        (Value::Str(x), Value::Str(y)) => match op {
            BinaryOp::Eq => x == y,
            BinaryOp::Ne => x != y,
            BinaryOp::Lt => x < y,
            BinaryOp::Le => x <= y,
            BinaryOp::Gt => x > y,
            BinaryOp::Ge => x >= y,
            _ => false,
        },
        (Value::Bool(x), Value::Bool(y)) => match op {
            BinaryOp::Eq => x == y,
            BinaryOp::Ne => x != y,
            _ => return Err(EvalError::Type("booleans support only equality".into())),
        },
        _ => {
            if is_complex(a) || is_complex(b) {
                let (ar, ai) = as_complex(a).ok_or_else(|| EvalError::Type("complex comparison".into()))?;
                let (br, bi) = as_complex(b).ok_or_else(|| EvalError::Type("complex comparison".into()))?;
                let eq = (ar - br).abs() < 1e-12 && (ai - bi).abs() < 1e-12;
                match op {
                    BinaryOp::Eq => eq,
                    BinaryOp::Ne => !eq,
                    _ => return Err(EvalError::Type("complex numbers are not ordered".into())),
                }
            } else {
                let qa = as_quantity(a)?;
                let qb = as_quantity(b)?;
                if !qa.dim().compatible(&qb.dim()) {
                    return Err(EvalError::Dimension(format!(
                        "cannot compare {} with {}", qa.dim().quantity_name(), qb.dim().quantity_name()
                    )));
                }
                match op {
                    BinaryOp::Eq => (qa.si - qb.si).abs() < 1e-12,
                    BinaryOp::Ne => (qa.si - qb.si).abs() >= 1e-12,
                    BinaryOp::Lt => qa.si < qb.si,
                    BinaryOp::Le => qa.si <= qb.si,
                    BinaryOp::Gt => qa.si > qb.si,
                    BinaryOp::Ge => qa.si >= qb.si,
                    _ => false,
                }
            }
        }
    };
    Ok(Value::Bool(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aaiyyan_parser::parse_program;

    fn run(src: &str) -> Environment {
        let stmts = parse_program(src).unwrap();
        let mut env = Environment::new();
        let mut ev = Evaluator::new(EvalLimits::default());
        for s in &stmts {
            ev.eval_statement(s, &mut env).unwrap_or_else(|e| panic!("{e} in {src}"));
        }
        env
    }

    #[test]
    fn mvp_beam_deflection() {
        let env = run("E := 210 GPa\nI := 8.5e-6 m^4\nL := 3 m\nw := 10 kN/m\ndelta := 5*w*L^4/(384*E*I)");
        let q = env.get("delta").unwrap().as_quantity().unwrap();
        assert!((q.si - 0.0059086).abs() < 1e-6, "si = {}", q.si);
        assert!(q.dim().compatible(&Dimension::length()));
        assert_eq!(q.unit.name, "m");
    }

    #[test]
    fn matrix_multiply_and_elementwise() {
        let env = run("A := [[1,2],[3,4]]\nB := [[5,6],[7,8]]\nC := A*B\nD := A.*B");
        match env.get("C").unwrap() {
            Value::Matrix(m) => assert_eq!(m.data, vec![19.0, 22.0, 43.0, 50.0]),
            _ => panic!("expected matrix"),
        }
        match env.get("D").unwrap() {
            Value::Matrix(m) => assert_eq!(m.data, vec![5.0, 12.0, 21.0, 32.0]),
            _ => panic!("expected matrix"),
        }
    }

    #[test]
    fn functions_and_conditionals() {
        let env = run("f(x) := x^2 + 1\ny := f(3)\nz := if y > 5 then 1 else 0");
        assert_eq!(env.get("y").unwrap().as_f64(), Some(10.0));
        assert_eq!(env.get("z").unwrap().as_f64(), Some(1.0));
    }

    #[test]
    fn dimensional_error_is_reported() {
        let stmts = parse_program("a := 5 m\nb := 2 s\nc := a + b").unwrap();
        let mut env = Environment::new();
        let mut ev = Evaluator::new(EvalLimits::default());
        let mut err = None;
        for s in &stmts {
            if let Err(e) = ev.eval_statement(s, &mut env) { err = Some(e); }
        }
        assert!(err.is_some());
    }

    #[test]
    fn unit_conversion_builtin() {
        let env = run("d := 0.0059086 m\ndmm := convert(d, mm)");
        let q = env.get("dmm").unwrap().as_quantity().unwrap();
        assert!((q.value() - 5.9086).abs() < 1e-6);
    }

    #[test]
    fn ode_solver_runs() {
        let env = run("f(t, y) := -2*y\nS := odesolve(f, 0, 1, 1, 100)");
        match env.get("S").unwrap() {
            Value::Matrix(m) => {
                assert_eq!(m.rows, 101);
                let last = m.get(100, 1);
                assert!((last - (-2.0f64).exp()).abs() < 1e-4);
            }
            _ => panic!("expected matrix"),
        }
    }
}
