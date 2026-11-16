use crate::protocol::{CodeKind, ExecOutcome, JobId, PyInfo, Request, Response};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

static CANCEL: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Default)]
pub struct RuntimeConfig {
    pub extra_sys_paths: Vec<PathBuf>,
    pub allow_user_code: bool,
}

pub struct PythonHandle {
    tx: Sender<Request>,
    rx: Receiver<Response>,
    next_id: JobId,
    info: PyInfo,
    pub config: RuntimeConfig,
}

impl PythonHandle {
    pub fn spawn(config: RuntimeConfig) -> PythonHandle {
        let (req_tx, req_rx) = channel::<Request>();
        let (resp_tx, resp_rx) = channel::<Response>();
        let cfg = config.clone();
        std::thread::Builder::new()
            .name("aaiyyan-python".into())
            .spawn(move || worker_main(cfg, req_rx, resp_tx))
            .expect("failed to spawn Python worker thread");
        PythonHandle { tx: req_tx, rx: resp_rx, next_id: 1, info: PyInfo::default(), config }
    }

    pub fn info(&self) -> &PyInfo { &self.info }
    pub fn available(&self) -> bool { self.info.available }

    pub fn execute(&mut self, code: String, kind: CodeKind) -> JobId {
        let id = self.next_id;
        self.next_id += 1;
        CANCEL.store(false, Ordering::SeqCst);
        let _ = self.tx.send(Request::Execute { id, code, kind });
        id
    }

    pub fn reset(&mut self) -> JobId {
        let id = self.next_id;
        self.next_id += 1;
        let _ = self.tx.send(Request::Reset { id });
        id
    }

    pub fn cancel(&self) { CANCEL.store(true, Ordering::SeqCst); }

    pub fn poll(&mut self) -> Vec<Response> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(Response::Info(info)) => { self.info = info; }
                Ok(r) => out.push(r),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }

    pub fn shutdown(&self) { let _ = self.tx.send(Request::Shutdown); }
}

#[cfg(not(feature = "python"))]
fn worker_main(_cfg: RuntimeConfig, rx: Receiver<Request>, tx: Sender<Response>) {
    let info = PyInfo {
        available: false,
        message: "Built without the 'python' feature. Rebuild with --features python to enable the embedded backend.".into(),
        ..Default::default()
    };
    let _ = tx.send(Response::Info(info));
    while let Ok(req) = rx.recv() {
        match req {
            Request::Execute { id, .. } => {
                let _ = tx.send(Response::Executed {
                    id,
                    outcome: ExecOutcome {
                        ok: false,
                        error: Some("embedded Python backend is not compiled in".into()),
                        ..Default::default()
                    },
                });
            }
            Request::Reset { id } => { let _ = tx.send(Response::ResetDone { id }); }
            Request::Shutdown => break,
        }
    }
    let _ = tx.send(Response::Stopped);
}

#[cfg(feature = "python")]
fn worker_main(cfg: RuntimeConfig, rx: Receiver<Request>, tx: Sender<Response>) {
    use pyo3::prelude::*;

    pyo3::prepare_freethreaded_python();

    let setup: PyResult<(Py<PyModule>, PyInfo)> = Python::with_gil(|py| {
        let sys = py.import_bound("sys")?;
        let path = sys.getattr("path")?;
        for extra in &cfg.extra_sys_paths {
            path.call_method1("insert", (0, extra.to_string_lossy().to_string()))?;
        }
        let module = PyModule::from_code_bound(
            py,
            crate::RUNTIME_SOURCE,
            "aaiyyan_runtime.py",
            "aaiyyan_runtime",
        )?;
        module.add_function(pyo3::wrap_pyfunction_bound!(cancel_requested, module)?)?;
        sys.getattr("modules")?.set_item("aaiyyan_runtime", &module)?;
        let raw: String = module.getattr("info")?.call0()?.extract()?;
        let parsed: RawInfo = serde_json::from_str(&raw).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("invalid runtime info: {e}"))
        })?;
        let info = PyInfo {
            available: true,
            version: parsed.version,
            executable: parsed.executable,
            libraries: parsed.libraries,
            message: "embedded Python ready".into(),
        };
        Ok((module.unbind(), info))
    });

    let (module, info) = match setup {
        Ok(v) => v,
        Err(e) => {
            let _ = tx.send(Response::Info(PyInfo {
                available: false,
                message: format!("Python initialisation failed: {e}"),
                ..Default::default()
            }));
            while let Ok(req) = rx.recv() {
                match req {
                    Request::Execute { id, .. } => {
                        let _ = tx.send(Response::Executed {
                            id,
                            outcome: ExecOutcome {
                                ok: false,
                                error: Some("Python unavailable".into()),
                                ..Default::default()
                            },
                        });
                    }
                    Request::Reset { id } => { let _ = tx.send(Response::ResetDone { id }); }
                    Request::Shutdown => break,
                }
            }
            let _ = tx.send(Response::Stopped);
            return;
        }
    };
    let _ = tx.send(Response::Info(info));

    // The GIL is only held while a request is being executed.
    while let Ok(req) = rx.recv() {
        match req {
            Request::Execute { id, code, kind } => {
                if kind == CodeKind::UserTrusted && !cfg.allow_user_code {
                    let _ = tx.send(Response::Executed {
                        id,
                        outcome: ExecOutcome {
                            ok: false,
                            error: Some("user Python code is disabled in settings (trust control)".into()),
                            ..Default::default()
                        },
                    });
                    continue;
                }
                CANCEL.store(false, Ordering::SeqCst);
                let result: PyResult<String> = Python::with_gil(|py| {
                    let m = module.bind(py);
                    m.getattr("execute")?.call1((code,))?.extract()
                });
                match result {
                    Ok(json) => match serde_json::from_str::<ExecOutcome>(&json) {
                        Ok(outcome) => { let _ = tx.send(Response::Executed { id, outcome }); }
                        Err(e) => { let _ = tx.send(Response::Error { id, message: format!("bad runtime payload: {e}") }); }
                    },
                    Err(e) => {
                        let _ = tx.send(Response::Executed {
                            id,
                            outcome: ExecOutcome { ok: false, error: Some(e.to_string()), ..Default::default() },
                        });
                    }
                }
            }
            Request::Reset { id } => {
                let _ = Python::with_gil(|py| -> PyResult<()> {
                    module.bind(py).getattr("reset")?.call0()?;
                    Ok(())
                });
                let _ = tx.send(Response::ResetDone { id });
            }
            Request::Shutdown => break,
        }
    }
    let _ = tx.send(Response::Stopped);
}

#[cfg(feature = "python")]
#[derive(serde::Deserialize)]
struct RawInfo {
    version: String,
    executable: String,
    #[serde(default)]
    libraries: Vec<(String, String)>,
}

#[cfg(feature = "python")]
#[pyo3::pyfunction]
fn cancel_requested() -> bool {
    CANCEL.load(Ordering::SeqCst)
}
