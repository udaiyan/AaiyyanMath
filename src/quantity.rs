use std::fmt;
use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;

use exmex::{BinOp, MakeOperators, Operator};
use serde::{Serialize, Deserialize};

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
        if self == rhs { Some(self) } else { None }
    }
}

impl Sub for Dim {
    type Output = Option<Dim>;
    fn sub(self, rhs: Self) -> Self::Output {
        if self == rhs { Some(self) } else { None }
    }
}

impl Mul for Dim {
    type Output = Dim;
    fn mul(self, rhs: Self) -> Self::Output {
        let mut out = [0; 7];
        for i in 0..7 { out[i] = self.0[i] + rhs.0[i]; }
        Dim(out)
    }
}

impl Div for Dim {
    type Output = Dim;
    fn div(self, rhs: Self) -> Self::Output {
        let mut out = [0; 7];
        for i in 0..7 { out[i] = self.0[i] - rhs.0[i]; }
        Dim(out)
    }
}

impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = ["kg", "m", "s", "A", "K", "mol", "cd"];
        let mut parts = Vec::new();
        for i in 0..7 {
            if self.0[i] != 0 {
                let suffix = if self.0[i] == 1 { "".to_string() } else { format!("^{}", self.0[i]) };
                parts.push(format!("{}{}", names[i], suffix));
            }
        }
        if parts.is_empty() { write!(f, "") } else { write!(f, " {}", parts.join("·")) }
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
        Self { value, dim, error: None }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self { value: f64::NAN, dim: Dim::ZERO, error: Some(msg.into()) }
    }
}

impl Default for Quantity {
    fn default() -> Self {
        Self { value: 0.0, dim: Dim::ZERO, error: None }
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
        if let Some(e) = self.error { return Quantity::err(e); }
        if let Some(e) = rhs.error { return Quantity::err(e); }
        match self.dim + rhs.dim {
            Some(d) => Quantity::new(self.value + rhs.value, d),
            None => Quantity::err(format!("Dimension mismatch: cannot add '{}' and '{}'", self.dim, rhs.dim)),
        }
    }
}

impl Sub for Quantity {
    type Output = Quantity;
    fn sub(self, rhs: Self) -> Self::Output {
        if let Some(e) = self.error { return Quantity::err(e); }
        if let Some(e) = rhs.error { return Quantity::err(e); }
        match self.dim - rhs.dim {
            Some(d) => Quantity::new(self.value - rhs.value, d),
            None => Quantity::err(format!("Dimension mismatch: cannot subtract '{}' and '{}'", self.dim, rhs.dim)),
        }
    }
}

impl Mul for Quantity {
    type Output = Quantity;
    fn mul(self, rhs: Self) -> Self::Output {
        if let Some(e) = self.error { return Quantity::err(e); }
        if let Some(e) = rhs.error { return Quantity::err(e); }
        Quantity::new(self.value * rhs.value, self.dim * rhs.dim)
    }
}

impl Div for Quantity {
    type Output = Quantity;
    fn div(self, rhs: Self) -> Self::Output {
        if let Some(e) = self.error { return Quantity::err(e); }
        if let Some(e) = rhs.error { return Quantity::err(e); }
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

pub fn preprocess(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    let mut prev: Option<char> = None;
    let mut prev_was_space = false;
    let mut prev_word = String::new();

    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            prev_was_space = true;
            continue;
        }

        if let Some(p) = prev {
            let p_is_alnum = p.is_ascii_digit() || p.is_alphabetic() || p == '_' || p == ')';
            let c_is_alnum = c.is_ascii_digit() || c.is_alphabetic() || c == '_' || c == '(';

            let mut insert = false;

            if prev_was_space {
                if p_is_alnum && c_is_alnum {
                    if c == '(' {
                        if p.is_ascii_digit() || p == ')' {
                            insert = true;
                        } else if p.is_alphabetic() {
                            let is_func = ["sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "ln", "log", "exp", "sqrt", "abs", "min", "max", "floor", "ceil", "round"].contains(&prev_word.as_str());
                            if !is_func {
                                insert = true;
                            }
                        }
                    } else {
                        insert = true;
                    }
                }
            } else {
                if (p.is_ascii_digit() || p == ')') && (c.is_alphabetic() || c == '(' || c == '_') {
                    insert = true;
                } else if p == ')' && c.is_ascii_digit() {
                    insert = true;
                }
            }

            if insert {
                out.push('*');
            }
        }

        out.push(c);
        prev = Some(c);

        if prev_was_space {
            prev_word.clear();
        }

        if c.is_alphabetic() {
            prev_word.push(c);
        } else {
            prev_word.clear();
        }

        prev_was_space = false;
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

#[derive(Clone, Debug)]
pub struct QuantityOpsFactory;

impl MakeOperators<Quantity> for QuantityOpsFactory {
    fn make<'a>() -> Vec<Operator<'a, Quantity>> {
        vec![
            Operator::make_bin_unary(
                "+",
                BinOp { apply: |a, b| a + b, prio: 0, is_commutative: true },
                |a| a,
            ),
            Operator::make_bin_unary(
                "-",
                BinOp { apply: |a, b| a - b, prio: 0, is_commutative: false },
                |a| Quantity::new(-a.value, a.dim),
            ),
            Operator::make_bin(
                "*",
                BinOp { apply: |a, b| a * b, prio: 1, is_commutative: true },
            ),
            Operator::make_bin(
                "/",
                BinOp { apply: |a, b| a / b, prio: 1, is_commutative: false },
            ),
            Operator::make_bin(
                "^",
                BinOp {
                    apply: |a, b| {
                        if !b.dim.is_zero() { return Quantity::err("Exponent must be dimensionless"); }
                        if b.value.fract() != 0.0 { return Quantity::err("Exponent must be an integer"); }
                        let exp = b.value as i8;
                        let mut new_dim = a.dim;
                        for i in 0..7 { new_dim.0[i] *= exp; }
                        Quantity::new(a.value.powf(b.value), new_dim)
                    },
                    prio: 2,
                    is_commutative: false,
                },
            ),
            Operator::make_unary("sin", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.sin(), Dim::ZERO) } }),
            Operator::make_unary("cos", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.cos(), Dim::ZERO) } }),
            Operator::make_unary("tan", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.tan(), Dim::ZERO) } }),
            Operator::make_unary("asin", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.asin(), Dim::ZERO) } }),
            Operator::make_unary("acos", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.acos(), Dim::ZERO) } }),
            Operator::make_unary("atan", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.atan(), Dim::ZERO) } }),
            Operator::make_unary("sinh", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.sinh(), Dim::ZERO) } }),
            Operator::make_unary("cosh", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.cosh(), Dim::ZERO) } }),
            Operator::make_unary("tanh", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.tanh(), Dim::ZERO) } }),
            Operator::make_unary("ln", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.ln(), Dim::ZERO) } }),
            Operator::make_unary("log", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.log10(), Dim::ZERO) } }),
            Operator::make_unary("exp", |a| { if !a.dim.is_zero() { Quantity::err("Argument must be dimensionless") } else { Quantity::new(a.value.exp(), Dim::ZERO) } }),
            Operator::make_unary("abs", |a| Quantity::new(a.value.abs(), a.dim)),
            Operator::make_unary("floor", |a| Quantity::new(a.value.floor(), a.dim)),
            Operator::make_unary("ceil", |a| Quantity::new(a.value.ceil(), a.dim)),
            Operator::make_unary("round", |a| Quantity::new(a.value.round(), a.dim)),
            Operator::make_unary("sqrt", |a| {
                let mut new_dim = a.dim;
                for i in 0..7 {
                    if a.dim.0[i] % 2 != 0 { return Quantity::err("Cannot take sqrt of a dimension with odd exponent"); }
                    new_dim.0[i] /= 2;
                }
                Quantity::new(a.value.sqrt(), new_dim)
            }),
            Operator::make_bin(
                "min",
                BinOp {
                    apply: |a, b| {
                        if let Some(e) = a.error { return Quantity::err(e); }
                        if let Some(e) = b.error { return Quantity::err(e); }
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
                        if let Some(e) = a.error { return Quantity::err(e); }
                        if let Some(e) = b.error { return Quantity::err(e); }
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
    use exmex::{Express, FlatEx};

    fn eval_str(s: &str) -> Quantity {
        let processed = preprocess(s);
        let expr: FlatEx<Quantity, QuantityOpsFactory> = FlatEx::parse(&processed).unwrap();
        let var_names = expr.var_names();
        let mut vars = Vec::new();
        for name in var_names {
            if let Some((_, q)) = default_bindings().into_iter().find(|(n, _)| n == &name) {
                vars.push(q);
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
}