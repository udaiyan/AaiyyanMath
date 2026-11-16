use crate::dimension::Dimension;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::sync::OnceLock;

/// A unit is described by its dimension, a multiplicative factor to SI base
/// units and an optional affine offset (temperatures).
/// si_value = value * factor + offset
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    pub name: String,
    pub dim: Dimension,
    pub factor: f64,
    pub offset: f64,
}

impl Unit {
    pub fn dimensionless() -> Unit {
        Unit { name: String::new(), dim: Dimension::DIMENSIONLESS, factor: 1.0, offset: 0.0 }
    }

    pub fn is_affine(&self) -> bool { self.offset.abs() > 1e-12 }

    pub fn mul_unit(&self, other: &Unit) -> Unit {
        Unit {
            name: format!("{}*{}", self.name, other.name),
            dim: self.dim * other.dim,
            factor: self.factor * other.factor,
            offset: 0.0,
        }
    }

    pub fn div_unit(&self, other: &Unit) -> Unit {
        let rhs = if other.name.contains('*') || other.name.contains('/') {
            format!("({})", other.name)
        } else {
            other.name.clone()
        };
        Unit {
            name: format!("{}/{}", self.name, rhs),
            dim: self.dim / other.dim,
            factor: self.factor / other.factor,
            offset: 0.0,
        }
    }

    pub fn pow_unit(&self, exp: f64) -> Unit {
        let name = if (exp - 1.0).abs() < 1e-12 {
            self.name.clone()
        } else if (exp - exp.round()).abs() < 1e-9 {
            format!("{}^{}", self.name, exp.round() as i32)
        } else {
            format!("{}^{}", self.name, exp)
        };
        Unit { name, dim: self.dim.scale(exp), factor: self.factor.powf(exp), offset: 0.0 }
    }
}

struct Entry {
    name: &'static str,
    dim: Dimension,
    factor: f64,
    offset: f64,
    prefixable: bool,
}

fn e(name: &'static str, dim: Dimension, factor: f64, prefixable: bool) -> Entry {
    Entry { name, dim, factor, offset: 0.0, prefixable }
}

fn build() -> Vec<Entry> {
    let l = Dimension::length();
    let m = Dimension::mass();
    let t = Dimension::time();
    let i = Dimension::current();
    let th = Dimension::temperature();
    let n = Dimension::amount();
    let j = Dimension::luminosity();
    let force = Dimension::force();
    let energy = Dimension::energy();
    let power = Dimension::power();
    let pressure = Dimension::pressure();
    let dimless = Dimension::DIMENSIONLESS;

    vec![
        e("m", l, 1.0, true),
        e("g", m, 1e-3, true),
        e("kg", m, 1.0, false),
        e("s", t, 1.0, true),
        e("A", i, 1.0, true),
        e("K", th, 1.0, true),
        e("mol", n, 1.0, true),
        e("cd", j, 1.0, true),
        e("Hz", Dimension::frequency(), 1.0, true),
        e("N", force, 1.0, true),
        e("Pa", pressure, 1.0, true),
        e("J", energy, 1.0, true),
        e("W", power, 1.0, true),
        e("C", Dimension::charge(), 1.0, true),
        e("V", Dimension::voltage(), 1.0, true),
        e("ohm", Dimension::resistance(), 1.0, true),
        e("S", Dimension::current() / Dimension::voltage(), 1.0, true),
        e("F", Dimension::charge() / Dimension::voltage(), 1.0, true),
        e("H", Dimension::voltage() * Dimension::time() / Dimension::current(), 1.0, true),
        e("T", Dimension::mass() / (Dimension::current() * Dimension::time().scale(2.0)), 1.0, true),
        e("Wb", Dimension::voltage() * Dimension::time(), 1.0, true),
        e("lm", j, 1.0, true),
        e("lx", j / Dimension::area(), 1.0, true),
        e("eV", energy, 1.602176634e-19, true),
        e("bar", pressure, 1e5, true),
        e("L", Dimension::volume(), 1e-3, true),
        e("rad", dimless, 1.0, false),
        e("sr", dimless, 1.0, false),
        e("deg", dimless, PI / 180.0, false),
        e("rev", dimless, 2.0 * PI, false),
        e("percent", dimless, 0.01, false),
        e("ppm", dimless, 1e-6, false),
        e("min", t, 60.0, false),
        e("h", t, 3600.0, false),
        e("day", t, 86400.0, false),
        e("week", t, 604800.0, false),
        e("yr", t, 31557600.0, false),
        e("t", m, 1000.0, false),
        e("lb", m, 0.45359237, false),
        e("oz", m, 0.028349523125, false),
        e("slug", m, 14.593902937, false),
        e("in", l, 0.0254, false),
        e("ft", l, 0.3048, false),
        e("yd", l, 0.9144, false),
        e("mi", l, 1609.344, false),
        e("nmi", l, 1852.0, false),
        e("angstrom", l, 1e-10, false),
        e("acre", Dimension::area(), 4046.8564224, false),
        e("gal", Dimension::volume(), 3.785411784e-3, false),
        e("qt", Dimension::volume(), 9.46352946e-4, false),
        e("pt", Dimension::volume(), 4.73176473e-4, false),
        e("floz", Dimension::volume(), 2.95735295625e-5, false),
        e("atm", pressure, 101325.0, false),
        e("psi", pressure, 6894.757293168, false),
        e("torr", pressure, 133.322368421, false),
        e("mmHg", pressure, 133.322387415, false),
        e("cal", energy, 4.184, false),
        e("kcal", energy, 4184.0, false),
        e("Wh", energy, 3600.0, true),
        e("kWh", energy, 3.6e6, false),
        e("BTU", energy, 1055.05585262, false),
        e("hp", power, 745.699871582, false),
        e("kn", Dimension::velocity(), 0.514444444444, false),
        Entry { name: "degC", dim: th, factor: 1.0, offset: 273.15, prefixable: false },
        Entry { name: "degF", dim: th, factor: 5.0 / 9.0, offset: 459.67 * 5.0 / 9.0, prefixable: false },
        Entry { name: "R", dim: th, factor: 5.0 / 9.0, offset: 0.0, prefixable: false },
        e("meter", l, 1.0, false),
        e("metre", l, 1.0, false),
        e("gram", m, 1e-3, false),
        e("kilogram", m, 1.0, false),
        e("second", t, 1.0, false),
        e("newton", force, 1.0, false),
        e("pascal", pressure, 1.0, false),
        e("joule", energy, 1.0, false),
        e("watt", power, 1.0, false),
        e("volt", Dimension::voltage(), 1.0, false),
        e("amp", i, 1.0, false),
        e("kelvin", th, 1.0, false),
        e("litre", Dimension::volume(), 1e-3, false),
        e("liter", Dimension::volume(), 1e-3, false),
        e("radian", dimless, 1.0, false),
        e("degree", dimless, PI / 180.0, false),
        e("\u{03A9}", Dimension::resistance(), 1.0, false),
    ]
}

fn entries() -> &'static Vec<Entry> {
    static E: OnceLock<Vec<Entry>> = OnceLock::new();
    E.get_or_init(build)
}

const PREFIXES: &[(&str, f64)] = &[
    ("da", 1e1), ("Y", 1e24), ("Z", 1e21), ("E", 1e18), ("P", 1e15), ("T", 1e12), ("G", 1e9),
    ("M", 1e6), ("k", 1e3), ("h", 1e2), ("d", 1e-1), ("c", 1e-2), ("m", 1e-3), ("u", 1e-6),
    ("\u{00B5}", 1e-6), ("\u{03BC}", 1e-6), ("n", 1e-9), ("p", 1e-12), ("f", 1e-15),
    ("a", 1e-18), ("z", 1e-21), ("y", 1e-24),
];

pub fn lookup(name: &str) -> Option<Unit> {
    if name.is_empty() {
        return Some(Unit::dimensionless());
    }
    for entry in entries() {
        if entry.name == name {
            return Some(Unit {
                name: name.to_string(),
                dim: entry.dim,
                factor: entry.factor,
                offset: entry.offset,
            });
        }
    }
    for (prefix, scale) in PREFIXES {
        if let Some(rest) = name.strip_prefix(prefix) {
            if rest.is_empty() { continue; }
            for entry in entries() {
                if entry.prefixable && entry.name == rest {
                    return Some(Unit {
                        name: name.to_string(),
                        dim: entry.dim,
                        factor: entry.factor * scale,
                        offset: 0.0,
                    });
                }
            }
        }
    }
    None
}

pub fn is_unit_name(name: &str) -> bool { lookup(name).is_some() }

pub fn all_units() -> Vec<Unit> {
    entries().iter()
        .map(|x| Unit { name: x.name.to_string(), dim: x.dim, factor: x.factor, offset: x.offset })
        .collect()
}

/// Find a named coherent SI unit for a dimension (factor 1, no offset).
pub fn coherent(dim: Dimension) -> Option<Unit> {
    const PRIORITY: &[&str] = &[
        "m", "kg", "s", "A", "K", "mol", "cd", "N", "Pa", "J", "W", "C", "V", "ohm", "S", "F",
        "H", "T", "Wb", "Hz", "lm", "lx", "rad",
    ];
    for name in PRIORITY {
        if let Some(u) = lookup(name) {
            if (u.factor - 1.0).abs() < 1e-12 && !u.is_affine() && u.dim.compatible(&dim) {
                return Some(u);
            }
        }
    }
    None
}

pub fn simplify(dim: Dimension) -> Unit {
    if dim.is_dimensionless() {
        return Unit::dimensionless();
    }
    coherent(dim).unwrap_or_else(|| Unit {
        name: dim.to_unit_string(),
        dim,
        factor: 1.0,
        offset: 0.0,
    })
}
