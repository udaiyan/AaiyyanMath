//! Extended native function library (trig reciprocals, statistics, matrix assembly,
//! fitting, solvers, random). Mirrored in python/scripts/aaiyyan_runtime.py so the
//! generated Python stays runnable.
use crate::env::Environment;
use crate::interp::{dimensionless, Evaluator};
use crate::value::{Matrix, Value};
use crate::EvalError;
use std::cell::RefCell;

pub const NAMES: &[&str] = &[
    "sec", "csc", "cot", "asec", "acsc", "acot", "sech", "csch", "coth", "sinc",
    "frac", "clamp", "gcd", "lcm", "nroot", "log1p", "expm1",
    "geomean", "harmmean", "mode", "percentile", "corr", "cov", "rms", "skew", "kurt",
    "augment", "stack", "submatrix", "row", "col", "last", "vec", "reshape",
    "cumsum", "cumprod", "logspace",
    "linterp", "linfit", "expfit", "logfit", "pwrfit",
    "maximize", "newton", "runif", "rnorm", "seed", "unitsOf", "SIUnitsOf",
];

thread_local! {
    static RNG: RefCell<u64> = RefCell::new(0x2545F4914F6CDD1D);
}

fn next_unit() -> f64 {
    RNG.with(|r| {
        let mut s = r.borrow_mut();
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        (*s >> 11) as f64 / 9007199254740992.0
    })
}

fn num(v: &Value) -> Result<f64, EvalError> { dimensionless(v) }

fn vec_of(v: &Value) -> Result<Vec<f64>, EvalError> {
    match v {
        Value::Vector(a) => Ok(a.clone()),
        Value::Matrix(m) if m.rows == 1 || m.cols == 1 => Ok(m.data.clone()),
        Value::Scalar(x) => Ok(vec![*x]),
        other => Err(EvalError::Type(format!("expected a vector, found {}", other.type_name()))),
    }
}

fn mat_of(v: &Value) -> Result<Matrix, EvalError> {
    match v {
        Value::Matrix(m) => Ok(m.clone()),
        Value::Vector(a) => Ok(Matrix::new(a.len(), 1, a.clone())),
        other => Err(EvalError::Type(format!("expected a matrix, found {}", other.type_name()))),
    }
}

fn need(name: &str, args: &[Value], n: usize) -> Result<(), EvalError> {
    if args.len() != n {
        Err(EvalError::Args(format!("{name} expects {n} argument(s), got {}", args.len())))
    } else { Ok(()) }
}

fn mean(v: &[f64]) -> f64 { v.iter().sum::<f64>() / v.len().max(1) as f64 }

fn moment(v: &[f64], k: i32) -> f64 {
    let m = mean(v);
    v.iter().map(|x| (x - m).powi(k)).sum::<f64>() / v.len().max(1) as f64
}

/// Returns None when the name is not part of this extension library.
pub fn call(name: &str, args: &[Value], ev: &mut Evaluator, env: &mut Environment) -> Option<Result<Value, EvalError>> {
    if !NAMES.contains(&name) { return None; }
    Some(invoke(name, args, ev, env))
}

fn invoke(name: &str, args: &[Value], ev: &mut Evaluator, env: &mut Environment) -> Result<Value, EvalError> {
    match name {
        "sec" | "csc" | "cot" | "asec" | "acsc" | "acot" | "sech" | "csch" | "coth" | "sinc"
        | "frac" | "log1p" | "expm1" => {
            need(name, args, 1)?;
            let x = num(&args[0])?;
            let v = match name {
                "sec" => 1.0 / x.cos(),
                "csc" => 1.0 / x.sin(),
                "cot" => 1.0 / x.tan(),
                "asec" => (1.0 / x).acos(),
                "acsc" => (1.0 / x).asin(),
                "acot" => (1.0 / x).atan(),
                "sech" => 1.0 / x.cosh(),
                "csch" => 1.0 / x.sinh(),
                "coth" => 1.0 / x.tanh(),
                "sinc" => if x == 0.0 { 1.0 } else { x.sin() / x },
                "frac" => x - x.trunc(),
                "log1p" => x.ln_1p(),
                _ => x.exp_m1(),
            };
            Ok(Value::Scalar(v))
        }
        "clamp" => {
            need(name, args, 3)?;
            let x = num(&args[0])?;
            let lo = num(&args[1])?;
            let hi = num(&args[2])?;
            Ok(Value::Scalar(x.clamp(lo.min(hi), lo.max(hi))))
        }
        "gcd" | "lcm" => {
            need(name, args, 2)?;
            let a = num(&args[0])?.abs().round() as u64;
            let b = num(&args[1])?.abs().round() as u64;
            let (mut x, mut y) = (a.max(b), a.min(b));
            while y != 0 { let t = x % y; x = y; y = t; }
            let g = x.max(1);
            Ok(Value::Scalar(if name == "gcd" { g as f64 } else { (a / g * b) as f64 }))
        }
        "nroot" => {
            need(name, args, 2)?;
            let x = num(&args[0])?;
            let n = num(&args[1])?;
            if n == 0.0 { return Err(EvalError::Math("nroot: degree cannot be zero".into())); }
            Ok(Value::Scalar(if x < 0.0 && (n.round() as i64) % 2 != 0 { -(-x).powf(1.0 / n) } else { x.powf(1.0 / n) }))
        }
        "geomean" | "harmmean" | "rms" | "mode" | "skew" | "kurt" => {
            need(name, args, 1)?;
            let v = vec_of(&args[0])?;
            if v.is_empty() { return Err(EvalError::Args("empty array".into())); }
            let n = v.len() as f64;
            let out = match name {
                "geomean" => (v.iter().map(|x| x.ln()).sum::<f64>() / n).exp(),
                "harmmean" => n / v.iter().map(|x| 1.0 / x).sum::<f64>(),
                "rms" => (v.iter().map(|x| x * x).sum::<f64>() / n).sqrt(),
                "skew" => { let s = moment(&v, 2).sqrt(); if s == 0.0 { 0.0 } else { moment(&v, 3) / s.powi(3) } }
                "kurt" => { let s = moment(&v, 2); if s == 0.0 { 0.0 } else { moment(&v, 4) / (s * s) - 3.0 } }
                _ => {
                    let mut best = v[0];
                    let mut best_count = 0usize;
                    for x in &v {
                        let c = v.iter().filter(|y| (*y - x).abs() < 1e-12).count();
                        if c > best_count { best_count = c; best = *x; }
                    }
                    best
                }
            };
            Ok(Value::Scalar(out))
        }
        "percentile" => {
            need(name, args, 2)?;
            let mut v = vec_of(&args[0])?;
            let p = num(&args[1])?.clamp(0.0, 100.0);
            if v.is_empty() { return Err(EvalError::Args("empty array".into())); }
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let idx = (p / 100.0) * (v.len() - 1) as f64;
            let lo = idx.floor() as usize;
            let hi = idx.ceil() as usize;
            let t = idx - lo as f64;
            Ok(Value::Scalar(v[lo] + t * (v[hi] - v[lo])))
        }
        "corr" | "cov" => {
            need(name, args, 2)?;
            let x = vec_of(&args[0])?;
            let y = vec_of(&args[1])?;
            if x.len() != y.len() || x.len() < 2 { return Err(EvalError::Args("corr/cov need equal-length arrays".into())); }
            let (mx, my) = (mean(&x), mean(&y));
            let n = x.len() as f64;
            let sxy: f64 = x.iter().zip(&y).map(|(a, b)| (a - mx) * (b - my)).sum();
            let sxx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum();
            let syy: f64 = y.iter().map(|b| (b - my).powi(2)).sum();
            Ok(Value::Scalar(if name == "cov" { sxy / (n - 1.0) } else if sxx * syy <= 0.0 { 0.0 } else { sxy / (sxx * syy).sqrt() }))
        }
        "augment" | "stack" => {
            need(name, args, 2)?;
            let a = mat_of(&args[0])?;
            let b = mat_of(&args[1])?;
            if name == "augment" {
                if a.rows != b.rows { return Err(EvalError::Args("augment: row counts differ".into())); }
                let mut data = Vec::new();
                for r in 0..a.rows {
                    for c in 0..a.cols { data.push(a.get(r, c)); }
                    for c in 0..b.cols { data.push(b.get(r, c)); }
                }
                Ok(Value::Matrix(Matrix::new(a.rows, a.cols + b.cols, data)))
            } else {
                if a.cols != b.cols { return Err(EvalError::Args("stack: column counts differ".into())); }
                let mut data = a.data.clone();
                data.extend_from_slice(&b.data);
                Ok(Value::Matrix(Matrix::new(a.rows + b.rows, a.cols, data)))
            }
        }
        "submatrix" => {
            need(name, args, 5)?;
            let m = mat_of(&args[0])?;
            let r0 = num(&args[1])?.round() as usize;
            let r1 = num(&args[2])?.round() as usize;
            let c0 = num(&args[3])?.round() as usize;
            let c1 = num(&args[4])?.round() as usize;
            if r1 < r0 || c1 < c0 || r1 >= m.rows || c1 >= m.cols {
                return Err(EvalError::Index("submatrix range out of bounds".into()));
            }
            let mut data = Vec::new();
            for r in r0..=r1 { for c in c0..=c1 { data.push(m.get(r, c)); } }
            Ok(Value::Matrix(Matrix::new(r1 - r0 + 1, c1 - c0 + 1, data)))
        }
        "row" | "col" => {
            need(name, args, 2)?;
            let m = mat_of(&args[0])?;
            let i = num(&args[1])?.round() as usize;
            if name == "row" {
                if i >= m.rows { return Err(EvalError::Index("row index out of range".into())); }
                Ok(Value::Vector((0..m.cols).map(|c| m.get(i, c)).collect()))
            } else {
                if i >= m.cols { return Err(EvalError::Index("column index out of range".into())); }
                Ok(Value::Vector((0..m.rows).map(|r| m.get(r, i)).collect()))
            }
        }
        "last" => {
            need(name, args, 1)?;
            let v = vec_of(&args[0])?;
            v.last().copied().map(Value::Scalar).ok_or_else(|| EvalError::Args("empty array".into()))
        }
        "vec" => {
            need(name, args, 1)?;
            Ok(Value::Vector(mat_of(&args[0])?.data))
        }
        "reshape" => {
            need(name, args, 3)?;
            let v = vec_of(&args[0])?;
            let r = num(&args[1])?.round() as usize;
            let c = num(&args[2])?.round() as usize;
            if r * c != v.len() || r == 0 || c == 0 {
                return Err(EvalError::Args("reshape: element count must match rows*cols".into()));
            }
            Ok(Value::Matrix(Matrix::new(r, c, v)))
        }
        "cumsum" | "cumprod" => {
            need(name, args, 1)?;
            let v = vec_of(&args[0])?;
            let mut acc = if name == "cumsum" { 0.0 } else { 1.0 };
            let mut out = Vec::with_capacity(v.len());
            for x in v {
                acc = if name == "cumsum" { acc + x } else { acc * x };
                out.push(acc);
            }
            Ok(Value::Vector(out))
        }
        "logspace" => {
            need(name, args, 3)?;
            let a = num(&args[0])?;
            let b = num(&args[1])?;
            let n = num(&args[2])?.round() as i64;
            if n < 2 || n > 1_000_000 { return Err(EvalError::Args("logspace count must be 2..1e6".into())); }
            Ok(Value::Vector((0..n).map(|i| 10f64.powf(a + (b - a) * i as f64 / (n - 1) as f64)).collect()))
        }
        "linterp" => crate::builtins::call("interp1", args, ev, env),
        "linfit" | "expfit" | "logfit" | "pwrfit" => {
            need(name, args, 2)?;
            let x = vec_of(&args[0])?;
            let y = vec_of(&args[1])?;
            if x.len() != y.len() || x.len() < 2 { return Err(EvalError::Args("fit needs equal-length arrays".into())); }
            let (tx, ty): (Vec<f64>, Vec<f64>) = match name {
                "expfit" => (x.clone(), y.iter().map(|v| v.ln()).collect()),
                "logfit" => (x.iter().map(|v| v.ln()).collect(), y.clone()),
                "pwrfit" => (x.iter().map(|v| v.ln()).collect(), y.iter().map(|v| v.ln()).collect()),
                _ => (x.clone(), y.clone()),
            };
            let mx = mean(&tx);
            let my = mean(&ty);
            let sxx: f64 = tx.iter().map(|v| (v - mx).powi(2)).sum();
            if sxx == 0.0 { return Err(EvalError::Math("fit: x values are constant".into())); }
            let slope: f64 = tx.iter().zip(&ty).map(|(a, b)| (a - mx) * (b - my)).sum::<f64>() / sxx;
            let intercept = my - slope * mx;
            let out = match name {
                "linfit" | "logfit" => vec![slope, intercept],
                _ => vec![intercept.exp(), slope],
            };
            Ok(Value::Vector(out))
        }
        "maximize" | "newton" => {
            if name == "maximize" {
                need(name, args, 3)?;
                let f = args[0].clone();
                let a = num(&args[1])?;
                let b = num(&args[2])?;
                let gr = (5f64.sqrt() - 1.0) / 2.0;
                let (mut lo, mut hi) = (a, b);
                for _ in 0..200 {
                    let c = hi - gr * (hi - lo);
                    let d = lo + gr * (hi - lo);
                    let fc = num(&ev.call_value(&f, vec![Value::Scalar(c)], env)?)?;
                    let fd = num(&ev.call_value(&f, vec![Value::Scalar(d)], env)?)?;
                    if fc > fd { hi = d; } else { lo = c; }
                    if (hi - lo).abs() < 1e-12 { break; }
                }
                Ok(Value::Scalar(0.5 * (lo + hi)))
            } else {
                need(name, args, 2)?;
                let f = args[0].clone();
                let mut x = num(&args[1])?;
                for _ in 0..100 {
                    let h = 1e-6 * (1.0 + x.abs());
                    let fx = num(&ev.call_value(&f, vec![Value::Scalar(x)], env)?)?;
                    if fx.abs() < 1e-13 { return Ok(Value::Scalar(x)); }
                    let fp = num(&ev.call_value(&f, vec![Value::Scalar(x + h)], env)?)?;
                    let fm = num(&ev.call_value(&f, vec![Value::Scalar(x - h)], env)?)?;
                    let d = (fp - fm) / (2.0 * h);
                    if d.abs() < 1e-18 { return Err(EvalError::Math("newton: zero derivative".into())); }
                    let next = x - fx / d;
                    if (next - x).abs() < 1e-14 { return Ok(Value::Scalar(next)); }
                    x = next;
                }
                Ok(Value::Scalar(x))
            }
        }
        "seed" => {
            need(name, args, 1)?;
            let s = num(&args[0])?.round() as u64;
            RNG.with(|r| *r.borrow_mut() = if s == 0 { 1 } else { s });
            Ok(Value::Scalar(s as f64))
        }
        "runif" | "rnorm" => {
            need(name, args, 3)?;
            let n = num(&args[0])?.round() as i64;
            let a = num(&args[1])?;
            let b = num(&args[2])?;
            if n < 1 || n > 1_000_000 { return Err(EvalError::Args("sample count out of range".into())); }
            let mut out = Vec::with_capacity(n as usize);
            for _ in 0..n {
                if name == "runif" {
                    out.push(a + (b - a) * next_unit());
                } else {
                    let u1 = next_unit().max(1e-12);
                    let u2 = next_unit();
                    let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
                    out.push(a + b * z);
                }
            }
            Ok(Value::Vector(out))
        }
        "unitsOf" => crate::builtins::call("unit_of", args, ev, env),
        "SIUnitsOf" => crate::builtins::call("si", args, ev, env),
        other => Err(EvalError::UnknownFunction(other.to_string())),
    }
}

#[cfg(test)]
mod parity_tests {
    use super::*;

    fn ex(name: &str, args: &[Value]) -> Value {
        let mut e = Evaluator::new(Default::default());
        let mut env = Environment::new();
        call(name, args, &mut e, &mut env)
            .unwrap_or_else(|| panic!("{name} not in builtins_ext"))
            .unwrap_or_else(|err| panic!("{name} failed: {err}"))
    }
    fn sc(name: &str, args: &[Value]) -> f64 { match ex(name,args){ Value::Scalar(x)=>x, _=>panic!("{name}: scalar") } }
    fn vv(name: &str, args: &[Value]) -> Vec<f64> { match ex(name,args){ Value::Vector(a)=>a, Value::Matrix(m)=>m.data, _=>panic!("{name}: array") } }
    fn mm(name: &str, args: &[Value]) -> Matrix { match ex(name,args){ Value::Matrix(m)=>m, Value::Vector(a)=>Matrix::new(a.len(),1,a), _=>panic!("{name}: matrix") } }
    fn v1(d: &[f64]) -> Value { Value::Vector(d.to_vec()) }
    fn m2(r:usize,c:usize,d:&[f64])->Value{ Value::Matrix(Matrix::new(r,c,d.to_vec())) }
    fn near(a:f64,b:f64)->bool{ (a-b).abs()<1e-9 }
    fn nearv(a:&[f64],b:&[f64])->bool{ a.len()==b.len() && a.iter().zip(b).all(|(x,y)| near(*x,*y)) }
    const PI: f64 = std::f64::consts::PI;

    #[test] fn reciprocal_trig() {
        assert!(near(sc("sec",&[Value::Scalar(0.0)]),1.0));
        assert!(near(sc("csc",&[Value::Scalar(PI/2.0)]),1.0));
        assert!(near(sc("cot",&[Value::Scalar(PI/4.0)]),1.0));
        assert!(near(sc("asec",&[Value::Scalar(1.0)]),0.0));
        assert!(near(sc("acsc",&[Value::Scalar(1.0)]),PI/2.0));
        assert!(near(sc("acot",&[Value::Scalar(1.0)]),PI/4.0));
        assert!(near(sc("sinc",&[Value::Scalar(0.0)]),1.0));
        assert!(sc("sinc",&[Value::Scalar(PI)]).abs()<1e-9);
        assert!(near(sc("sech",&[Value::Scalar(0.0)]),1.0));
    }
    #[test] fn number_theory_and_clamp() {
        assert!(near(sc("frac",&[Value::Scalar(1.5)]),0.5));
        assert!(near(sc("frac",&[Value::Scalar(-1.5)]),-0.5));
        assert!(near(sc("clamp",&[Value::Scalar(5.0),Value::Scalar(1.0),Value::Scalar(3.0)]),3.0));
        assert!(near(sc("gcd",&[Value::Scalar(12.0),Value::Scalar(18.0)]),6.0));
        assert!(near(sc("lcm",&[Value::Scalar(4.0),Value::Scalar(6.0)]),12.0));
        assert!(near(sc("nroot",&[Value::Scalar(27.0),Value::Scalar(3.0)]),3.0));
        assert!(near(sc("nroot",&[Value::Scalar(-8.0),Value::Scalar(3.0)]),-2.0));
        assert!(near(sc("log1p",&[Value::Scalar(0.0)]),0.0));
        assert!(near(sc("expm1",&[Value::Scalar(0.0)]),0.0));
    }
    #[test] fn statistics() {
        assert!(near(sc("geomean",&[v1(&[1.0,4.0])]),2.0));
        assert!(near(sc("harmmean",&[v1(&[1.0,4.0])]),1.6));
        assert!(near(sc("rms",&[v1(&[3.0,4.0])]),(12.5f64).sqrt()));
        assert!(near(sc("mode",&[v1(&[1.0,2.0,2.0,3.0])]),2.0));
        assert!(near(sc("percentile",&[v1(&[1.0,2.0,3.0,4.0]),Value::Scalar(50.0)]),2.5));
        assert!(near(sc("corr",&[v1(&[1.0,2.0,3.0]),v1(&[2.0,4.0,6.0])]),1.0));
        assert!(near(sc("cov",&[v1(&[1.0,2.0,3.0]),v1(&[1.0,2.0,3.0])]),1.0));
        assert!(sc("skew",&[v1(&[1.0,2.0,3.0,4.0,5.0])]).abs()<1e-9);
        // Excess kurtosis of a discrete uniform sample is -1.3, not 0.
        assert!((sc("kurt",&[v1(&[1.0,2.0,3.0,4.0,5.0])]) + 1.3).abs() < 1e-9);
    }
    #[test] fn matrix_assembly() {
        let a = m2(2,2,&[1.0,2.0,3.0,4.0]);
        assert_eq!(mm("augment",&[a.clone(),a.clone()]).cols,4);
        assert_eq!(mm("stack",&[a.clone(),a.clone()]).rows,4);
        assert!(nearv(&vv("row",&[a.clone(),Value::Scalar(0.0)]),&[1.0,2.0]));
        assert!(nearv(&vv("col",&[a.clone(),Value::Scalar(0.0)]),&[1.0,3.0]));
        assert!(near(sc("last",&[v1(&[7.0,8.0,9.0])]),9.0));
        assert!(nearv(&vv("cumsum",&[v1(&[1.0,2.0,3.0])]),&[1.0,3.0,6.0]));
        assert!(nearv(&vv("cumprod",&[v1(&[1.0,2.0,3.0])]),&[1.0,2.0,6.0]));
        assert!(nearv(&vv("logspace",&[Value::Scalar(0.0),Value::Scalar(2.0),Value::Scalar(3.0)]),&[1.0,10.0,100.0]));
    }
    #[test] fn interpolation_and_fitting() {
        assert!(near(sc("linterp",&[v1(&[0.0,1.0,2.0]),v1(&[0.0,1.0,4.0]),Value::Scalar(1.5)]),2.5));
        // linfit returns [slope, intercept] (descending powers).
        // y = 2x + 2 at x = 0,1,2  ->  [slope, intercept] = [2, 2].
        assert!(nearv(&vv("linfit", &[v1(&[0.0, 1.0, 2.0]), v1(&[2.0, 4.0, 6.0])]), &[2.0, 2.0]));
        // Zero-intercept data pins the coefficient order unambiguously.
        assert!(nearv(&vv("linfit", &[v1(&[0.0, 1.0, 2.0]), v1(&[0.0, 2.0, 4.0])]), &[2.0, 0.0]));
        let y: Vec<f64> = (0..3).map(|i| 2.0 * (0.5*i as f64).exp()).collect();
        let f = vv("expfit",&[v1(&[0.0,1.0,2.0]), v1(&y)]);
        assert!(near(f[0],2.0) && near(f[1],0.5));
        let pf = vv("pwrfit",&[v1(&[1.0,2.0,4.0]), v1(&[1.0,2.0,4.0])]);
        assert!(near(pf[1],1.0));
    }
    #[test] fn random_seed_determinism() {
        sc("seed",&[Value::Scalar(3.0)]);
        let a = vv("runif",&[Value::Scalar(6.0),Value::Scalar(0.0),Value::Scalar(1.0)]);
        sc("seed",&[Value::Scalar(3.0)]);
        let b = vv("runif",&[Value::Scalar(6.0),Value::Scalar(0.0),Value::Scalar(1.0)]);
        assert!(nearv(&a,&b));
        assert_eq!(vv("rnorm",&[Value::Scalar(5.0),Value::Scalar(0.0),Value::Scalar(1.0)]).len(),5);
    }
    #[test] fn unknown_returns_none() {
        let mut e = Evaluator::new(Default::default());
        let mut env = Environment::new();
        assert!(call("zzz_not_a_fn", &[], &mut e, &mut env).is_none());
    }
}
