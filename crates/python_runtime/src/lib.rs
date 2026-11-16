//! Embedded Python runtime (spec section 8).
//!
//! * One interpreter, initialised once, owned by a dedicated worker thread.
//! * Message passing only; the GUI thread never touches the GIL.
//! * Cancellation is cooperative (`sys.settrace` + Rust atomic). Long C-level
//!   NumPy calls cannot be interrupted; this is documented, not hidden.
//! * Embedded Python is NOT a sandbox. User code requires explicit trust.
pub mod protocol;
pub mod worker;

pub use protocol::{CodeKind, ExecOutcome, JobId, PyInfo, Request, Response, ValueDto};
pub use worker::{PythonHandle, RuntimeConfig};

/// Runtime support module: single source of truth shared with exported scripts.
pub const RUNTIME_SOURCE: &str = include_str!("../../../python/scripts/aaiyyan_runtime.py");
