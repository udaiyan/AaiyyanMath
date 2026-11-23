//! The whole maths core: one AST, two parsers (LaTeX, Python), two renderers,
//! and a native evaluator. Deliberately small and dependency-free.
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Num(f64),
    Var(String),
    Neg(Box<Node>),
    Bin(&'static str, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
    Int { body: Box<Node>, var: String, lo: Box<Node>, hi: Box<Node> },
    Sum { body: Box<Node>, var: String, lo: Box<Node>, hi: Box<Node>, prod: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub name: Option<String>,
    pub expr: Node,
}

pub const FUNCS: &[&str] = &[
    "sqrt", "nroot", "abs", "exp", "ln", "log10", "log2", "sin", "cos", "tan",
    "asin", "acos", "atan", "sinh", "cosh", "tanh", "floor", "ceil", "round",
    "sign", "min", "max", "sum", "mean", "std", "fact", "deg", "rad",
];

fn prec(op: &str) -> u8 {
    match op { "+" | "-" => 1, "*" | "/" => 2, "^" => 4, _ => 0 }
}

// ───────────────────────── shared tokenizer ─────────────────────────
#[derive(Clone, Debug, PartialEq)]
pub enum Tok { Num(f64), Id(String), Op(String), LP, RP, Comma, Colon, Lambda }

pub fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() { i += 1; continue; }
        if ch == '#' { break; }
        match ch {
            '(' => { out.push(Tok::LP); i += 1; }
            ')' => { out.push(Tok::RP); i += 1; }
            ',' => { out.push(Tok::Comma); i += 1; }
            ':' => { out.push(Tok::Colon); i += 1; }
            '0'..='9' | '.' => {
                let mut s = String::new();
                while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') { s.push(c[i]); i += 1; }
                if i < c.len() && (c[i] == 'e' || c[i] == 'E') {
                    let save = i; let mut e = String::from("e"); i += 1;
                    if i < c.len() && (c[i] == '+' || c[i] == '-') { e.push(c[i]); i += 1; }
                    if i < c.len() && c[i].is_ascii_digit() {
                        while i < c.len() && c[i].is_ascii_digit() { e.push(c[i]); i += 1; }
                        s.push_str(&e);
                    } else { i = save; }
                }
                out.push(Tok::Num(s.parse().map_err(|_| format!("bad number {s}"))?));
            }
            '*' => {
                if i + 1 < c.len() && c[i + 1] == '*' { out.push(Tok::Op("^".into())); i += 2; }
                else { out.push(Tok::Op("*".into())); i += 1; }
            }
            '=' => { out.push(Tok::Op("=".into())); i += 1; }
            '+' | '-' | '/' | '^' => { out.push(Tok::Op(ch.to_string())); i += 1; }
            _ if ch.is_alphabetic() || ch == '_' => {
                let mut s = String::new();
                while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') { s.push(c[i]); i += 1; }
                if s == "lambda" { out.push(Tok::Lambda); } else { out.push(Tok::Id(s)); }
            }
            other => return Err(format!("unexpected character '{other}'")),
        }
    }
    Ok(out)
}

// ───────────────────────── expression parser ─────────────────────────
pub struct P { pub t: Vec<Tok>, pub i: usize }

impl P {
    pub fn peek(&self) -> Option<&Tok> { self.t.get(self.i) }
    pub fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) { self.i += 1; true } else { false }
    }
    pub fn expr(&mut self, min: u8) -> Result<Node, String> {
        let mut lhs = self.atom()?;
        loop {
            if let Some(Tok::Op(op)) = self.peek().cloned() {
                let p = prec(&op);
                if p == 0 || p < min { break; }
                self.i += 1;
                let next = if op == "^" { p } else { p + 1 };
                let rhs = self.expr(next)?;
                let static_op: &'static str = match op.as_str() {
                    "+" => "+", "-" => "-", "*" => "*", "/" => "/", "^" => "^", _ => unreachable!(),
                };
                lhs = Node::Bin(static_op, Box::new(lhs), Box::new(rhs));
                continue;
            }
            // implicit multiplication: 2x, 3(x+1)
            if min <= 2 && matches!(self.peek(), Some(Tok::Num(_)) | Some(Tok::Id(_)) | Some(Tok::LP)) {
                let rhs = self.expr(3)?;
                lhs = Node::Bin("*", Box::new(lhs), Box::new(rhs));
                continue;
            }
            break;
        }
        Ok(lhs)
    }
    fn atom(&mut self) -> Result<Node, String> {
        match self.t.get(self.i).cloned() {
            Some(Tok::Num(v)) => { self.i += 1; Ok(Node::Num(v)) }
            Some(Tok::Op(o)) if o == "-" => { self.i += 1; Ok(Node::Neg(Box::new(self.expr(3)?))) }
            Some(Tok::Op(o)) if o == "+" => { self.i += 1; self.expr(3) }
            Some(Tok::LP) => {
                self.i += 1;
                let e = self.expr(0)?;
                if !self.eat(&Tok::RP) { return Err("expected )".into()); }
                Ok(e)
            }
            Some(Tok::Id(name)) => {
                self.i += 1;
                if !self.eat(&Tok::LP) { return Ok(Node::Var(name)); }
                self.call(name)
            }
            Some(Tok::Lambda) => Err("lambda is only valid inside integrate/sumrange".into()),
            other => Err(format!("unexpected token {other:?}")),
        }
    }
    fn call(&mut self, name: String) -> Result<Node, String> {
        if matches!(name.as_str(), "integrate" | "sumrange" | "prodrange") {
            if !self.eat(&Tok::Lambda) { return Err(format!("{name} expects lambda v: body")); }
            let var = match self.t.get(self.i).cloned() {
                Some(Tok::Id(v)) => { self.i += 1; v }
                _ => return Err("expected a variable after lambda".into()),
            };
            if !self.eat(&Tok::Colon) { return Err("expected : after lambda variable".into()); }
            let body = self.expr(0)?;
            if !self.eat(&Tok::Comma) { return Err("expected , after lambda body".into()); }
            let lo = self.expr(0)?;
            if !self.eat(&Tok::Comma) { return Err("expected , after lower limit".into()); }
            let hi = self.expr(0)?;
            if !self.eat(&Tok::RP) { return Err("expected )".into()); }
            return Ok(if name == "integrate" {
                Node::Int { body: Box::new(body), var, lo: Box::new(lo), hi: Box::new(hi) }
            } else {
                Node::Sum { body: Box::new(body), var, lo: Box::new(lo), hi: Box::new(hi), prod: name == "prodrange" }
            });
        }
        let mut args = Vec::new();
        if !self.eat(&Tok::RP) {
            loop {
                args.push(self.expr(0)?);
                if self.eat(&Tok::Comma) { continue; }
                if !self.eat(&Tok::RP) { return Err("expected , or )".into()); }
                break;
            }
        }
        Ok(Node::Call(name, args))
    }
}

/// One Python line -> statement. `x = 3` assigns; anything else is an expression.
pub fn parse_python(line: &str) -> Result<Option<Stmt>, String> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') { return Ok(None); }
    let t = lex(s)?;
    if t.is_empty() { return Ok(None); }
    let name = match (&t[0], t.get(1)) {
        (Tok::Id(n), Some(Tok::Op(o))) if o == "=" => Some(n.clone()),
        _ => None,
    };
    let mut p = P { t, i: if name.is_some() { 2 } else { 0 } };
    let expr = p.expr(0)?;
    if p.i != p.t.len() { return Err(format!("unexpected trailing input")); }
    Ok(Some(Stmt { name, expr }))
}

// ───────────────────────── Python renderer ─────────────────────────
fn py(n: &Node, parent: u8) -> String {
    match n {
        Node::Num(v) => fmt_num(*v),
        Node::Var(s) => s.clone(),
        Node::Neg(a) => format!("-{}", py(a, 3)),
        Node::Bin(op, a, b) => {
            let p = prec(op);
            let s = format!("{} {} {}", py(a, p), op, py(b, p + 1));
            if p < parent { format!("({s})") } else { s }
        }
        Node::Call(f, args) => {
            let a: Vec<String> = args.iter().map(|x| py(x, 0)).collect();
            format!("{f}({})", a.join(", "))
        }
        Node::Int { body, var, lo, hi } =>
            format!("integrate(lambda {}: {}, {}, {})", var, py(body, 0), py(lo, 0), py(hi, 0)),
        Node::Sum { body, var, lo, hi, prod } =>
            format!("{}(lambda {}: {}, {}, {})", if *prod { "prodrange" } else { "sumrange" },
                    var, py(body, 0), py(lo, 0), py(hi, 0)),
    }
}

pub fn to_python(s: &Stmt) -> String {
    match &s.name {
        Some(n) => format!("{n} = {}", py(&s.expr, 0)),
        None => py(&s.expr, 0),
    }
}

pub fn fmt_num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 { return format!("{}", v as i64); }
    let a = v.abs();
    if a != 0.0 && (a >= 1e9 || a < 1e-6) { return format!("{v:.6e}"); }
    let s = format!("{v:.10}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

// ───────────────────────── evaluator ─────────────────────────
pub type Env = HashMap<String, f64>;

pub fn eval(n: &Node, env: &mut Env) -> Result<f64, String> {
    Ok(match n {
        Node::Num(v) => *v,
        Node::Var(s) => match s.as_str() {
            "pi" | "PI" => std::f64::consts::PI,
            "e" => std::f64::consts::E,
            "tau" => std::f64::consts::TAU,
            _ => *env.get(s).ok_or_else(|| format!("'{s}' is not defined"))?,
        },
        Node::Neg(a) => -eval(a, env)?,
        Node::Bin(op, a, b) => {
            let x = eval(a, env)?; let y = eval(b, env)?;
            match *op {
                "+" => x + y, "-" => x - y, "*" => x * y,
                "/" => { if y == 0.0 { return Err("division by zero".into()); } x / y }
                "^" => x.powf(y),
                _ => unreachable!(),
            }
        }
        Node::Call(f, args) => {
            let mut v = Vec::with_capacity(args.len());
            for a in args { v.push(eval(a, env)?); }
            apply(f, &v)?
        }
        Node::Int { body, var, lo, hi } => {
            let a = eval(lo, env)?; let b = eval(hi, env)?;
            let n = 2000usize;
            let h = (b - a) / n as f64;
            if h == 0.0 { 0.0 } else {
                let mut f = |x: f64| -> Result<f64, String> {
                    let save = env.get(var).copied();
                    env.insert(var.clone(), x);
                    let r = eval(body, env);
                    match save { Some(s) => { env.insert(var.clone(), s); } None => { env.remove(var); } }
                    r
                };
                let mut acc = f(a)? + f(b)?;
                for k in 1..n { acc += f(a + k as f64 * h)? * if k % 2 == 1 { 4.0 } else { 2.0 }; }
                acc * h / 3.0
            }
        }
        Node::Sum { body, var, lo, hi, prod } => {
            let a = eval(lo, env)?.round() as i64;
            let b = eval(hi, env)?.round() as i64;
            if b - a > 1_000_000 { return Err("range too large (max 1e6 terms)".into()); }
            let save = env.get(var).copied();
            let mut acc = if *prod { 1.0 } else { 0.0 };
            for k in a..=b {
                env.insert(var.clone(), k as f64);
                let t = eval(body, env)?;
                if *prod { acc *= t; } else { acc += t; }
            }
            match save { Some(s) => { env.insert(var.clone(), s); } None => { env.remove(var); } }
            acc
        }
    })
}

fn apply(f: &str, v: &[f64]) -> Result<f64, String> {
    let one = || -> Result<f64, String> {
        if v.len() == 1 { Ok(v[0]) } else { Err(format!("{f} takes 1 argument")) }
    };
    let two = || -> Result<(f64, f64), String> {
        if v.len() == 2 { Ok((v[0], v[1])) } else { Err(format!("{f} takes 2 arguments")) }
    };
    Ok(match f {
        "sqrt" => one()?.sqrt(),
        "nroot" => { let (x, n) = two()?; if x < 0.0 && n.round() as i64 % 2 != 0 { -(-x).powf(1.0 / n) } else { x.powf(1.0 / n) } }
        "abs" => one()?.abs(),
        "exp" => one()?.exp(),
        "ln" => one()?.ln(),
        "log10" => one()?.log10(),
        "log2" => one()?.log2(),
        "sin" => one()?.sin(), "cos" => one()?.cos(), "tan" => one()?.tan(),
        "asin" => one()?.asin(), "acos" => one()?.acos(), "atan" => one()?.atan(),
        "sinh" => one()?.sinh(), "cosh" => one()?.cosh(), "tanh" => one()?.tanh(),
        "floor" => one()?.floor(), "ceil" => one()?.ceil(), "round" => one()?.round(),
        "sign" => one()?.signum(),
        "deg" => one()?.to_degrees(), "rad" => one()?.to_radians(),
        "fact" => { let n = one()?.round() as i64; if !(0..=170).contains(&n) { return Err("fact needs 0..170".into()); }
                    (1..=n).map(|k| k as f64).product::<f64>() }
        "min" => v.iter().cloned().fold(f64::INFINITY, f64::min),
        "max" => v.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        "sum" => v.iter().sum(),
        "mean" => { if v.is_empty() { return Err("mean needs arguments".into()); } v.iter().sum::<f64>() / v.len() as f64 }
        "std" => { if v.len() < 2 { return Err("std needs 2+ arguments".into()); }
                   let m = v.iter().sum::<f64>() / v.len() as f64;
                   (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt() }
        other => return Err(format!("unknown function '{other}'")),
    })
}

pub const PRELUDE: &str = r#"# AaiyyanMath — runnable helpers
import math
from math import *

def nroot(x, n): return math.copysign(abs(x) ** (1.0 / n), x) if x < 0 else x ** (1.0 / n)
def ln(x): return math.log(x)
def log10(x): return math.log10(x)
def log2(x): return math.log2(x)
def sign(x): return (x > 0) - (x < 0)
def deg(x): return math.degrees(x)
def rad(x): return math.radians(x)
def fact(n): return math.factorial(int(n))
def mean(*a): return sum(a) / len(a)
def std(*a):
    m = mean(*a); return (sum((x - m) ** 2 for x in a) / (len(a) - 1)) ** 0.5

def integrate(f, a, b, n=2000):
    h = (b - a) / n
    if h == 0: return 0.0
    s = f(a) + f(b)
    for k in range(1, n): s += f(a + k * h) * (4 if k % 2 else 2)
    return s * h / 3

def sumrange(f, a, b): return sum(f(k) for k in range(int(a), int(b) + 1))
def prodrange(f, a, b):
    r = 1
    for k in range(int(a), int(b) + 1): r *= f(k)
    return r

"#;

#[cfg(test)]
mod tests {
    use super::*;
    fn val(src: &str) -> f64 {
        let s = parse_python(src).unwrap().unwrap();
        eval(&s.expr, &mut Env::new()).unwrap()
    }
    #[test] fn arithmetic() { assert_eq!(val("1+2*3"), 7.0); }
    #[test] fn power_right_assoc() { assert_eq!(val("2**3**2"), 512.0); }
    #[test] fn implicit_times() { assert_eq!(val("2(3+4)"), 14.0); }
    #[test] fn integral_of_square() { assert!((val("integrate(lambda x: x**2, 0, 1)") - 1.0 / 3.0).abs() < 1e-9); }
    #[test] fn sum_of_squares() { assert_eq!(val("sumrange(lambda k: k**2, 1, 10)"), 385.0); }
    #[test] fn product() { assert_eq!(val("prodrange(lambda k: k, 1, 5)"), 120.0); }
    #[test] fn assignment_roundtrip() {
        let s = parse_python("x = 3").unwrap().unwrap();
        assert_eq!(s.name.as_deref(), Some("x"));
        assert_eq!(to_python(&s), "x = 3");
    }
    #[test] fn nested_calls() { assert!((val("sqrt(abs(-16))") - 4.0).abs() < 1e-12); }
    #[test] fn constants() { assert!((val("sin(pi/2)") - 1.0).abs() < 1e-12); }
    #[test] fn undefined_is_error() {
        let s = parse_python("y+1").unwrap().unwrap();
        assert!(eval(&s.expr, &mut Env::new()).is_err());
    }
}
