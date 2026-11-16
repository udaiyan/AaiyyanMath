//! Native calculation worker: keeps the GUI responsive even for heavy native
//! work (spec section 26). Values are plain data, so they move across threads.
use aaiyyan_ast::Statement;
use aaiyyan_evaluator::{Environment, EvalLimits, Evaluator, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct CalcJob {
    pub id: u64,
    pub full: bool,
    pub statements: Vec<(Uuid, Statement)>,
    pub env: Environment,
    pub timeout: Duration,
}

pub struct CalcDone {
    pub id: u64,
    pub full: bool,
    pub values: Vec<(Uuid, String, Value)>,
    pub errors: Vec<(Uuid, String)>,
    pub output: Vec<String>,
    pub duration_ms: f64,
    pub cancelled: bool,
}

enum Message {
    Run(CalcJob),
    Shutdown,
}

pub struct NativeWorker {
    tx: Sender<Message>,
    rx: Receiver<CalcDone>,
    cancel: Arc<AtomicBool>,
    next_id: u64,
}

impl NativeWorker {
    pub fn spawn() -> NativeWorker {
        let (tx, rx_in) = channel::<Message>();
        let (tx_out, rx) = channel::<CalcDone>();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        std::thread::Builder::new()
            .name("aaiyyan-native-calc".into())
            .spawn(move || loop {
                match rx_in.recv() {
                    Ok(Message::Run(job)) => {
                        worker_cancel.store(false, Ordering::SeqCst);
                        let done = execute(job, worker_cancel.clone());
                        if tx_out.send(done).is_err() { break; }
                    }
                    Ok(Message::Shutdown) | Err(_) => break,
                }
            })
            .expect("failed to spawn native calculation worker");
        NativeWorker { tx, rx, cancel, next_id: 1 }
    }

    pub fn run(&mut self, statements: Vec<(Uuid, Statement)>, env: Environment, full: bool, timeout: Duration) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let _ = self.tx.send(Message::Run(CalcJob { id, full, statements, env, timeout }));
        id
    }

    pub fn cancel(&self) { self.cancel.store(true, Ordering::SeqCst); }

    pub fn poll(&mut self) -> Vec<CalcDone> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(d) => out.push(d),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }

    pub fn shutdown(&self) { let _ = self.tx.send(Message::Shutdown); }
}

fn execute(job: CalcJob, cancel: Arc<AtomicBool>) -> CalcDone {
    let started = Instant::now();
    let mut env = if job.full { Environment::new() } else { job.env };
    let mut ev = Evaluator::new(EvalLimits {
        max_steps: 50_000_000,
        timeout: Some(job.timeout),
        cancel: Some(cancel),
    });
    let mut values = Vec::new();
    let mut errors = Vec::new();
    let mut cancelled = false;
    for (block, stmt) in &job.statements {
        match ev.eval_statement(stmt, &mut env) {
            Ok((name, value)) => values.push((*block, name, value)),
            Err(e) => {
                let msg = format!("{}: {e}", aaiyyan_ast::mathstr::statement_to_string(stmt));
                if matches!(e, aaiyyan_evaluator::EvalError::Cancelled) { cancelled = true; }
                errors.push((*block, msg));
                if cancelled { break; }
            }
        }
    }
    CalcDone {
        id: job.id,
        full: job.full,
        values,
        errors,
        output: ev.output,
        duration_ms: started.elapsed().as_secs_f64() * 1000.0,
        cancelled,
    }
}
