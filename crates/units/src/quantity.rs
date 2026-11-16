use crate::dimension::Dimension;
use crate::latex_number::format_number;
use crate::registry::{simplify, Unit};
use crate::UnitError;
use serde::{Deserialize, Serialize};

/// A physical quantity stored as an SI base value plus a display unit.
/// This representation crosses the Rust/Python boundary unchanged (spec 10).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub si: f64,
    pub unit: Unit,
}

impl Quantity {
    pub fn new(value: f64, unit: Unit) -> Quantity {
        Quantity { si: value * unit.factor + unit.offset, unit }
    }

    pub fn from_si(si: f64, dim: Dimension) -> Quantity {
        Quantity { si, unit: simplify(dim) }
    }

    pub fn dimensionless(value: f64) -> Quantity {
        Quantity { si: value, unit: Unit::dimensionless() }
    }

    pub fn value(&self) -> f64 {
        if self.unit.factor == 0.0 { return self.si; }
        (self.si - self.unit.offset) / self.unit.factor
    }

    pub fn dim(&self) -> Dimension { self.unit.dim }

    pub fn is_dimensionless(&self) -> bool { self.unit.dim.is_dimensionless() }

    pub fn convert(&self, target: &Unit) -> Result<Quantity, UnitError> {
        if !self.dim().compatible(&target.dim) {
            return Err(UnitError::DimensionMismatch(format!(
                "cannot convert {} to {}", self.dim().quantity_name(), target.dim.quantity_name()
            )));
        }
        Ok(Quantity { si: self.si, unit: target.clone() })
    }

    fn guard_affine(&self, other: &Quantity, op: &str) -> Result<(), UnitError> {
        if self.unit.is_affine() || other.unit.is_affine() {
            return Err(UnitError::AffineOperation(op.to_string()));
        }
        Ok(())
    }

    pub fn mul(&self, other: &Quantity) -> Result<Quantity, UnitError> {
        self.guard_affine(other, "multiplication")?;
        Ok(Quantity::from_si(self.si * other.si, self.dim() * other.dim()))
    }

    pub fn div(&self, other: &Quantity) -> Result<Quantity, UnitError> {
        self.guard_affine(other, "division")?;
        if other.si == 0.0 {
            return Err(UnitError::InvalidExpression("division by zero quantity".into()));
        }
        Ok(Quantity::from_si(self.si / other.si, self.dim() / other.dim()))
    }

    pub fn scale(&self, k: f64) -> Result<Quantity, UnitError> {
        if self.unit.is_affine() {
            return Err(UnitError::AffineOperation("scaling".into()));
        }
        Ok(Quantity { si: self.si * k, unit: self.unit.clone() })
    }

    pub fn powf(&self, exp: f64) -> Result<Quantity, UnitError> {
        if self.unit.is_affine() {
            return Err(UnitError::AffineOperation("exponentiation".into()));
        }
        let dim = self.dim().scale(exp);
        let unit = if (exp - 1.0).abs() < 1e-12 { self.unit.clone() } else { simplify(dim) };
        Ok(Quantity { si: self.si.powf(exp), unit })
    }

    pub fn mul_unit(&self, unit: &Unit) -> Result<Quantity, UnitError> {
        self.guard_affine(&Quantity::new(1.0, unit.clone()), "multiplication")?;
        Ok(Quantity { si: self.si * unit.factor, unit: self.unit.mul_unit(unit) })
    }

    pub fn div_unit(&self, unit: &Unit) -> Result<Quantity, UnitError> {
        self.guard_affine(&Quantity::new(1.0, unit.clone()), "division")?;
        if unit.factor == 0.0 {
            return Err(UnitError::InvalidExpression("division by zero unit".into()));
        }
        Ok(Quantity { si: self.si / unit.factor, unit: self.unit.div_unit(unit) })
    }

    pub fn add(&self, other: &Quantity) -> Result<Quantity, UnitError> {
        if !self.dim().compatible(&other.dim()) {
            return Err(UnitError::DimensionMismatch(format!(
                "{} + {}", self.dim().quantity_name(), other.dim().quantity_name()
            )));
        }
        Ok(Quantity { si: self.si + other.si, unit: self.unit.clone() })
    }

    pub fn sub(&self, other: &Quantity) -> Result<Quantity, UnitError> {
        if !self.dim().compatible(&other.dim()) {
            return Err(UnitError::DimensionMismatch(format!(
                "{} - {}", self.dim().quantity_name(), other.dim().quantity_name()
            )));
        }
        Ok(Quantity { si: self.si - other.si, unit: self.unit.clone() })
    }

    pub fn display(&self) -> String {
        let v = format_number(self.value());
        if self.unit.name.is_empty() { v } else { format!("{v} {}", self.unit.name) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::lookup;

    #[test]
    fn velocity_from_length_and_time() {
        let l = Quantity::new(5.0, lookup("m").unwrap());
        let t = Quantity::new(2.0, lookup("s").unwrap());
        let v = l.div(&t).unwrap();
        assert!((v.value() - 2.5).abs() < 1e-12);
        assert_eq!(v.unit.name, "m/s");
    }

    #[test]
    fn temperature_conversion() {
        let c = Quantity::new(100.0, lookup("degC").unwrap());
        assert!((c.si - 373.15).abs() < 1e-9);
        let f = c.convert(&lookup("degF").unwrap()).unwrap();
        assert!((f.value() - 212.0).abs() < 1e-6);
    }

    #[test]
    fn dimensional_mismatch_is_rejected() {
        let l = Quantity::new(1.0, lookup("m").unwrap());
        let t = Quantity::new(1.0, lookup("s").unwrap());
        assert!(l.add(&t).is_err());
    }
}
