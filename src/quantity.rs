use std::collections::HashMap;
use std::fmt;
use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;

use exmex::{BinOp, Express, FlatEx, MakeOperators, Operator};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dim(pub [i8; 7]);

impl Dim {
    pub const ZERO: Dim = Dim([0; 7]);
    pub const MASS: Dim = Dim([1, 0, 0, 0, 0, 0, 0]);
    pub const LENGTH: Dim = Dim([0, 1, 0, 0, 0, 0, 0]);
    pub const TIME: Dim = Dim([0, 0, 1, 0, 0, 0, 0]);
    pub const CURRENT: Dim = Dim([0, 0, 0, 1, 0, 0, 0]);
    pub const TEMP: Dim = Dim([0, 0, 0, 0, 1, 0, 0]);
    pub const AMOUNT: Dim = Dim([0, 0, 0, 0, 0, 1, 0]);
    pub const LUMINOUS: Dim = Dim([0, 0, 0, 0, 0, 0, 1]);

    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&x| x == 0)
    }
}

impl Add for Dim {
    type Output = Option<Dim>;
    fn add(self, rhs: Self) -> Self::Output {
        if self == rhs {
            Some(self)
        } else {
            None
        }
    }
}

impl Sub for Dim {
    type Output = Option<Dim>;
    fn sub(self, rhs: Self) -> Self::Output {
        if self == rhs {
            Some(self)
        } else {
            None
        }
    }
}

impl Mul for Dim {
    type Output = Dim;
    // Exponent vectors ADD when quantities multiply (m·s → m^1·s^1) — intentional.
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, rhs: Self) -> Self::Output {
        let mut out = [0; 7];
        for i in 0..7 {
            out[i] = self.0[i] + rhs.0[i];
        }
        Dim(out)
    }
}

impl Div for Dim {
    type Output = Dim;
    // Exponent vectors SUBTRACT when quantities divide (m/s → m^1·s^-1) — intentional.
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        let mut out = [0; 7];
        for i in 0..7 {
            out[i] = self.0[i] - rhs.0[i];
        }
        Dim(out)
    }
}

impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = ["kg", "m", "s", "A", "K", "mol", "cd"];
        let mut parts = Vec::new();
        for i in 0..7 {
            if self.0[i] != 0 {
                let suffix = if self.0[i] == 1 {
                    "".to_string()
                } else {
                    format!("^{}", self.0[i])
                };
                parts.push(format!("{}{}", names[i], suffix));
            }
        }
        if parts.is_empty() {
            write!(f, "")
        } else {
            write!(f, " {}", parts.join("·"))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub value: f64,
    pub dim: Dim,
    pub error: Option<String>,
}

impl Quantity {
    pub fn new(value: f64, dim: Dim) -> Self {
        Self {
            value,
            dim,
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            value: f64::NAN,
            dim: Dim::ZERO,
            error: Some(msg.into()),
        }
    }
}

impl Default for Quantity {
    fn default() -> Self {
        Self {
            value: 0.0,
            dim: Dim::ZERO,
            error: None,
        }
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(e) = &self.error {
            write!(f, "Error: {}", e)
        } else {
            write!(f, "{}{}", self.value, self.dim)
        }
    }
}

impl Add for Quantity {
    type Output = Quantity;
    fn add(self, rhs: Self) -> Self::Output {
        if let Some(e) = self.error {
            return Quantity::err(e);
        }
        if let Some(e) = rhs.error {
            return Quantity::err(e);
        }
        match self.dim + rhs.dim {
            Some(d) => Quantity::new(self.value + rhs.value, d),
            None => Quantity::err(format!(
                "Dimension mismatch: cannot add '{}' and '{}'",
                self.dim, rhs.dim
            )),
        }
    }
}

impl Sub for Quantity {
    type Output = Quantity;
    fn sub(self, rhs: Self) -> Self::Output {
        if let Some(e) = self.error {
            return Quantity::err(e);
        }
        if let Some(e) = rhs.error {
            return Quantity::err(e);
        }
        match self.dim - rhs.dim {
            Some(d) => Quantity::new(self.value - rhs.value, d),
            None => Quantity::err(format!(
                "Dimension mismatch: cannot subtract '{}' and '{}'",
                self.dim, rhs.dim
            )),
        }
    }
}

impl Mul for Quantity {
    type Output = Quantity;
    fn mul(self, rhs: Self) -> Self::Output {
        if let Some(e) = self.error {
            return Quantity::err(e);
        }
        if let Some(e) = rhs.error {
            return Quantity::err(e);
        }
        Quantity::new(self.value * rhs.value, self.dim * rhs.dim)
    }
}

impl Div for Quantity {
    type Output = Quantity;
    fn div(self, rhs: Self) -> Self::Output {
        if let Some(e) = self.error {
            return Quantity::err(e);
        }
        if let Some(e) = rhs.error {
            return Quantity::err(e);
        }
        Quantity::new(self.value / rhs.value, self.dim / rhs.dim)
    }
}

impl FromStr for Quantity {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let val: f64 = s.parse().map_err(|_| format!("invalid number: {}", s))?;
        Ok(Quantity::new(val, Dim::ZERO))
    }
}

/// Functions whose name followed by `(` is application, not juxtaposition.
/// `deriv`/`integ` are expanded later by `expand_calculus`.
const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "ln", "log", "exp",
    "sqrt", "abs", "min", "max", "floor", "ceil", "round", "deriv", "integ",
];

#[derive(Clone)]
enum Tok {
    Num(String),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    Comma,
    Other(char),
}

fn tokenize(input: &str) -> Vec<Tok> {
    let chars: Vec<char> = input.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            toks.push(Tok::Num(chars[start..i].iter().collect()));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            toks.push(Tok::Ident(chars[start..i].iter().collect()));
            continue;
        }
        match c {
            '+' | '-' | '*' | '/' | '^' => toks.push(Tok::Op(c)),
            '(' => toks.push(Tok::LParen),
            ')' => toks.push(Tok::RParen),
            ',' => toks.push(Tok::Comma),
            _ => toks.push(Tok::Other(c)),
        }
        i += 1;
    }
    toks
}

struct Rewriter {
    toks: Vec<Tok>,
    pos: usize,
}

impl Rewriter {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    /// A juxtaposed product continues with a number, identifier or `(` —
    /// never across an explicit operator (so `2 -3` stays subtraction).
    fn chain_continues(&self) -> bool {
        matches!(
            self.peek(),
            Some(Tok::Num(_)) | Some(Tok::Ident(_)) | Some(Tok::LParen)
        )
    }

    /// A unit is an atom (number, identifier, function call, parenthesized
    /// sequence) with an optional `^` suffix — powers bind tighter than
    /// juxtaposition, so `2 x^2` is `2*(x^2)`, never `(2*x)^2`.
    fn unit(&mut self) -> String {
        let mut out = String::new();
        if let Some(Tok::Op(c)) = self.peek() {
            if *c == '+' || *c == '-' {
                let sign = *c;
                self.pos += 1;
                out.push(sign);
                out.push_str(&self.unit());
                return out;
            }
        }
        match self.peek().cloned() {
            Some(Tok::Num(s)) => {
                self.pos += 1;
                out.push_str(&s);
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                out.push_str(&name);
                if FUNCTIONS.contains(&name.as_str()) && matches!(self.peek(), Some(Tok::LParen)) {
                    self.pos += 1;
                    out.push('(');
                    out.push_str(&self.seq());
                    if matches!(self.peek(), Some(Tok::RParen)) {
                        self.pos += 1;
                        out.push(')');
                    }
                }
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                out.push('(');
                out.push_str(&self.seq());
                if matches!(self.peek(), Some(Tok::RParen)) {
                    self.pos += 1;
                    out.push(')');
                }
            }
            _ => return out,
        }
        if matches!(self.peek(), Some(Tok::Op('^'))) {
            self.pos += 1;
            out.push('^');
            out.push_str(&self.unit());
        }
        out
    }

    /// A maximal juxtaposition chain. It is parenthesized only when it is the
    /// right operand of `/` (Mathcad: `5 m / 2 s` = `5*m/(2*s)`); anywhere
    /// else plain `*` insertion is value-equivalent and keeps output minimal.
    fn chain(&mut self, wrap: bool) -> String {
        let first = self.unit();
        if !self.chain_continues() {
            return first;
        }
        let mut parts = vec![first];
        while self.chain_continues() {
            parts.push(self.unit());
        }
        let joined = parts.join("*");
        if wrap {
            format!("({joined})")
        } else {
            joined
        }
    }

    /// Chains separated by explicit operators / commas, within one paren scope.
    fn seq(&mut self) -> String {
        let mut out = self.chain(false);
        loop {
            match self.peek() {
                Some(Tok::Op(c)) if *c != '^' => {
                    let is_div = *c == '/';
                    out.push(*c);
                    self.pos += 1;
                    out.push_str(&self.chain(is_div));
                }
                Some(Tok::Comma) => {
                    out.push(',');
                    self.pos += 1;
                    out.push_str(&self.chain(false));
                }
                Some(Tok::Other(c)) => {
                    out.push(*c);
                    self.pos += 1;
                    out.push_str(&self.chain(false));
                }
                _ => break,
            }
        }
        out
    }
}

/// Normalize an expression for exmex: strip whitespace and make implicit
/// multiplication explicit with **Mathcad precedence** — juxtaposition binds
/// tighter than `*` and `/` (but looser than `^`).
///
/// `5 m / 2 s` becomes `5*m/(2*s)` (= 2.5 m/s), not `5*m/2*s` (= 2.5 m·s).
/// `ms` stays one identifier (milliseconds); `m s` is m·s. Malformed input is
/// passed through for exmex to reject. Idempotent: the output contains no
/// juxtaposition, so re-processing it changes nothing.
pub fn preprocess(input: &str) -> String {
    let mut rw = Rewriter {
        toks: tokenize(input),
        pos: 0,
    };
    let mut out = rw.seq();
    // Pass through anything the rewriter could not place (malformed input —
    // exmex produces the parse error, not us).
    for tok in &rw.toks[rw.pos..] {
        match tok {
            Tok::Num(s) | Tok::Ident(s) => out.push_str(s),
            Tok::Op(c) | Tok::Other(c) => out.push(*c),
            Tok::LParen => out.push('('),
            Tok::RParen => out.push(')'),
            Tok::Comma => out.push(','),
        }
    }
    out
}

pub fn default_bindings() -> Vec<(&'static str, Quantity)> {
    vec![
        ("m", Quantity::new(1.0, Dim::LENGTH)),
        ("s", Quantity::new(1.0, Dim::TIME)),
        ("kg", Quantity::new(1.0, Dim::MASS)),
        ("A", Quantity::new(1.0, Dim::CURRENT)),
        ("K", Quantity::new(1.0, Dim::TEMP)),
        ("mol", Quantity::new(1.0, Dim::AMOUNT)),
        ("cd", Quantity::new(1.0, Dim::LUMINOUS)),
        ("km", Quantity::new(1000.0, Dim::LENGTH)),
        ("cm", Quantity::new(0.01, Dim::LENGTH)),
        ("mm", Quantity::new(0.001, Dim::LENGTH)),
        ("g", Quantity::new(0.001, Dim::MASS)),
        ("ms", Quantity::new(0.001, Dim::TIME)),
        ("us", Quantity::new(1e-6, Dim::TIME)),
        ("ns", Quantity::new(1e-9, Dim::TIME)),
        ("pi", Quantity::new(std::f64::consts::PI, Dim::ZERO)),
        ("e", Quantity::new(std::f64::consts::E, Dim::ZERO)),
    ]
}

pub fn find_func_call(s: &str, name: &str) -> Option<(usize, usize)> {
    let start = s.find(&format!("{}(", name))?;
    let mut depth = 0;
    for (i, c) in s[start..].char_indices() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return Some((start, start + i));
            }
        }
    }
    None
}

pub fn split_top_level(s: &str, delim: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut last = 0;
    for (i, c) in s.char_indices() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
        } else if c == delim && depth == 0 {
            parts.push(&s[last..i]);
            last = i + 1;
        }
    }
    parts.push(&s[last..]);
    parts
}

pub fn evaluate_single_expr(expr: &str, bindings: &HashMap<String, Quantity>) -> Option<Quantity> {
    let expanded = expand_calculus(expr, bindings).ok()?;
    let parsed: FlatEx<Quantity, QuantityOpsFactory> = FlatEx::parse(&expanded).ok()?;
    let var_names = parsed.var_names();
    let mut vars = Vec::new();
    for name in var_names {
        let q = bindings.get(name)?;
        vars.push(q.clone());
    }
    parsed.eval(&vars).ok()
}

pub fn numerical_derivative(
    expr: &str,
    var: &str,
    at: f64,
    bindings: &HashMap<String, Quantity>,
) -> Quantity {
    let h = 1e-5;
    let eval_at = |x: f64| -> f64 {
        let mut local_bindings = bindings.clone();
        local_bindings.insert(var.to_string(), Quantity::new(x, Dim::ZERO));
        evaluate_single_expr(expr, &local_bindings)
            .map(|q| q.value)
            .unwrap_or(f64::NAN)
    };

    let f_plus = eval_at(at + h);
    let f_minus = eval_at(at - h);
    let deriv_value = (f_plus - f_minus) / (2.0 * h);

    let mut local_bindings = bindings.clone();
    local_bindings.insert(var.to_string(), Quantity::new(1.0, Dim::ZERO));
    let inner_dim = evaluate_single_expr(expr, &local_bindings)
        .map(|q| q.dim)
        .unwrap_or(Dim::ZERO);

    Quantity::new(deriv_value, inner_dim)
}

pub fn numerical_integral(
    expr: &str,
    var: &str,
    a: f64,
    b: f64,
    bindings: &HashMap<String, Quantity>,
) -> Quantity {
    let n = 1000;
    let h = (b - a) / n as f64;
    let eval_at = |x: f64| -> f64 {
        let mut local_bindings = bindings.clone();
        local_bindings.insert(var.to_string(), Quantity::new(x, Dim::ZERO));
        evaluate_single_expr(expr, &local_bindings)
            .map(|q| q.value)
            .unwrap_or(f64::NAN)
    };

    let mut sum = eval_at(a) + eval_at(b);
    for i in 1..n {
        let x = a + i as f64 * h;
        let val = eval_at(x);
        if i % 2 == 0 {
            sum += 2.0 * val;
        } else {
            sum += 4.0 * val;
        }
    }
    let integral_value = (h / 3.0) * sum;

    let mut local_bindings = bindings.clone();
    local_bindings.insert(var.to_string(), Quantity::new(1.0, Dim::ZERO));
    let inner_dim = evaluate_single_expr(expr, &local_bindings)
        .map(|q| q.dim)
        .unwrap_or(Dim::ZERO);

    Quantity::new(integral_value, inner_dim)
}

fn format_f64(val: f64) -> String {
    let s = format!("{:.10}", val);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

pub fn expand_calculus(
    input: &str,
    bindings: &HashMap<String, Quantity>,
) -> Result<String, String> {
    let mut processed = preprocess(input);

    loop {
        let mut changed = false;

        if let Some((start, end)) = find_func_call(&processed, "deriv") {
            let inner = &processed[start + 6..end];
            let parts = split_top_level(inner, ',');
            if parts.len() == 2 {
                let expr_str = parts[0].trim();
                let var = parts[1].trim();
                let at = bindings.get(var).map(|q| q.value).unwrap_or(0.0);
                let val = numerical_derivative(expr_str, var, at, bindings);
                processed.replace_range(start..=end, &format!("({})", format_f64(val.value)));
                changed = true;
            } else {
                return Err("deriv requires (expr, var)".to_string());
            }
        } else if let Some((start, end)) = find_func_call(&processed, "integ") {
            let inner = &processed[start + 6..end];
            let parts = split_top_level(inner, ',');
            if parts.len() == 4 {
                let expr_str = parts[0].trim();
                let var = parts[1].trim();
                let a = evaluate_single_expr(parts[2].trim(), bindings)
                    .map(|q| q.value)
                    .ok_or_else(|| "Invalid integral lower bound".to_string())?;
                let b = evaluate_single_expr(parts[3].trim(), bindings)
                    .map(|q| q.value)
                    .ok_or_else(|| "Invalid integral upper bound".to_string())?;
                let val = numerical_integral(expr_str, var, a, b, bindings);
                processed.replace_range(start..=end, &format!("({})", format_f64(val.value)));
                changed = true;
            } else {
                return Err("integ requires (expr, var, a, b)".to_string());
            }
        }

        if !changed {
            break;
        }
    }

    Ok(processed)
}

#[derive(Clone, Debug)]
pub struct QuantityOpsFactory;

impl MakeOperators<Quantity> for QuantityOpsFactory {
    fn make<'a>() -> Vec<Operator<'a, Quantity>> {
        vec![
            Operator::make_bin_unary(
                "+",
                BinOp {
                    apply: |a, b| a + b,
                    prio: 0,
                    is_commutative: true,
                },
                |a| a,
            ),
            Operator::make_bin_unary(
                "-",
                BinOp {
                    apply: |a, b| a - b,
                    prio: 0,
                    is_commutative: false,
                },
                |a| Quantity::new(-a.value, a.dim),
            ),
            Operator::make_bin(
                "*",
                BinOp {
                    apply: |a, b| a * b,
                    prio: 1,
                    is_commutative: true,
                },
            ),
            Operator::make_bin(
                "/",
                BinOp {
                    apply: |a, b| a / b,
                    prio: 1,
                    is_commutative: false,
                },
            ),
            Operator::make_bin(
                "^",
                BinOp {
                    apply: |a, b| {
                        if !b.dim.is_zero() {
                            return Quantity::err("Exponent must be dimensionless");
                        }
                        if b.value.fract() != 0.0 {
                            return Quantity::err("Exponent must be an integer");
                        }
                        let exp = b.value as i8;
                        let mut new_dim = a.dim;
                        for d in new_dim.0.iter_mut() {
                            *d *= exp;
                        }
                        Quantity::new(a.value.powf(b.value), new_dim)
                    },
                    prio: 2,
                    is_commutative: false,
                },
            ),
            Operator::make_unary("sin", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.sin(), Dim::ZERO)
                }
            }),
            Operator::make_unary("cos", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.cos(), Dim::ZERO)
                }
            }),
            Operator::make_unary("tan", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.tan(), Dim::ZERO)
                }
            }),
            Operator::make_unary("asin", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.asin(), Dim::ZERO)
                }
            }),
            Operator::make_unary("acos", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.acos(), Dim::ZERO)
                }
            }),
            Operator::make_unary("atan", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.atan(), Dim::ZERO)
                }
            }),
            Operator::make_unary("sinh", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.sinh(), Dim::ZERO)
                }
            }),
            Operator::make_unary("cosh", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.cosh(), Dim::ZERO)
                }
            }),
            Operator::make_unary("tanh", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.tanh(), Dim::ZERO)
                }
            }),
            Operator::make_unary("ln", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.ln(), Dim::ZERO)
                }
            }),
            Operator::make_unary("log", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.log10(), Dim::ZERO)
                }
            }),
            Operator::make_unary("exp", |a| {
                if !a.dim.is_zero() {
                    Quantity::err("Argument must be dimensionless")
                } else {
                    Quantity::new(a.value.exp(), Dim::ZERO)
                }
            }),
            Operator::make_unary("abs", |a| Quantity::new(a.value.abs(), a.dim)),
            Operator::make_unary("floor", |a| Quantity::new(a.value.floor(), a.dim)),
            Operator::make_unary("ceil", |a| Quantity::new(a.value.ceil(), a.dim)),
            Operator::make_unary("round", |a| Quantity::new(a.value.round(), a.dim)),
            Operator::make_unary("sqrt", |a| {
                let mut new_dim = a.dim;
                for i in 0..7 {
                    if a.dim.0[i] % 2 != 0 {
                        return Quantity::err("Cannot take sqrt of a dimension with odd exponent");
                    }
                    new_dim.0[i] /= 2;
                }
                Quantity::new(a.value.sqrt(), new_dim)
            }),
            Operator::make_bin(
                "min",
                BinOp {
                    apply: |a, b| {
                        if let Some(e) = a.error {
                            return Quantity::err(e);
                        }
                        if let Some(e) = b.error {
                            return Quantity::err(e);
                        }
                        if a.dim == b.dim {
                            Quantity::new(a.value.min(b.value), a.dim)
                        } else {
                            Quantity::err("min requires matching dimensions")
                        }
                    },
                    prio: 3,
                    is_commutative: true,
                },
            ),
            Operator::make_bin(
                "max",
                BinOp {
                    apply: |a, b| {
                        if let Some(e) = a.error {
                            return Quantity::err(e);
                        }
                        if let Some(e) = b.error {
                            return Quantity::err(e);
                        }
                        if a.dim == b.dim {
                            Quantity::new(a.value.max(b.value), a.dim)
                        } else {
                            Quantity::err("max requires matching dimensions")
                        }
                    },
                    prio: 3,
                    is_commutative: true,
                },
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(v: f64) -> Quantity {
        Quantity::new(v, Dim::ZERO)
    }

    fn eval_str(s: &str) -> Quantity {
        eval_with(s, &[])
    }

    fn eval_with(s: &str, extra: &[(&str, Quantity)]) -> Quantity {
        let mut binds = HashMap::new();
        for (k, v) in default_bindings() {
            binds.insert(k.to_string(), v);
        }
        for (k, v) in extra {
            binds.insert((*k).to_string(), v.clone());
        }
        let processed = expand_calculus(s, &binds).unwrap();
        let expr: FlatEx<Quantity, QuantityOpsFactory> = FlatEx::parse(&processed).unwrap();
        let var_names = expr.var_names();
        let mut vars = Vec::new();
        for name in var_names {
            if let Some(q) = binds.get(name) {
                vars.push(q.clone());
            } else {
                panic!("Unknown variable in test: {}", name);
            }
        }
        expr.eval(&vars).unwrap()
    }

    #[test]
    fn dim_math() {
        assert_eq!(Dim::LENGTH * Dim::TIME, Dim([0, 1, 1, 0, 0, 0, 0]));
        assert_eq!(Dim::LENGTH / Dim::TIME, Dim([0, 1, -1, 0, 0, 0, 0]));
        assert_eq!(Dim::LENGTH + Dim::LENGTH, Some(Dim::LENGTH));
        assert_eq!(Dim::LENGTH + Dim::TIME, None);
    }

    #[test]
    fn preprocess_spaces() {
        assert_eq!(preprocess("5 m"), "5*m");
        assert_eq!(preprocess("5 * m"), "5*m");
        assert_eq!(preprocess("2(x)"), "2*(x)");
        assert_eq!(preprocess("2.5"), "2.5");
        assert_eq!(preprocess("(a)2"), "(a)*2");
        assert_eq!(preprocess("m s"), "m*s");
        assert_eq!(preprocess("m/s"), "m/s");
        assert_eq!(preprocess("sin(x)"), "sin(x)");
        assert_eq!(preprocess("sin (x)"), "sin(x)");
        assert_eq!(preprocess("2 sin(x)"), "2*sin(x)");
        assert_eq!(preprocess("min(a, b)"), "min(a,b)");
    }

    #[test]
    fn preprocess_mathcad_juxtaposition() {
        // Juxtaposition binds tighter than '/' — denominators get grouped.
        assert_eq!(preprocess("5 m / 2 s"), "5*m/(2*s)");
        assert_eq!(preprocess("1 / 2 x"), "1/(2*x)");
        assert_eq!(preprocess("2 x / 3 y"), "2*x/(3*y)");
        assert_eq!(preprocess("6 kg m / 2 s^2"), "6*kg*m/(2*s^2)");
        assert_eq!(preprocess("2 (a+b) / 3 c"), "2*(a+b)/(3*c)");
        assert_eq!(preprocess("a / b c / d e"), "a/(b*c)/(d*e)");
        // Powers bind tighter than juxtaposition.
        assert_eq!(preprocess("2 m^2"), "2*m^2");
        assert_eq!(preprocess("x^2 y"), "x^2*y");
        assert_eq!(preprocess("2^3 m"), "2^3*m");
        // Everything else keeps the minimal, left-to-right form.
        assert_eq!(preprocess("kg m / s^2"), "kg*m/s^2");
        assert_eq!(preprocess("5 * m / 2 * s"), "5*m/2*s");
        assert_eq!(preprocess("a b c"), "a*b*c");
        assert_eq!(preprocess("(a+b)(c+d)"), "(a+b)*(c+d)");
        assert_eq!(preprocess("sin(x) cos(x)"), "sin(x)*cos(x)");
        assert_eq!(preprocess("-2 m"), "-2*m");
        assert_eq!(preprocess("5 m / (2 s)"), "5*m/(2*s)");
    }

    #[test]
    fn preprocess_idempotent() {
        for s in [
            "5 m / 2 s",
            "1 / 2 x",
            "2 x / 3 y",
            "6 kg m / 2 s^2",
            "2 m^2",
            "min(2 m, 3 m)",
            "deriv(x^2, x)",
            "5*m/(2*s)",
            "(a+b)(c+d)",
            "2 sin(x)",
        ] {
            let once = preprocess(s);
            assert_eq!(preprocess(&once), once, "not idempotent for {s:?}");
        }
    }

    #[test]
    fn preprocess_malformed_passthrough() {
        // preprocess never panics; exmex produces the parse error downstream.
        assert_eq!(preprocess("2 +"), "2+");
        assert_eq!(preprocess("(a"), "(a");
        assert_eq!(preprocess("a)"), "a)");
        assert_eq!(preprocess(""), "");
        assert_eq!(preprocess("()"), "()");
    }

    #[test]
    fn preprocess_function_and_variable_spacing() {
        // deriv/integ tolerate a space before '(' (previously broken).
        assert_eq!(preprocess("deriv (x^2, x)"), "deriv(x^2,x)");
        assert_eq!(preprocess("integ (x^2, x, 0, 3)"), "integ(x^2,x,0,3)");
        // Non-function identifier + parens is implicit multiplication,
        // with or without a space (previously inconsistent).
        assert_eq!(preprocess("x(y)"), "x*(y)");
        assert_eq!(preprocess("x (y)"), "x*(y)");
    }

    #[test]
    fn preprocess_scientific_notation_is_not_special() {
        // Existing behavior preserved: `e` is Euler's constant, so `1e-5`
        // means 1*e - 5. Scientific-notation literals are a later step.
        assert_eq!(preprocess("1e-5"), "1*e-5");
    }

    #[test]
    fn eval_mathcad_velocity() {
        let v = eval_str("5 m / 2 s");
        assert_eq!(v.value, 2.5);
        assert_eq!(v.dim, Dim::LENGTH / Dim::TIME);
        // Parenthesized denominators give the same answer as before.
        let p = eval_str("5 m / (2 s)");
        assert_eq!(p.value, 2.5);
        assert_eq!(p.dim, Dim::LENGTH / Dim::TIME);
    }

    #[test]
    fn eval_one_over_juxtaposition() {
        let r = eval_str("1 / 2 m");
        assert_eq!(r.value, 0.5);
        assert_eq!(r.dim, Dim([0, -1, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn eval_chained_division() {
        let r = eval_str("10 m / 2 s / 5");
        assert_eq!(r.value, 1.0);
        assert_eq!(r.dim, Dim::LENGTH / Dim::TIME);
        let n = eval_str("100 / 10 / 2");
        assert_eq!(n.value, 5.0);
        assert!(n.dim.is_zero());
    }

    #[test]
    fn eval_powers_bind_tighter_than_juxtaposition() {
        let a = eval_str("2 m^2");
        assert_eq!(a.value, 2.0);
        assert_eq!(a.dim, Dim::LENGTH * Dim::LENGTH);

        let b = eval_str("m^2 s");
        assert_eq!(b.value, 1.0);
        assert_eq!(b.dim, Dim::LENGTH * Dim::LENGTH * Dim::TIME);

        let c = eval_str("2^3 m");
        assert_eq!(c.value, 8.0);
        assert_eq!(c.dim, Dim::LENGTH);
    }

    #[test]
    fn eval_force_dimensions() {
        let f = eval_str("6 kg m / 2 s^2");
        assert_eq!(f.value, 3.0);
        assert_eq!(f.dim, Dim::MASS * Dim::LENGTH / (Dim::TIME * Dim::TIME));
    }

    #[test]
    fn eval_juxtaposition_with_variables() {
        let r = eval_with("2 x / 3 y", &[("x", q(6.0)), ("y", q(2.0))]);
        assert_eq!(r.value, 2.0);
        assert!(r.dim.is_zero());

        let s = eval_with("x^2 y", &[("x", q(3.0)), ("y", q(2.0))]);
        assert_eq!(s.value, 18.0);
    }

    #[test]
    fn eval_parenthesised_groups_juxtapose() {
        let r = eval_with("(a+b) c", &[("a", q(1.0)), ("b", q(2.0)), ("c", q(3.0))]);
        assert_eq!(r.value, 9.0);

        let s = eval_with(
            "2 (a+b) / 3 c",
            &[("a", q(1.0)), ("b", q(2.0)), ("c", q(1.0))],
        );
        assert_eq!(s.value, 2.0);
    }

    #[test]
    fn eval_ms_is_one_unit_m_s_is_two() {
        let ms = eval_str("5 ms");
        assert_eq!(ms.value, 0.005);
        assert_eq!(ms.dim, Dim::TIME);

        let m_s = eval_str("5 m s");
        assert_eq!(m_s.value, 5.0);
        assert_eq!(m_s.dim, Dim::LENGTH * Dim::TIME);
    }

    #[test]
    fn eval_unary_signs_with_juxtaposition() {
        let a = eval_str("-2 m");
        assert_eq!(a.value, -2.0);
        assert_eq!(a.dim, Dim::LENGTH);

        let b = eval_with("2 -x", &[("x", q(5.0))]);
        assert_eq!(b.value, -3.0);

        let c = eval_with("-x y", &[("x", q(2.0)), ("y", q(3.0))]);
        assert_eq!(c.value, -6.0);
    }

    #[test]
    fn eval_explicit_operators_keep_left_to_right() {
        // Explicit '*' and '/' are untouched by the Mathcad rule.
        let r = eval_str("5 * m / 2 * s");
        assert_eq!(r.value, 2.5);
        assert_eq!(r.dim, Dim::LENGTH * Dim::TIME);
    }

    #[test]
    fn expand_calculus_inside_juxtaposition() {
        let res = expand_calculus("2 deriv(x^2, x)", &HashMap::new()).unwrap();
        assert_eq!(res, "2*(0)");
    }

    #[test]
    fn quantity_add_mismatch() {
        let a = Quantity::new(2.0, Dim::LENGTH);
        let b = Quantity::new(3.0, Dim::TIME);
        let sum = a + b;
        assert!(sum.error.is_some());
        assert!(sum.error.unwrap().contains("Dimension mismatch"));
    }

    #[test]
    fn quantity_mul() {
        let a = Quantity::new(2.0, Dim::LENGTH);
        let b = Quantity::new(3.0, Dim::TIME);
        let prod = a * b;
        assert!(prod.error.is_none());
        assert_eq!(prod.value, 6.0);
        assert_eq!(prod.dim, Dim::LENGTH * Dim::TIME);
    }

    #[test]
    fn math_dimensionless() {
        let tan = eval_str("tan(pi / 4)");
        assert!((tan.value - 1.0).abs() < 1e-9);
        assert!(tan.dim.is_zero());

        let ln = eval_str("ln(e)");
        assert!((ln.value - 1.0).abs() < 1e-9);
        assert!(ln.dim.is_zero());
    }

    #[test]
    fn math_preserves_dimensions() {
        let abs = eval_str("abs(-5 m)");
        assert_eq!(abs.value, 5.0);
        assert_eq!(abs.dim, Dim::LENGTH);
    }

    #[test]
    fn math_min_max() {
        let min = eval_str("min(2 m, 3 m)");
        assert_eq!(min.value, 2.0);
        assert_eq!(min.dim, Dim::LENGTH);

        let max = eval_str("max(2 m, 3 m)");
        assert_eq!(max.value, 3.0);
        assert_eq!(max.dim, Dim::LENGTH);
    }

    #[test]
    fn math_min_dimension_mismatch() {
        let min = eval_str("min(2 m, 3 s)");
        assert!(min.error.is_some());
    }

    #[test]
    fn test_expand_calculus_deriv() {
        let res = expand_calculus("deriv(x^2, x)", &HashMap::new()).unwrap();
        assert_eq!(res, "(0)");
    }

    #[test]
    fn test_expand_calculus_integ() {
        let res = expand_calculus("integ(x^2, x, 0, 3)", &HashMap::new()).unwrap();
        assert_eq!(res, "(9)");
    }
}
