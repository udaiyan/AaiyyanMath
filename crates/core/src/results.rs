use aaiyyan_evaluator::Value;

#[derive(Clone, Debug)]
pub struct BlockResult {
    pub name: String,
    pub value: Value,
    pub display: String,
    pub latex: String,
    pub source: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel { Info, Warning, Error }

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub time: String,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> LogEntry {
        LogEntry { level, message: message.into(), time: timestamp() }
    }
}

fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}", now.as_secs() % 86400, now.subsec_millis())
}

#[derive(Clone, Debug, Default)]
pub struct CalcReport {
    pub duration_ms: f64,
    pub evaluated: Vec<String>,
    pub errors: usize,
    pub backend: String,
    pub cancelled: bool,
}

#[derive(Clone, Debug)]
pub struct VariableInfo {
    pub name: String,
    pub kind: String,
    pub display: String,
    pub unit: Option<String>,
    pub latex: String,
}
