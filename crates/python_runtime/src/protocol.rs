use serde::{Deserialize, Serialize};

pub type JobId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeKind {
    /// Produced by the Rust code generator from the worksheet AST.
    Generated,
    /// Written by the user; execution requires explicit trust.
    UserTrusted,
}

#[derive(Clone, Debug)]
pub enum Request {
    Execute { id: JobId, code: String, kind: CodeKind },
    Reset { id: JobId },
    Shutdown,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValueDto {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub latex: Option<String>,
    #[serde(default)]
    pub number: Option<f64>,
    #[serde(default)]
    pub si: Option<f64>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub shape: Vec<usize>,
    #[serde(default)]
    pub data: Vec<f64>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExecOutcome {
    pub ok: bool,
    #[serde(default)]
    pub values: Vec<ValueDto>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub traceback: Option<String>,
    #[serde(default)]
    pub duration_ms: f64,
    #[serde(default)]
    pub cancelled: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PyInfo {
    pub available: bool,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub executable: String,
    #[serde(default)]
    pub libraries: Vec<(String, String)>,
    #[serde(default)]
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum Response {
    Info(PyInfo),
    Executed { id: JobId, outcome: ExecOutcome },
    ResetDone { id: JobId },
    Error { id: JobId, message: String },
    Stopped,
}
