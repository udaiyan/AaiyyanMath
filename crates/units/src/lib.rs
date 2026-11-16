//! Native Rust engineering unit engine (spec section 10).
pub mod dimension;
pub mod latex_number;
pub mod parse;
pub mod quantity;
pub mod registry;

pub use dimension::{Dimension, BASE_NAMES, BASE_SYMBOLS};
pub use latex_number::format_number;
pub use parse::parse_unit_string;
pub use quantity::Quantity;
pub use registry::{all_units, coherent, is_unit_name, lookup, simplify, Unit};

#[derive(Debug, thiserror::Error)]
pub enum UnitError {
    #[error("unknown unit '{0}'")]
    UnknownUnit(String),
    #[error("dimension mismatch: {0}")]
    DimensionMismatch(String),
    #[error("affine temperature units cannot be used in '{0}'")]
    AffineOperation(String),
    #[error("invalid unit expression: {0}")]
    InvalidExpression(String),
}
