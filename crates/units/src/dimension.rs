use serde::{Deserialize, Serialize};
use std::ops::{Div, Mul};

pub const BASE_SYMBOLS: [&str; 7] = ["m", "kg", "s", "A", "K", "mol", "cd"];
pub const BASE_NAMES: [&str; 7] = [
    "length", "mass", "time", "electric current", "temperature", "amount of substance", "luminous intensity",
];
const EPS: f64 = 1e-9;

/// Physical dimension as exponents of the seven SI base dimensions.
/// Rational exponents (roots) are supported; comparison uses an epsilon.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dimension(pub [f64; 7]);

impl Dimension {
    pub const DIMENSIONLESS: Dimension = Dimension([0.0; 7]);

    pub fn base(index: usize) -> Dimension {
        let mut e = [0.0; 7];
        e[index] = 1.0;
        Dimension(e)
    }

    pub fn length() -> Dimension { Dimension::base(0) }
    pub fn mass() -> Dimension { Dimension::base(1) }
    pub fn time() -> Dimension { Dimension::base(2) }
    pub fn current() -> Dimension { Dimension::base(3) }
    pub fn temperature() -> Dimension { Dimension::base(4) }
    pub fn amount() -> Dimension { Dimension::base(5) }
    pub fn luminosity() -> Dimension { Dimension::base(6) }

    pub fn area() -> Dimension { Dimension::length().scale(2.0) }
    pub fn volume() -> Dimension { Dimension::length().scale(3.0) }
    pub fn velocity() -> Dimension { Dimension::length() / Dimension::time() }
    pub fn acceleration() -> Dimension { Dimension::velocity() / Dimension::time() }
    pub fn force() -> Dimension { Dimension::mass() * Dimension::acceleration() }
    pub fn energy() -> Dimension { Dimension::force() * Dimension::length() }
    pub fn power() -> Dimension { Dimension::energy() / Dimension::time() }
    pub fn pressure() -> Dimension { Dimension::force() / Dimension::area() }
    pub fn frequency() -> Dimension { Dimension([0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0]) }
    pub fn charge() -> Dimension { Dimension::current() * Dimension::time() }
    pub fn voltage() -> Dimension { Dimension::power() / Dimension::current() }
    pub fn resistance() -> Dimension { Dimension::voltage() / Dimension::current() }

    pub fn scale(&self, k: f64) -> Dimension {
        let mut e = self.0;
        for v in e.iter_mut() { *v *= k; }
        Dimension(e)
    }

    pub fn is_dimensionless(&self) -> bool { self.0.iter().all(|e| e.abs() < EPS) }

    pub fn compatible(&self, other: &Dimension) -> bool {
        self.0.iter().zip(other.0.iter()).all(|(a, b)| (a - b).abs() < EPS)
    }

    pub fn to_unit_string(&self) -> String {
        let mut num: Vec<String> = Vec::new();
        let mut den: Vec<String> = Vec::new();
        for (i, e) in self.0.iter().enumerate() {
            if e.abs() < EPS { continue; }
            let term = fmt_term(BASE_SYMBOLS[i], e.abs());
            if *e > 0.0 { num.push(term); } else { den.push(term); }
        }
        let n = if num.is_empty() { "1".to_string() } else { num.join("*") };
        if den.is_empty() {
            n
        } else if den.len() == 1 {
            format!("{n}/{}", den[0])
        } else {
            format!("{n}/({})", den.join("*"))
        }
    }

    pub fn quantity_name(&self) -> String {
        let table: [(Dimension, &str); 16] = [
            (Dimension::DIMENSIONLESS, "dimensionless"),
            (Dimension::length(), "length"),
            (Dimension::area(), "area"),
            (Dimension::volume(), "volume"),
            (Dimension::mass(), "mass"),
            (Dimension::time(), "time"),
            (Dimension::velocity(), "velocity"),
            (Dimension::acceleration(), "acceleration"),
            (Dimension::force(), "force"),
            (Dimension::energy(), "energy"),
            (Dimension::power(), "power"),
            (Dimension::pressure(), "pressure"),
            (Dimension::frequency(), "frequency"),
            (Dimension::charge(), "electric charge"),
            (Dimension::voltage(), "voltage"),
            (Dimension::temperature(), "temperature"),
        ];
        for (d, name) in table.iter() {
            if self.compatible(d) { return name.to_string(); }
        }
        self.to_unit_string()
    }
}

fn fmt_term(symbol: &str, exp: f64) -> String {
    if (exp - 1.0).abs() < EPS {
        symbol.to_string()
    } else if (exp - exp.round()).abs() < EPS {
        format!("{symbol}^{}", exp.round() as i32)
    } else {
        format!("{symbol}^{exp:.3}")
    }
}

impl Mul for Dimension {
    type Output = Dimension;
    fn mul(self, o: Dimension) -> Dimension {
        let mut e = self.0;
        for i in 0..7 { e[i] += o.0[i]; }
        Dimension(e)
    }
}

impl Div for Dimension {
    type Output = Dimension;
    fn div(self, o: Dimension) -> Dimension {
        let mut e = self.0;
        for i in 0..7 { e[i] -= o.0[i]; }
        Dimension(e)
    }
}
