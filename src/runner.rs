use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::python::generate_python;
use crate::worksheet::Worksheet;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct RunOutcome {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub plot_dir: PathBuf,
    /// Spawn failure, timeout, or cancellation (as opposed to a Python traceback).
    pub problem: Option<String>,
}

pub struct PythonRunner {
    rx: Option<Receiver<RunOutcome>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    pub running: bool,
    pub last: Option<RunOutcome>,
    /// How long a run may take before it is killed.
    pub timeout: Duration,
}

impl Default for PythonRunner {
    fn default() -> Self {
        Self {
            rx: None,
            cancel_flag: None,
            running: false,
            last: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl PythonRunner {
    /// Snapshot the worksheet to a script and execute it on a background thread.
    /// A non-empty `interpreter` overrides Python auto-detection.
    pub fn start(&mut self, ws: &Worksheet, interpreter: &str) {
        if self.running {
            return;
        }

        // Previous run's files are no longer needed (textures are already loaded).
        if let Some(prev) = &self.last {
            let _ = fs::remove_dir_all(&prev.plot_dir);
        }

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("aaiyyanmath_run_{nanos}"));

        if let Err(e) = fs::create_dir_all(&dir)
            .and_then(|_| fs::write(dir.join("script.py"), generate_python(ws)))
        {
            self.last = Some(failed(dir, format!("Cannot write script: {e}")));
            return;
        }
        let script = dir.join("script.py");

        let python = match resolve_interpreter(interpreter) {
            Some(p) => p,
            None => {
                self.last = Some(failed(
                    dir,
                    "Python not found. Install Python 3, then: pip install numpy pint matplotlib"
                        .into(),
                ));
                return;
            }
        };

        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.rx = Some(rx);
        self.cancel_flag = Some(cancel.clone());
        self.running = true;
        let timeout = self.timeout;

        thread::spawn(move || {
            let outcome = run_script(&script, &dir, &cancel, timeout, &python);
            let _ = tx.send(outcome);
        });
    }

    pub fn cancel(&self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::SeqCst);
        }
    }

    /// Poll from the UI thread. Returns true when a new outcome has arrived.
    pub fn poll(&mut self) -> bool {
        if let Some(rx) = &self.rx {
            if let Ok(outcome) = rx.try_recv() {
                self.last = Some(outcome);
                self.running = false;
                self.rx = None;
                self.cancel_flag = None;
                return true;
            }
        }
        false
    }
}

fn failed(dir: PathBuf, msg: String) -> RunOutcome {
    RunOutcome {
        stdout: String::new(),
        stderr: String::new(),
        success: false,
        exit_code: None,
        duration: Duration::ZERO,
        plot_dir: dir,
        problem: Some(msg),
    }
}

fn find_python() -> Option<&'static str> {
    let candidates: &[&str] = if cfg!(windows) {
        &["python", "py"]
    } else {
        &["python3", "python"]
    };
    for c in candidates {
        if let Ok(out) = Command::new(c)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            if out.status.success() {
                return Some(c);
            }
        }
    }
    None
}

/// A trimmed, non-empty override wins; otherwise auto-detect Python.
fn resolve_interpreter(override_path: &str) -> Option<String> {
    let trimmed = override_path.trim();
    if !trimmed.is_empty() {
        return Some(trimmed.to_string());
    }
    find_python().map(|s| s.to_string())
}

/// Suggest an install command when stderr shows a missing Python package.
/// Requires both "ModuleNotFoundError" and a quoted "No module named 'X'"
/// (single or double quotes); reports the top-level package.
pub fn missing_module_hint(stderr: &str) -> Option<String> {
    if !stderr.contains("ModuleNotFoundError") {
        return None;
    }
    const MARKER: &str = "No module named ";
    let after = &stderr[stderr.find(MARKER)? + MARKER.len()..];
    let quote = after.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let close = after[1..].find(quote)?;
    let module = &after[1..1 + close];
    if module.is_empty() {
        return None;
    }
    let top = module.split('.').next().unwrap_or(module);
    Some(format!(
        "Missing Python package '{top}'. Install the worksheet dependencies: pip install numpy pint matplotlib"
    ))
}

fn run_script(
    script: &Path,
    dir: &Path,
    cancel: &Arc<AtomicBool>,
    timeout: Duration,
    interpreter: &str,
) -> RunOutcome {
    let start = Instant::now();
    let mut outcome = failed(dir.to_path_buf(), String::new());
    outcome.problem = None;

    let mut child = match Command::new(interpreter)
        .arg("-u") // unbuffered output
        .arg("-X")
        .arg("utf8") // avoid Windows cp1252 encoding errors on pint output
        .arg(script)
        .current_dir(dir) // relative savefig paths land in the run dir
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            outcome.problem = Some(format!("Failed to start {interpreter}: {e}"));
            outcome.duration = start.elapsed();
            return outcome;
        }
    };

    // Drain both pipes on separate threads so a chatty script can't deadlock
    // by filling the OS pipe buffer while we wait.
    let mut stdout_pipe = child.stdout.take().expect("piped stdout");
    let mut stderr_pipe = child.stderr.take().expect("piped stderr");
    let t_out = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout_pipe.read_to_end(&mut buf);
        buf
    });
    let t_err = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buf);
        buf
    });

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if cancel.load(Ordering::SeqCst) {
                    let _ = child.kill();
                    let _ = child.wait();
                    outcome.problem = Some("Cancelled".into());
                    break None;
                }
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    outcome.problem = Some(format!("Timed out after {timeout:?}"));
                    break None;
                }
                thread::sleep(Duration::from_millis(30));
            }
            Err(e) => {
                outcome.problem = Some(format!("Wait failed: {e}"));
                break None;
            }
        }
    };

    outcome.stdout = String::from_utf8_lossy(&t_out.join().unwrap_or_default()).into_owned();
    outcome.stderr = String::from_utf8_lossy(&t_err.join().unwrap_or_default()).into_owned();
    if let Some(status) = status {
        outcome.exit_code = status.code();
        outcome.success = status.success() && outcome.problem.is_none();
    }
    outcome.duration = start.elapsed();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::Expression;
    use crate::region::Region;
    use std::sync::atomic::AtomicU64;

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    /// Unique run dir under the OS temp dir — never the repo cwd.
    fn test_dir(tag: &str) -> PathBuf {
        let seq = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "aaiyyanmath_runner_test_{}_{}_{}",
            tag,
            std::process::id(),
            seq
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_script(dir: &Path, code: &str) -> PathBuf {
        let script = dir.join("script.py");
        fs::write(&script, code).unwrap();
        script
    }

    fn no_cancel() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn problem(outcome: &RunOutcome) -> &str {
        outcome.problem.as_deref().unwrap_or("")
    }

    /// The full worksheet pipeline needs the generated script's imports.
    fn has_worksheet_deps(py: &str) -> bool {
        Command::new(py)
            .arg("-c")
            .arg("import numpy, pint, matplotlib")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn wait_for_finish(runner: &mut PythonRunner) {
        let deadline = Instant::now() + Duration::from_secs(60);
        while Instant::now() < deadline {
            if runner.poll() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("runner did not finish within 60 s");
    }

    // ---- pure / ungated tests ----

    #[test]
    fn default_runner_has_30s_timeout() {
        let runner = PythonRunner::default();
        assert_eq!(runner.timeout, Duration::from_secs(30));
        assert!(!runner.running);
        assert!(runner.last.is_none());
    }

    #[test]
    fn resolve_interpreter_trims_and_prefers_override() {
        assert_eq!(
            resolve_interpreter("  /custom/python  ").as_deref(),
            Some("/custom/python")
        );
        assert_eq!(
            resolve_interpreter("my_python").as_deref(),
            Some("my_python")
        );
    }

    #[test]
    fn resolve_interpreter_blank_falls_back_to_auto() {
        let auto = find_python().map(|s| s.to_string());
        assert_eq!(resolve_interpreter(""), auto);
        assert_eq!(resolve_interpreter("   \t "), auto);
    }

    #[test]
    fn bogus_interpreter_reports_failed_to_start() {
        let dir = test_dir("bogus");
        let script = write_script(&dir, "print('hi')\n");
        let outcome = run_script(
            &script,
            &dir,
            &no_cancel(),
            Duration::from_secs(10),
            "aaiyyanmath_no_such_python_xyz",
        );
        assert!(!outcome.success);
        assert!(
            problem(&outcome).contains("Failed to start"),
            "got: {:?}",
            outcome.problem
        );
        assert!(outcome.stdout.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hint_single_quotes() {
        let hint =
            missing_module_hint("ModuleNotFoundError: No module named 'numpy'").expect("hint");
        assert!(hint.contains("numpy"));
        assert!(hint.contains("pip install numpy pint matplotlib"));
    }

    #[test]
    fn hint_double_quotes() {
        let hint =
            missing_module_hint("ModuleNotFoundError: No module named \"pint\"").expect("hint");
        assert!(hint.contains("pint"));
        assert!(hint.contains("pip install numpy pint matplotlib"));
    }

    #[test]
    fn hint_dotted_module_reports_top_level() {
        let hint = missing_module_hint("ModuleNotFoundError: No module named 'matplotlib.pyplot'")
            .expect("hint");
        assert!(hint.contains("'matplotlib'"));
        assert!(!hint.contains("matplotlib.pyplot"));
    }

    #[test]
    fn hint_full_traceback() {
        let stderr = "Traceback (most recent call last):\n  File \"script.py\", line 3, in <module>\n    import numpy\nModuleNotFoundError: No module named 'numpy'\n";
        let hint = missing_module_hint(stderr).expect("hint");
        assert!(hint.contains("numpy"));
        assert!(hint.contains("pip install numpy pint matplotlib"));
    }

    #[test]
    fn hint_requires_modulenotfounderror() {
        assert!(missing_module_hint("ImportError: No module named 'numpy'").is_none());
        assert!(missing_module_hint("No module named 'numpy'").is_none());
    }

    #[test]
    fn hint_requires_quoted_marker() {
        assert!(missing_module_hint("ModuleNotFoundError: something else").is_none());
        assert!(missing_module_hint("ModuleNotFoundError: No module named numpy").is_none());
    }

    #[test]
    fn hint_empty_module_is_ignored() {
        assert!(missing_module_hint("ModuleNotFoundError: No module named ''").is_none());
    }

    #[test]
    fn hint_unterminated_quote_is_ignored() {
        assert!(missing_module_hint("ModuleNotFoundError: No module named 'numpy").is_none());
    }

    #[test]
    fn hint_empty_and_unrelated_stderr() {
        assert!(missing_module_hint("").is_none());
        assert!(missing_module_hint("SyntaxError: invalid syntax").is_none());
        assert!(missing_module_hint("a = 4\n").is_none());
    }

    // ---- subprocess tests (skip with a note when Python is absent) ----

    #[test]
    fn stdout_and_exit_zero_captured() {
        let Some(py) = find_python() else {
            eprintln!("skipping: Python not found");
            return;
        };
        let dir = test_dir("stdout");
        let script = write_script(&dir, "print('hello from python')\n");
        let outcome = run_script(&script, &dir, &no_cancel(), Duration::from_secs(30), py);
        assert!(outcome.success, "problem: {:?}", outcome.problem);
        assert_eq!(outcome.exit_code, Some(0));
        assert!(outcome.stdout.contains("hello from python"));
        assert!(outcome.stderr.is_empty());
        assert!(outcome.problem.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn nonzero_exit_and_stderr_captured() {
        let Some(py) = find_python() else {
            eprintln!("skipping: Python not found");
            return;
        };
        let dir = test_dir("exit3");
        let script = write_script(
            &dir,
            "import sys\nsys.stderr.write('boom\\n')\nsys.exit(3)\n",
        );
        let outcome = run_script(&script, &dir, &no_cancel(), Duration::from_secs(30), py);
        assert!(!outcome.success);
        assert_eq!(outcome.exit_code, Some(3));
        assert!(outcome.stderr.contains("boom"));
        assert!(
            outcome.problem.is_none(),
            "a Python traceback is not a runner problem"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_module_stderr_produces_hint() {
        let Some(py) = find_python() else {
            eprintln!("skipping: Python not found");
            return;
        };
        let dir = test_dir("missing_mod");
        let script = write_script(&dir, "import aaiyyanmath_definitely_not_installed_xyz\n");
        let outcome = run_script(&script, &dir, &no_cancel(), Duration::from_secs(30), py);
        assert!(!outcome.success);
        assert!(outcome.stderr.contains("ModuleNotFoundError"));
        let hint = missing_module_hint(&outcome.stderr).expect("hint from real stderr");
        assert!(hint.contains("aaiyyanmath_definitely_not_installed_xyz"));
        assert!(hint.contains("pip install numpy pint matplotlib"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn short_timeout_kills_runaway_script() {
        let Some(py) = find_python() else {
            eprintln!("skipping: Python not found");
            return;
        };
        let dir = test_dir("timeout");
        let script = write_script(&dir, "while True:\n    pass\n");
        let outcome = run_script(&script, &dir, &no_cancel(), Duration::from_millis(300), py);
        assert!(!outcome.success);
        assert!(
            problem(&outcome).contains("Timed out"),
            "got: {:?}",
            outcome.problem
        );
        assert!(outcome.duration < Duration::from_secs(10));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancel_flag_stops_script() {
        let Some(py) = find_python() else {
            eprintln!("skipping: Python not found");
            return;
        };
        let dir = test_dir("cancel");
        let script = write_script(&dir, "import time\nwhile True:\n    time.sleep(0.05)\n");
        let cancel = no_cancel();
        let flag = cancel.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            flag.store(true, Ordering::SeqCst);
        });
        let outcome = run_script(&script, &dir, &cancel, Duration::from_secs(30), py);
        assert!(!outcome.success);
        assert!(
            problem(&outcome).contains("Cancelled"),
            "got: {:?}",
            outcome.problem
        );
        assert!(outcome.duration < Duration::from_secs(10));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn start_poll_pipeline() {
        let Some(py) = find_python() else {
            eprintln!("skipping: Python not found");
            return;
        };
        if !has_worksheet_deps(py) {
            eprintln!("skipping: numpy/pint/matplotlib not installed");
            return;
        }

        let mut ws = Worksheet::new("Pipeline");
        ws.regions = vec![Region::Math(Expression::new("a", "2 + 2"))];
        ws.evaluate_all();

        // Run 1: auto-detection (empty override).
        let mut runner = PythonRunner::default();
        runner.start(&ws, "");
        assert!(runner.running);
        wait_for_finish(&mut runner);
        let last = runner.last.as_ref().expect("outcome");
        assert!(
            last.success,
            "problem: {:?}\nstderr: {}",
            last.problem, last.stderr
        );
        assert!(last.stdout.contains("a = 4"), "stdout: {}", last.stdout);

        // Run 2: explicit interpreter override through the same pipeline.
        runner.start(&ws, py);
        wait_for_finish(&mut runner);
        let last = runner.last.as_ref().expect("outcome");
        assert!(
            last.success,
            "problem: {:?}\nstderr: {}",
            last.problem, last.stderr
        );
        assert!(last.stdout.contains("a = 4"), "stdout: {}", last.stdout);

        let _ = fs::remove_dir_all(&last.plot_dir);
    }
}
