//! Native Rust evaluation engine. The GUI never evaluates mathematics itself;
//! it talks to `aaiyyan-core`, which schedules work on this engine.
pub mod builtins;
pub mod builtins_ext;
pub mod functions;
pub mod env;
pub mod interp;
pub mod linalg;
pub mod value;

pub use env::Environment;
pub use interp::{EvalLimits, Evaluator};
pub use value::{Cell, Function, FunctionBody, Matrix, Table, Value};

use aaiyyan_units::UnitError;

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("undefined symbol '{0}'")]
    Undefined(String),
    #[error("unknown function '{0}'")]
    UnknownFunction(String),
    #[error("dimension error: {0}")]
    Dimension(String),
    #[error("type error: {0}")]
    Type(String),
    #[error("argument error: {0}")]
    Args(String),
    #[error("math error: {0}")]
    Math(String),
    #[error("index error: {0}")]
    Index(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("requires the Python backend: {0}")]
    RequiresPython(String),
    #[error("calculation cancelled")]
    Cancelled,
    #[error("calculation time limit exceeded")]
    Timeout,
    #[error("calculation step limit exceeded")]
    StepLimit,
    #[error(transparent)]
    Unit(#[from] UnitError),
}
