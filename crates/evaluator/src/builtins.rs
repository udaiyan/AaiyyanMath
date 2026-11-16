//! Native numerical builtins. Symbolic operations are intentionally routed to
//! the Python/SymPy backend instead of being reimplemented here.
use crate::env::Environment;
use crate::interp::{as_quantity, dimensionless, truthy, Evaluator};
use crate::linalg;
use crate::value::{Cell, Matrix, Table, Value};
use crate::EvalError;
use aaiyyan_units::{lookup, Quantity};

pub const NAMES: &[&str] = &[
    "sqrt", "cbrt", "abs", "exp", "ln", "log", "log10", "log2", "sin", "cos", "tan", "asin",
    "acos", "atan", "atan2", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "floor", "ceil",
    "round", "trunc", "sign", "mod", "hypot", "min", "max", "sum", "prod", "mean", "std", "var",
    "median", "len", "rows", "cols", "linspace", "range", "arange", "zeros", "ones", "identity",
    "eye", "diag", "transpose", "det", "inv", "rank", "trace", "norm", "dot", "cross", "solve",
    "eigenvals", "eigenvectors", "sort", "reverse", "polyfit", "polyval", "interp1", "integrate",
    "nderiv", "root", "minimize", "odesolve", "convert", "si", "strip_units", "unit_of", "if",
    "slider", "input_number", "checkbox", "dropdown", "read_csv", "column", "table_matrix",
    "print", "Re", "Im", "conj", "arg", "to_deg", "to_rad",
];

pub fn is_builtin(name: &str) -> bool {
    NAMES.contains(&name) || crate::builtins_ext::NAMES.contains(&name)
}

fn arity(name: &str, args: &[Value], n: usize) -> Result<(), EvalError> {
    if args.len() != n {
        return Err(EvalError::Args(format!("{name} expects {n} argument(s), got {}", args.len())));
    }
    Ok(())
}

fn vector(v: &Value) -> Result<Vec<f64>, EvalError> {
    match v {
        Value::Vector(a) => Ok(a.clone()),
        Value::Matrix(m) if m.cols == 1 || m.rows == 1 => Ok(m.data.clone()),
        Value::Scalar(x) => Ok(vec![*x]),
        other => Err(EvalError::Type(format!("expected a vector, found {}", other.type_name()))),
    }
}

fn matrix(v: &Value) -> Result<Matrix, EvalError> {
    match v {
        Value::Matrix(m) => Ok(m.clone()),
        Value::Vector(a) => Ok(Matrix::new(a.len(), 1, a.clone())),
        other => Err(EvalError::Type(format!("expected a matrix, found {}", other.type_name()))),
    }
}

pub fn call(name: &str, args: &[Value], ev: &mut Evaluator, env: &mut Environment) -> Result<Value, EvalError> {
    match name {
        "sqrt" => {
            arity(name, args, 1)?;
            match &args[0] {
                Value::Quantity(q) if !q.is_dimensionless() => Ok(Value::Quantity(q.powf(0.5)?)),
                Value::Complex { re, im } => {
                    let r = (re * re + im * im).sqrt().sqrt();
                    let a = im.atan2(*re) / 2.0;
                    Ok(Value::Complex { re: r * a.cos(), im: r * a.sin() })
                }
                other => {
                    let x = dimensionless(other)?;
                    if x < 0.0 { Ok(Value::Complex { re: 0.0, im: (-x).sqrt() }) } else { Ok(Value::Scalar(x.sqrt())) }
                }
            }
        }
        "cbrt" => { arity(name, args, 1)?; Ok(Value::Scalar(dimensionless(&args[0])?.cbrt())) }
        "abs" => {
            arity(name, args, 1)?;
            match &args[0] {
                Value::Quantity(q) => Ok(Value::Quantity(Quantity { si: q.si.abs(), unit: q.unit.clone() })),
                Value::Complex { re, im } => Ok(Value::Scalar((re * re + im * im).sqrt())),
                Value::Vector(v) => Ok(Value::Scalar(v.iter().map(|x| x * x).sum::<f64>().sqrt())),
                other => Ok(Value::Scalar(dimensionless(other)?.abs())),
            }
        }
        "exp" | "ln" | "log10" | "log2" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan"
        | "sinh" | "cosh" | "tanh" | "asinh" | "acosh" | "atanh" | "floor" | "ceil" | "trunc"
        | "sign" | "to_deg" | "to_rad" => {
            arity(name, args, 1)?;
            let x = dimensionless(&args[0])?;
            let v = match name {
                "exp" => x.exp(), "ln" => x.ln(), "log10" => x.log10(), "log2" => x.log2(),
                "sin" => x.sin(), "cos" => x.cos(), "tan" => x.tan(),
                "asin" => x.asin(), "acos" => x.acos(), "atan" => x.atan(),
                "sinh" => x.sinh(), "cosh" => x.cosh(), "tanh" => x.tanh(),
                "asinh" => x.asinh(), "acosh" => x.acosh(), "atanh" => x.atanh(),
                "floor" => x.floor(), "ceil" => x.ceil(), "trunc" => x.trunc(),
                "sign" => if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 },
                "to_deg" => x.to_degrees(), "to_rad" => x.to_radians(),
                _ => unreachable!(),
            };
            if !v.is_finite() && matches!(name, "ln" | "log10" | "log2" | "asin" | "acos" | "atanh" | "acosh") {
                return Err(EvalError::Math(format!("{name}({x}) is not defined for real numbers")));
            }
            Ok(Value::Scalar(v))
        }
        "log" => {
            if args.len() == 1 { return Ok(Value::Scalar(dimensionless(&args[0])?.ln())); }
            arity(name, args, 2)?;
            let x = dimensionless(&args[0])?;
            let b = dimensionless(&args[1])?;
            Ok(Value::Scalar(x.log(b)))
        }
        "atan2" | "hypot" | "mod" => {
            arity(name, args, 2)?;
            let x = dimensionless(&args[0])?;
            let y = dimensionless(&args[1])?;
            Ok(Value::Scalar(match name {
                "atan2" => x.atan2(y),
                "hypot" => x.hypot(y),
                _ => x % y,
            }))
        }
        "round" => {
            if args.len() == 1 { return Ok(Value::Scalar(dimensionless(&args[0])?.round())); }
            arity(name, args, 2)?;
            let x = dimensionless(&args[0])?;
            let d = dimensionless(&args[1])?.round() as i32;
            let f = 10f64.powi(d);
            Ok(Value::Scalar((x * f).round() / f))
        }
        "min" | "max" => {
            if args.is_empty() { return Err(EvalError::Args("min/max need arguments".into())); }
            let values: Vec<Value> = if args.len() == 1 {
                vector(&args[0])?.into_iter().map(Value::Scalar).collect()
            } else {
                args.to_vec()
            };
            let mut best = as_quantity(&values[0])?;
            for v in &values[1..] {
                let q = as_quantity(v)?;
                if !best.dim().compatible(&q.dim()) {
                    return Err(EvalError::Dimension("min/max operands have different dimensions".into()));
                }
                let take = if name == "min" { q.si < best.si } else { q.si > best.si };
                if take { best = q; }
            }
            Ok(Value::Quantity(best))
        }
        "sum" | "prod" | "mean" | "std" | "var" | "median" => {
            arity(name, args, 1)?;
            let mut v = vector(&args[0])?;
            if v.is_empty() { return Err(EvalError::Args("empty array".into())); }
            let n = v.len() as f64;
            let out = match name {
                "sum" => v.iter().sum(),
                "prod" => v.iter().product(),
                "mean" => v.iter().sum::<f64>() / n,
                "var" => {
                    let m = v.iter().sum::<f64>() / n;
                    v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0).max(1.0)
                }
                "std" => {
                    let m = v.iter().sum::<f64>() / n;
                    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0).max(1.0)).sqrt()
                }
                _ => { v.sort_by(|a, b| a.partial_cmp(b).unwrap()); v[v.len() / 2] }
            };
            Ok(Value::Scalar(out))
        }
        "len" => {
            arity(name, args, 1)?;
            Ok(Value::Scalar(match &args[0] {
                Value::Vector(v) => v.len() as f64,
                Value::Matrix(m) => (m.rows * m.cols) as f64,
                Value::Str(s) => s.chars().count() as f64,
                Value::Table(t) => t.rows.len() as f64,
                other => return Err(EvalError::Type(format!("len() does not accept {}", other.type_name()))),
            }))
        }
        "rows" | "cols" => {
            arity(name, args, 1)?;
            let m = matrix(&args[0])?;
            Ok(Value::Scalar(if name == "rows" { m.rows as f64 } else { m.cols as f64 }))
        }
        "linspace" => {
            arity(name, args, 3)?;
            let a = dimensionless(&args[0])?;
            let b = dimensionless(&args[1])?;
            let n = dimensionless(&args[2])?.round() as i64;
            if n < 2 || n > 1_000_000 { return Err(EvalError::Args("linspace count must be 2..1e6".into())); }
            Ok(Value::Vector((0..n).map(|i| a + (b - a) * (i as f64) / ((n - 1) as f64)).collect()))
        }
        "range" | "arange" => {
            if args.len() == 2 {
                let a = dimensionless(&args[0])?;
                let b = dimensionless(&args[1])?;
                return Ok(Value::Vector(float_range(a, 1.0, b)?));
            }
            arity(name, args, 3)?;
            let a = dimensionless(&args[0])?;
            let s = dimensionless(&args[1])?;
            let b = dimensionless(&args[2])?;
            Ok(Value::Vector(float_range(a, s, b)?))
        }
        "zeros" | "ones" => {
            let (r, c) = match args.len() {
                1 => { let n = dimensionless(&args[0])?.round() as usize; (n, n) }
                2 => (dimensionless(&args[0])?.round() as usize, dimensionless(&args[1])?.round() as usize),
                _ => return Err(EvalError::Args("zeros/ones take 1 or 2 sizes".into())),
            };
            if r == 0 || c == 0 || r * c > 4_000_000 { return Err(EvalError::Args("invalid matrix size".into())); }
            if args.len() == 1 {
                let fill = if name == "zeros" { 0.0 } else { 1.0 };
                return Ok(Value::Vector(vec![fill; r]));
            }
            let fill = if name == "zeros" { 0.0 } else { 1.0 };
            Ok(Value::Matrix(Matrix::new(r, c, vec![fill; r * c])))
        }
        "identity" | "eye" => {
            arity(name, args, 1)?;
            let n = dimensionless(&args[0])?.round() as usize;
            if n == 0 || n > 2000 { return Err(EvalError::Args("invalid identity size".into())); }
            Ok(Value::Matrix(Matrix::identity(n)))
        }
        "diag" => {
            arity(name, args, 1)?;
            match &args[0] {
                Value::Vector(v) => {
                    let n = v.len();
                    let mut m = Matrix::new(n, n, vec![0.0; n * n]);
                    for i in 0..n { m.data[i * n + i] = v[i]; }
                    Ok(Value::Matrix(m))
                }
                Value::Matrix(m) => Ok(Value::Vector((0..m.rows.min(m.cols)).map(|i| m.get(i, i)).collect())),
                other => Err(EvalError::Type(format!("diag() does not accept {}", other.type_name()))),
            }
        }
        "transpose" => { arity(name, args, 1)?; Ok(Value::Matrix(linalg::transpose(&matrix(&args[0])?))) }
        "det" => { arity(name, args, 1)?; Ok(Value::Scalar(linalg::det(&matrix(&args[0])?)?)) }
        "inv" => { arity(name, args, 1)?; Ok(Value::Matrix(linalg::inv(&matrix(&args[0])?)?)) }
        "rank" => { arity(name, args, 1)?; Ok(Value::Scalar(linalg::rank(&matrix(&args[0])?)? as f64)) }
        "trace" => {
            arity(name, args, 1)?;
            let m = matrix(&args[0])?;
            Ok(Value::Scalar((0..m.rows.min(m.cols)).map(|i| m.get(i, i)).sum()))
        }
        "norm" => {
            arity(name, args, 1)?;
            let v = vector(&args[0])?;
            Ok(Value::Scalar(v.iter().map(|x| x * x).sum::<f64>().sqrt()))
        }
        "dot" => {
            arity(name, args, 2)?;
            let a = vector(&args[0])?;
            let b = vector(&args[1])?;
            if a.len() != b.len() { return Err(EvalError::Args("dot needs equal lengths".into())); }
            Ok(Value::Scalar(a.iter().zip(&b).map(|(x, y)| x * y).sum()))
        }
        "cross" => {
            arity(name, args, 2)?;
            let a = vector(&args[0])?;
            let b = vector(&args[1])?;
            if a.len() != 3 || b.len() != 3 { return Err(EvalError::Args("cross product needs 3-vectors".into())); }
            Ok(Value::Vector(vec![
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]))
        }
        "solve" => {
            arity(name, args, 2)?;
            let a = matrix(&args[0])?;
            let b = matrix(&args[1])?;
            let r = linalg::solve(&a, &b)?;
            if r.cols == 1 { Ok(Value::Vector(r.data)) } else { Ok(Value::Matrix(r)) }
        }
        "eigenvals" | "eigenvectors" => {
            arity(name, args, 1)?;
            let (values, vectors) = linalg::eigen_symmetric(&matrix(&args[0])?)?;
            if name == "eigenvals" { Ok(Value::Vector(values)) } else { Ok(Value::Matrix(vectors)) }
        }
        "sort" | "reverse" => {
            arity(name, args, 1)?;
            let mut v = vector(&args[0])?;
            if name == "sort" { v.sort_by(|a, b| a.partial_cmp(b).unwrap()); } else { v.reverse(); }
            Ok(Value::Vector(v))
        }
        "polyfit" => {
            arity(name, args, 3)?;
            let x = vector(&args[0])?;
            let y = vector(&args[1])?;
            let d = dimensionless(&args[2])?.round() as usize;
            Ok(Value::Vector(linalg::polyfit(&x, &y, d)?))
        }
        "polyval" => {
            arity(name, args, 2)?;
            let c = vector(&args[0])?;
            let x = dimensionless(&args[1])?;
            Ok(Value::Scalar(c.iter().fold(0.0, |acc, k| acc * x + k)))
        }
        "interp1" => {
            arity(name, args, 3)?;
            let xs = vector(&args[0])?;
            let ys = vector(&args[1])?;
            if xs.len() != ys.len() || xs.len() < 2 { return Err(EvalError::Args("interp1 needs matching arrays".into())); }
            let xq = dimensionless(&args[2])?;
            let mut out = ys[0];
            if xq >= xs[xs.len() - 1] { out = ys[ys.len() - 1]; }
            for i in 0..xs.len() - 1 {
                if xq >= xs[i] && xq <= xs[i + 1] {
                    let t = (xq - xs[i]) / (xs[i + 1] - xs[i]);
                    out = ys[i] + t * (ys[i + 1] - ys[i]);
                    break;
                }
            }
            Ok(Value::Scalar(out))
        }
        "integrate" => {
            if args.len() == 2 { return Err(EvalError::RequiresPython("symbolic integrate()".into())); }
            arity(name, args, 3)?;
            let f = args[0].clone();
            let a = dimensionless(&args[1])?;
            let b = dimensionless(&args[2])?;
            let n = 2000usize;
            let h = (b - a) / n as f64;
            let mut acc = 0.0;
            for i in 0..=n {
                let x = a + h * i as f64;
                let y = dimensionless(&ev.call_value(&f, vec![Value::Scalar(x)], env)?)?;
                let w = if i == 0 || i == n { 1.0 } else if i % 2 == 1 { 4.0 } else { 2.0 };
                acc += w * y;
            }
            Ok(Value::Scalar(acc * h / 3.0))
        }
        "nderiv" => {
            arity(name, args, 2)?;
            let f = args[0].clone();
            let x = dimensionless(&args[1])?;
            let h = 1e-5 * (1.0 + x.abs());
            let fp = dimensionless(&ev.call_value(&f, vec![Value::Scalar(x + h)], env)?)?;
            let fm = dimensionless(&ev.call_value(&f, vec![Value::Scalar(x - h)], env)?)?;
            Ok(Value::Scalar((fp - fm) / (2.0 * h)))
        }
        "root" => {
            arity(name, args, 3)?;
            let f = args[0].clone();
            let mut a = dimensionless(&args[1])?;
            let mut b = dimensionless(&args[2])?;
            let mut fa = dimensionless(&ev.call_value(&f, vec![Value::Scalar(a)], env)?)?;
            let fb = dimensionless(&ev.call_value(&f, vec![Value::Scalar(b)], env)?)?;
            if fa * fb > 0.0 { return Err(EvalError::Math("root() requires a sign change on [a, b]".into())); }
            for _ in 0..200 {
                let m = 0.5 * (a + b);
                let fm = dimensionless(&ev.call_value(&f, vec![Value::Scalar(m)], env)?)?;
                if fm.abs() < 1e-12 || (b - a).abs() < 1e-14 { return Ok(Value::Scalar(m)); }
                if fa * fm < 0.0 { b = m; } else { a = m; fa = fm; }
            }
            Ok(Value::Scalar(0.5 * (a + b)))
        }
        "minimize" => {
            arity(name, args, 3)?;
            let f = args[0].clone();
            let mut a = dimensionless(&args[1])?;
            let mut b = dimensionless(&args[2])?;
            let gr = (5f64.sqrt() - 1.0) / 2.0;
            for _ in 0..200 {
                let c = b - gr * (b - a);
                let d = a + gr * (b - a);
                let fc = dimensionless(&ev.call_value(&f, vec![Value::Scalar(c)], env)?)?;
                let fd = dimensionless(&ev.call_value(&f, vec![Value::Scalar(d)], env)?)?;
                if fc < fd { b = d; } else { a = c; }
                if (b - a).abs() < 1e-12 { break; }
            }
            Ok(Value::Scalar(0.5 * (a + b)))
        }
        "odesolve" => {
            if args.len() < 4 { return Err(EvalError::Args("odesolve(f, t0, t1, y0[, n])".into())); }
            let f = args[0].clone();
            let t0 = dimensionless(&args[1])?;
            let t1 = dimensionless(&args[2])?;
            let y0 = match &args[3] {
                Value::Vector(v) => v.clone(),
                other => vec![dimensionless(other)?],
            };
            let n = if args.len() > 4 { dimensionless(&args[4])?.round() as usize } else { 200 };
            if n < 1 || n > 200_000 { return Err(EvalError::Args("odesolve step count out of range".into())); }
            let h = (t1 - t0) / n as f64;
            let k = y0.len();
            let mut data = Vec::with_capacity((n + 1) * (k + 1));
            let mut y = y0.clone();
            let mut t = t0;
            data.push(t);
            data.extend_from_slice(&y);
            for _ in 0..n {
                let k1 = ode_rhs(ev, env, &f, t, &y)?;
                let y2: Vec<f64> = y.iter().zip(&k1).map(|(a, b)| a + 0.5 * h * b).collect();
                let k2 = ode_rhs(ev, env, &f, t + 0.5 * h, &y2)?;
                let y3: Vec<f64> = y.iter().zip(&k2).map(|(a, b)| a + 0.5 * h * b).collect();
                let k3 = ode_rhs(ev, env, &f, t + 0.5 * h, &y3)?;
                let y4: Vec<f64> = y.iter().zip(&k3).map(|(a, b)| a + h * b).collect();
                let k4 = ode_rhs(ev, env, &f, t + h, &y4)?;
                for i in 0..k {
                    y[i] += h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
                }
                t += h;
                data.push(t);
                data.extend_from_slice(&y);
            }
            Ok(Value::Matrix(Matrix::new(n + 1, k + 1, data)))
        }
        "convert" => {
            arity(name, args, 2)?;
            let q = as_quantity(&args[0])?;
            let unit = match &args[1] {
                Value::Unit(u) => u.clone(),
                Value::Str(s) => lookup(s).ok_or_else(|| EvalError::Args(format!("unknown unit '{s}'")))?,
                other => return Err(EvalError::Type(format!(
                    "convert() target must be a unit, found {}", other.type_name()
                ))),
            };
            Ok(Value::Quantity(q.convert(&unit)?))
        }
        "si" => { arity(name, args, 1)?; Ok(Value::Scalar(as_quantity(&args[0])?.si)) }
        "strip_units" => { arity(name, args, 1)?; Ok(Value::Scalar(as_quantity(&args[0])?.value())) }
        "unit_of" => { arity(name, args, 1)?; Ok(Value::Str(as_quantity(&args[0])?.unit.name)) }
        "if" => {
            arity(name, args, 3)?;
            if truthy(&args[0])? { Ok(args[1].clone()) } else { Ok(args[2].clone()) }
        }
        "slider" => {
            if args.len() < 2 { return Err(EvalError::Args("slider(min, max[, step])".into())); }
            Ok(Value::Scalar(dimensionless(&args[0])?))
        }
        "input_number" => { arity(name, args, 1)?; Ok(args[0].clone()) }
        "checkbox" => { arity(name, args, 1)?; Ok(args[0].clone()) }
        "dropdown" => {
            if args.is_empty() { return Err(EvalError::Args("dropdown(options...)".into())); }
            Ok(args[0].clone())
        }
        "read_csv" => {
            arity(name, args, 1)?;
            let path = match &args[0] {
                Value::Str(s) => s.clone(),
                _ => return Err(EvalError::Args("read_csv expects a path string".into())),
            };
            let text = std::fs::read_to_string(&path)
                .map_err(|e| EvalError::Args(format!("cannot read '{path}': {e}")))?;
            Ok(Value::Table(parse_csv(&text)))
        }
        "column" => {
            arity(name, args, 2)?;
            let t = match &args[0] { Value::Table(t) => t, _ => return Err(EvalError::Args("column expects a table".into())) };
            let cname = match &args[1] { Value::Str(s) => s.clone(), _ => return Err(EvalError::Args("column name must be text".into())) };
            let idx = t.columns.iter().position(|c| *c == cname)
                .ok_or_else(|| EvalError::Args(format!("no column '{cname}'")))?;
            let mut out = Vec::new();
            for row in &t.rows {
                match row.get(idx) {
                    Some(Cell::Num(v)) => out.push(*v),
                    Some(Cell::Text(s)) => out.push(s.parse::<f64>().unwrap_or(f64::NAN)),
                    None => out.push(f64::NAN),
                }
            }
            Ok(Value::Vector(out))
        }
        "table_matrix" => {
            arity(name, args, 1)?;
            let t = match &args[0] { Value::Table(t) => t, _ => return Err(EvalError::Args("table_matrix expects a table".into())) };
            let cols = t.columns.len();
            let mut data = Vec::new();
            for row in &t.rows {
                for c in 0..cols {
                    data.push(match row.get(c) {
                        Some(Cell::Num(v)) => *v,
                        Some(Cell::Text(s)) => s.parse::<f64>().unwrap_or(f64::NAN),
                        None => f64::NAN,
                    });
                }
            }
            Ok(Value::Matrix(Matrix::new(t.rows.len(), cols, data)))
        }
        "print" => {
            let parts: Vec<String> = args.iter().map(|v| v.display()).collect();
            ev.output.push(parts.join(" "));
            Ok(Value::Str(parts.join(" ")))
        }
        "Re" | "Im" | "conj" | "arg" => {
            arity(name, args, 1)?;
            let (re, im) = match &args[0] {
                Value::Complex { re, im } => (*re, *im),
                other => (dimensionless(other)?, 0.0),
            };
            if name == "conj" {
                Ok(Value::Complex { re, im: -im })
            } else {
                Ok(Value::Scalar(match name {
                    "Re" => re, "Im" => im, _ => im.atan2(re),
                }))
            }
        }
        other => match crate::builtins_ext::call(other, args, ev, env) {
            Some(result) => result,
            None => Err(EvalError::UnknownFunction(other.to_string())),
        },
    }
}

fn ode_rhs(
    ev: &mut Evaluator,
    env: &mut Environment,
    f: &Value,
    t: f64,
    y: &[f64],
) -> Result<Vec<f64>, EvalError> {
    let mut args = vec![Value::Scalar(t)];
    if y.len() == 1 { args.push(Value::Scalar(y[0])); } else { args.push(Value::Vector(y.to_vec())); }
    let r = ev.call_value(f, args, env)?;
    match r {
        Value::Vector(v) => Ok(v),
        other => Ok(vec![dimensionless(&other)?]),
    }
}

fn float_range(a: f64, step: f64, b: f64) -> Result<Vec<f64>, EvalError> {
    if step == 0.0 { return Err(EvalError::Args("range step cannot be zero".into())); }
    let count = ((b - a) / step).floor() as i64 + 1;
    if count < 0 || count > 1_000_000 { return Err(EvalError::Args("range produces too many elements".into())); }
    Ok((0..count).map(|k| a + (k as f64) * step).collect())
}

fn parse_csv(text: &str) -> Table {
    let mut lines = text.lines();
    let split = |line: &str| -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut quoted = false;
        for ch in line.chars() {
            match ch {
                '"' => quoted = !quoted,
                ',' if !quoted => { out.push(cur.trim().to_string()); cur.clear(); }
                _ => cur.push(ch),
            }
        }
        out.push(cur.trim().to_string());
        out
    };
    let columns = lines.next().map(split).unwrap_or_default();
    let mut rows = Vec::new();
    for line in lines {
        if line.trim().is_empty() { continue; }
        let cells = split(line).into_iter().map(|s| match s.parse::<f64>() {
            Ok(v) => Cell::Num(v),
            Err(_) => Cell::Text(s),
        }).collect();
        rows.push(cells);
    }
    Table { columns, rows }
}
