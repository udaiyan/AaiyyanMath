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

const TIMEOUT: Duration = Duration::from_secs(30);

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

#[derive(Default)]
pub struct PythonRunner {
    rx: Option<Receiver<RunOutcome>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    pub running: bool,
    pub last: Option<RunOutcome>,
}

impl PythonRunner {
    /// Snapshot the worksheet to a script and execute it on a background thread.
    pub fn start(&mut self, ws: &Worksheet) {
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

        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.rx = Some(rx);
        self.cancel_flag = Some(cancel.clone());
        self.running = true;

        thread::spawn(move || {
            let outcome = run_script(&script, &dir, &cancel);
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

fn run_script(script: &Path, dir: &Path, cancel: &Arc<AtomicBool>) -> RunOutcome {
    let start = Instant::now();
    let mut outcome = failed(dir.to_path_buf(), String::new());
    outcome.problem = None;

    let python = match find_python() {
        Some(p) => p,
        None => {
            outcome.problem = Some(
                "Python not found. Install Python 3, then: pip install numpy pint matplotlib"
                    .into(),
            );
            outcome.duration = start.elapsed();
            return outcome;
        }
    };

    let mut child = match Command::new(python)
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
            outcome.problem = Some(format!("Failed to start {python}: {e}"));
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
                if start.elapsed() > TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    outcome.problem = Some(format!("Timed out after {} s", TIMEOUT.as_secs()));
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
