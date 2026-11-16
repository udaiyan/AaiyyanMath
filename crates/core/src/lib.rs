//! Application engine: orchestrates parsing, dependency analysis, native
//! evaluation, Python execution, results and exports. No UI code here.
pub mod convert;
pub mod results;
pub mod scheduler;
pub mod session;

pub use results::{BlockResult, CalcReport, LogEntry, LogLevel, VariableInfo};
pub use session::Session;

/// Re-exports so front ends and tests can build blocks without extra deps.
pub use aaiyyan_plotting as plotting;
pub use aaiyyan_project as project;
pub use aaiyyan_worksheet as worksheet;
