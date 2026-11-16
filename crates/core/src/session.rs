use crate::convert::dto_to_value;
use crate::results::{BlockResult, CalcReport, LogEntry, LogLevel, VariableInfo};
use crate::scheduler::{CalcDone, NativeWorker};
use aaiyyan_ast::Statement;
use aaiyyan_codegen::{generate, Options as CodegenOptions};
use aaiyyan_dependency::DependencyGraph;
use aaiyyan_evaluator::{Environment, Value};
use aaiyyan_plotting::{sample_2d, sample_3d, Plot2DSpec, Plot3DSpec, SeriesData, SurfaceData};
use aaiyyan_project::{Backend, CalcMode, Project};
use aaiyyan_python_runtime::{CodeKind, ExecOutcome, PythonHandle, Response, RuntimeConfig};
use aaiyyan_reporting::ReportInput;
use aaiyyan_worksheet::{Block, BlockKind, ControlKind, History, Worksheet};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct Session {
    pub project: Project,
    pub active: usize,
    pub graph: DependencyGraph,
    pub env: Environment,
    pub results: BTreeMap<String, BlockResult>,
    pub block_values: BTreeMap<Uuid, Vec<(String, Value)>>,
    pub block_errors: BTreeMap<Uuid, Vec<String>>,
    pub generated: String,
    pub console: VecDeque<LogEntry>,
    pub history: History,
    pub last_calc: Option<CalcReport>,
    pub project_path: Option<PathBuf>,
    pub stale: bool,
    pub busy: bool,
    pub python_outcome: Option<ExecOutcome>,
    parsed: Vec<(Uuid, Statement)>,
    def_blocks: BTreeMap<String, Uuid>,
    prev_sources: BTreeMap<String, String>,
    native: NativeWorker,
    python: PythonHandle,
    pending_native: Option<u64>,
    pending_python: Option<u64>,
    queued: bool,
    force_full: bool,
    last_edit_block: Option<Uuid>,
}

impl Session {
    pub fn new() -> Session {
        Session::from_project(Project::new_demo(), None)
    }

    pub fn from_project(project: Project, path: Option<PathBuf>) -> Session {
        let config = RuntimeConfig {
            extra_sys_paths: project.python.extra_paths.iter().map(PathBuf::from).collect(),
            allow_user_code: project.python.allow_user_code,
        };
        let mut s = Session {
            project,
            active: 0,
            graph: DependencyGraph::new(),
            env: Environment::new(),
            results: BTreeMap::new(),
            block_values: BTreeMap::new(),
            block_errors: BTreeMap::new(),
            generated: String::new(),
            console: VecDeque::new(),
            history: History::new(100),
            last_calc: None,
            project_path: path,
            stale: true,
            busy: false,
            python_outcome: None,
            parsed: Vec::new(),
            def_blocks: BTreeMap::new(),
            prev_sources: BTreeMap::new(),
            native: NativeWorker::spawn(),
            python: PythonHandle::spawn(config),
            pending_native: None,
            pending_python: None,
            queued: false,
            force_full: true,
            last_edit_block: None,
        };
        s.ensure_layout();
        s.log(LogLevel::Info, "AaiyyanMath engine ready");
        s
    }

    // ---------------- document access ----------------

    pub fn worksheet(&self) -> &Worksheet {
        let idx = self.active.min(self.project.worksheets.len().saturating_sub(1));
        &self.project.worksheets[idx]
    }

    pub fn worksheet_mut(&mut self) -> &mut Worksheet {
        let idx = self.active.min(self.project.worksheets.len().saturating_sub(1));
        &mut self.project.worksheets[idx]
    }

    pub fn record_history(&mut self) {
        let snapshot = self.worksheet().clone();
        self.history.record(&snapshot);
    }

    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.console.push_back(LogEntry::new(level, message));
        while self.console.len() > 500 { self.console.pop_front(); }
    }

    pub fn mark_dirty(&mut self) {
        self.stale = true;
        if self.project.settings.mode == CalcMode::Auto { self.request_recalc(); }
    }

    /// Force a full recalculation on the next request.
    pub fn force_recalc(&mut self) {
        self.force_full = true;
        self.stale = true;
        self.request_recalc();
    }

    pub fn update_block(&mut self, id: Uuid, kind: BlockKind) {
        self.record_history();
        if let Some(b) = self.worksheet_mut().block_mut(id) { b.kind = kind; }
        self.force_full = true;
        self.mark_dirty();
    }

    pub fn update_source(&mut self, id: Uuid, source: String) {
        let kind = match self.worksheet().block(id).map(|b| b.kind.clone()) {
            Some(BlockKind::Equation { .. }) => BlockKind::Equation { source },
            Some(BlockKind::Result { .. }) => BlockKind::Result { source },
            Some(BlockKind::Python { trusted, .. }) => BlockKind::Python { source, trusted },
            Some(BlockKind::Text { .. }) => BlockKind::Text { text: source },
            Some(BlockKind::Heading { level, .. }) => BlockKind::Heading { level, text: source },
            _ => return,
        };
        // One undo entry per editing session, not per keystroke.
        if self.last_edit_block != Some(id) {
            self.record_history();
            self.last_edit_block = Some(id);
        }
        if let Some(b) = self.worksheet_mut().block_mut(id) { b.kind = kind; }
        // Never display a result computed from the previous source.
        self.block_values.remove(&id);
        self.force_full = true;
        self.mark_dirty();
    }

    /// Called when a region leaves edit mode; closes the current undo group.
    pub fn end_edit(&mut self) {
        self.last_edit_block = None;
    }

    pub fn insert_block(&mut self, kind: BlockKind, after: Option<Uuid>) {
        self.record_history();
        let index = after.and_then(|id| self.worksheet().index_of(id)).map(|i| i + 1);
        self.worksheet_mut().insert(index, Block::new(kind));
        self.force_full = true;
        self.mark_dirty();
    }

    pub fn remove_block(&mut self, id: Uuid) {
        self.record_history();
        self.worksheet_mut().remove(id);
        self.block_values.remove(&id);
        self.block_errors.remove(&id);
        self.force_full = true;
        self.mark_dirty();
    }

    pub fn move_block(&mut self, id: Uuid, delta: i32) {
        self.record_history();
        if self.worksheet_mut().move_block(id, delta) {
            self.force_full = true;
            self.mark_dirty();
        }
    }

    pub fn add_worksheet(&mut self) {
        let n = self.project.worksheets.len() + 1;
        self.project.worksheets.push(Worksheet::new(&format!("Worksheet {n}")));
        self.active = self.project.worksheets.len() - 1;
        self.force_full = true;
        self.mark_dirty();
    }

    pub fn select_worksheet(&mut self, index: usize) {
        if index < self.project.worksheets.len() {
            self.active = index;
            self.force_full = true;
            self.mark_dirty();
        }
    }

    pub fn undo(&mut self) {
        let current = self.worksheet().clone();
        if let Some(ws) = self.history.undo(&current) {
            let idx = self.active;
            self.project.worksheets[idx] = ws;
            self.force_full = true;
            self.mark_dirty();
        }
    }

    pub fn redo(&mut self) {
        let current = self.worksheet().clone();
        if let Some(ws) = self.history.redo(&current) {
            let idx = self.active;
            self.project.worksheets[idx] = ws;
            self.force_full = true;
            self.mark_dirty();
        }
    }

    /// Give legacy/demo blocks a sensible vertical layout on the canvas.
    pub fn ensure_layout(&mut self) {
        for ws in &mut self.project.worksheets {
            let mut y = 48.0f32;
            for b in &mut ws.blocks {
                if b.x == 0.0 && b.y == 0.0 {
                    b.x = 72.0;
                    b.y = y;
                    y += match &b.kind {
                        BlockKind::Plot2D(_) => 348.0,
                        BlockKind::Plot3D(_) => 384.0,
                        BlockKind::Python { .. } => 204.0,
                        BlockKind::Table { .. } => 144.0,
                        BlockKind::Heading { .. } => 60.0,
                        BlockKind::Text { .. } => 60.0,
                        _ => 48.0,
                    };
                }
            }
        }
    }

    pub fn set_block_position(&mut self, id: Uuid, x: f32, y: f32) {
        if let Some(b) = self.worksheet_mut().block_mut(id) {
            b.x = x;
            b.y = y;
        }
    }

    pub fn insert_block_at(&mut self, kind: BlockKind, x: f32, y: f32) -> Uuid {
        self.record_history();
        let mut block = Block::new(kind);
        block.x = x;
        block.y = y;
        let id = block.id;
        self.worksheet_mut().blocks.push(block);
        self.force_full = true;
        self.mark_dirty();
        id
    }

    pub fn set_control_value(&mut self, id: Uuid, kind: ControlKind) {
        if let Some(BlockKind::Control(spec)) = self.worksheet_mut().block_mut(id).map(|b| &mut b.kind) {
            spec.kind = kind;
        }
        self.mark_dirty();
    }

    // ---------------- calculation ----------------

    pub fn request_recalc(&mut self) {
        // Parsing and code generation are cheap and authoritative: refresh them
        // on every call so the worksheet and Generated Python never lag the source.
        self.block_errors.clear();
        self.parsed.clear();
        self.def_blocks.clear();

        let ws = self.worksheet().clone();
        for block in &ws.blocks {
            if !block.enabled { continue; }
            let source = match &block.kind {
                BlockKind::Equation { source } | BlockKind::Result { source } => source.clone(),
                _ => continue,
            };
            match aaiyyan_parser::parse_program(&source) {
                Ok(stmts) => {
                    for s in stmts {
                        if let Some(name) = s.defined_name() {
                            self.def_blocks.insert(name.to_string(), block.id);
                        }
                        self.parsed.push((block.id, s));
                    }
                }
                Err(e) => {
                    self.block_errors.entry(block.id).or_default().push(e.describe(&source));
                }
            }
        }

        self.apply_controls();

        self.graph.clear();
        let mut sources: BTreeMap<String, String> = BTreeMap::new();
        for (_, stmt) in &self.parsed {
            if let Some(name) = stmt.defined_name() {
                self.graph.define(name, aaiyyan_ast::statement_dependencies(stmt));
                sources.insert(name.to_string(), aaiyyan_ast::mathstr::statement_to_string(stmt));
            }
        }

        let mut cyclic: BTreeSet<String> = BTreeSet::new();
        if let Some(cycle) = self.graph.order().err() {
            let msg = format!("circular dependency: {}", cycle.0.join(" -> "));
            self.log(LogLevel::Error, msg.clone());
            for name in &cycle.0 {
                cyclic.insert(name.clone());
                if let Some(id) = self.def_blocks.get(name) {
                    self.block_errors.entry(*id).or_default().push(msg.clone());
                }
            }
        }

        let same_names = sources.keys().eq(self.prev_sources.keys());
        let changed: Vec<String> = sources.iter()
            .filter(|(k, v)| self.prev_sources.get(*k) != Some(*v))
            .map(|(k, _)| k.clone())
            .collect();
        let full = self.force_full || !same_names || self.results.is_empty() || changed.is_empty();

        let subset: Vec<(Uuid, Statement)> = if full {
            self.parsed.clone()
        } else {
            let affected: BTreeSet<String> =
                self.graph.affected(&changed).unwrap_or_default().into_iter().collect();
            self.parsed.iter()
                .filter(|(_, s)| match s.defined_name() {
                    Some(n) => affected.contains(n),
                    None => true,
                })
                .cloned()
                .collect()
        };
        let subset: Vec<(Uuid, Statement)> = subset.into_iter()
            .filter(|(_, s)| s.defined_name().map(|n| !cyclic.contains(n)).unwrap_or(true))
            .collect();

        let statements: Vec<Statement> = self.parsed.iter().map(|(_, s)| s.clone()).collect();
        self.generated = generate(&statements, &CodegenOptions::default()).code;

        // Only evaluation is asynchronous. If the worker is busy, keep the fresh
        // AST/codegen and re-dispatch as soon as it reports back.
        if self.pending_native.is_some() || self.pending_python.is_some() {
            self.queued = true;
            self.busy = true;
            self.stale = true;
            return;
        }

        let env = if full { Environment::new() } else { self.env.clone() };
        let timeout = Duration::from_millis(self.project.settings.timeout_ms.max(100));
        let id = self.native.run(subset, env, full, timeout);
        self.pending_native = Some(id);
        self.prev_sources = sources;
        self.stale = false;
        self.busy = true;
        self.force_full = false;
    }

    fn apply_controls(&mut self) {
        let controls: Vec<(String, Value)> = self.worksheet().blocks.iter().filter_map(|b| {
            match &b.kind {
                BlockKind::Control(spec) => {
                    let value = match &spec.kind {
                        ControlKind::Slider { value, .. } => Value::Scalar(*value),
                        ControlKind::Number { value } => Value::Scalar(*value),
                        ControlKind::Checkbox { value } => Value::Bool(*value),
                        ControlKind::Dropdown { options, selected } => {
                            Value::Str(options.get(*selected).cloned().unwrap_or_default())
                        }
                    };
                    Some((spec.variable.clone(), value))
                }
                _ => None,
            }
        }).collect();
        for (var, value) in controls {
            for (_, stmt) in self.parsed.iter_mut() {
                if stmt.defined_name() == Some(var.as_str()) {
                    let expr = match &value {
                        Value::Scalar(v) => aaiyyan_ast::Expr::Number(*v),
                        Value::Bool(b) => aaiyyan_ast::Expr::Bool(*b),
                        Value::Str(s) => aaiyyan_ast::Expr::Str(s.clone()),
                        _ => continue,
                    };
                    stmt.set_value(expr);
                }
            }
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        for done in self.native.poll() {
            if Some(done.id) == self.pending_native {
                self.apply_native(done);
                changed = true;
            }
        }
        for response in self.python.poll() {
            match response {
                Response::Executed { id, outcome } => {
                    if Some(id) == self.pending_python { self.pending_python = None; }
                    self.apply_python(outcome);
                    changed = true;
                }
                Response::Error { message, .. } => { self.log(LogLevel::Error, message); changed = true; }
                Response::ResetDone { .. } => { self.log(LogLevel::Info, "Python namespace reset"); }
                Response::Info(_) | Response::Stopped => {}
            }
        }
        self.busy = self.pending_native.is_some() || self.pending_python.is_some();
        if !self.busy && self.queued {
            self.queued = false;
            self.request_recalc();
        }
        changed
    }

    fn apply_native(&mut self, done: CalcDone) {
        self.pending_native = None;
        if done.full {
            self.env = Environment::new();
            self.results.clear();
            self.block_values.clear();
        }
        let mut evaluated = Vec::new();
        for (block, name, value) in done.values {
            self.env.define_global(name.clone(), value.clone());
            let entry = self.block_values.entry(block).or_default();
            if let Some(slot) = entry.iter_mut().find(|(n, _)| *n == name) {
                slot.1 = value.clone();
            } else {
                entry.push((name.clone(), value.clone()));
            }
            if name != "_" {
                let source = self.prev_sources.get(&name).cloned().unwrap_or_default();
                self.results.insert(name.clone(), BlockResult {
                    name: name.clone(),
                    display: value.display(),
                    latex: value.latex(),
                    value,
                    source,
                });
                evaluated.push(name);
            }
        }
        let error_count = done.errors.len();
        for (block, message) in done.errors {
            self.block_errors.entry(block).or_default().push(message.clone());
            self.log(LogLevel::Error, message);
        }
        for line in done.output { self.log(LogLevel::Info, line); }
        self.last_calc = Some(CalcReport {
            duration_ms: done.duration_ms,
            evaluated,
            errors: error_count,
            backend: "native".into(),
            cancelled: done.cancelled,
        });

        let statements: Vec<Statement> = self.parsed.iter().map(|(_, s)| s.clone()).collect();
        let needs_python = aaiyyan_symbolic::needs_python_any(&statements);
        let want = match self.project.settings.backend {
            Backend::Python => true,
            Backend::Auto => needs_python,
            Backend::Native => false,
        };
        if want && self.python.available() {
            let id = self.python.execute(self.generated.clone(), CodeKind::Generated);
            self.pending_python = Some(id);
            self.busy = true;
        } else if needs_python {
            self.log(LogLevel::Warning,
                "symbolic operations need the embedded Python backend (build with --features python)");
        }
    }

    fn apply_python(&mut self, outcome: ExecOutcome) {
        for line in outcome.stdout.lines() {
            if !line.trim().is_empty() { self.log(LogLevel::Info, format!("[py] {line}")); }
        }
        for line in outcome.stderr.lines() {
            if !line.trim().is_empty() { self.log(LogLevel::Warning, format!("[py] {line}")); }
        }
        if let Some(err) = &outcome.error {
            self.log(LogLevel::Error, format!("Python: {err}"));
            if let Some(tb) = &outcome.traceback {
                for line in tb.lines().take(20) { self.log(LogLevel::Error, line.to_string()); }
            }
        }
        for dto in &outcome.values {
            if dto.name == "_expr" { continue; }
            if let Some(value) = dto_to_value(dto) {
                self.env.define_global(dto.name.clone(), value.clone());
                let source = self.prev_sources.get(&dto.name).cloned().unwrap_or_default();
                self.results.insert(dto.name.clone(), BlockResult {
                    name: dto.name.clone(),
                    display: if dto.display.is_empty() { value.display() } else { dto.display.clone() },
                    latex: dto.latex.clone().unwrap_or_else(|| value.latex()),
                    value: value.clone(),
                    source,
                });
                if let Some(block) = self.def_blocks.get(&dto.name).copied() {
                    let entry = self.block_values.entry(block).or_default();
                    if let Some(slot) = entry.iter_mut().find(|(n, _)| *n == dto.name) {
                        slot.1 = value;
                    } else {
                        entry.push((dto.name.clone(), value));
                    }
                }
            }
        }
        let mut report = self.last_calc.clone().unwrap_or_default();
        report.backend = if outcome.cancelled { "python (cancelled)".into() } else { "python".into() };
        report.duration_ms += outcome.duration_ms;
        self.last_calc = Some(report);
        self.python_outcome = Some(outcome);
    }

    pub fn cancel(&mut self) {
        self.native.cancel();
        self.python.cancel();
        self.log(LogLevel::Warning, "cancellation requested");
    }

    /// Blocking helper for tests and CLI usage only. Never call from the GUI.
    pub fn wait_for_idle(&mut self, timeout: Duration) -> bool {
        let start = Instant::now();
        if self.stale { self.request_recalc(); }
        loop {
            self.poll();
            if !self.busy && !self.stale { return true; }
            if start.elapsed() > timeout { return false; }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // ---------------- inspection ----------------

    pub fn result(&self, name: &str) -> Option<&BlockResult> { self.results.get(name) }

    pub fn values_for(&self, block: Uuid) -> &[(String, Value)] {
        self.block_values.get(&block).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn errors_for(&self, block: Uuid) -> &[String] {
        self.block_errors.get(&block).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn variables(&self) -> Vec<VariableInfo> {
        let mut out: Vec<VariableInfo> = self.results.values().map(|r| VariableInfo {
            name: r.name.clone(),
            kind: r.value.type_name().to_string(),
            display: r.display.clone(),
            unit: match &r.value {
                Value::Quantity(q) => Some(q.unit.name.clone()),
                _ => None,
            },
            latex: r.latex.clone(),
        }).collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    pub fn parsed_block(&self, block: Uuid) -> Vec<Statement> {
        self.parsed.iter().filter(|(id, _)| *id == block).map(|(_, s)| s.clone()).collect()
    }

    pub fn sample_plot2d(&self, spec: &Plot2DSpec) -> Result<Vec<SeriesData>, String> { sample_2d(spec, &self.env) }
    pub fn sample_plot3d(&self, spec: &Plot3DSpec) -> Result<SurfaceData, String> { sample_3d(spec, &self.env) }

    pub fn python_info(&self) -> &aaiyyan_python_runtime::PyInfo { self.python.info() }

    pub fn run_python_block(&mut self, id: Uuid) {
        let block = self.worksheet().block(id).cloned();
        if let Some(BlockKind::Python { source, trusted }) = block.map(|b| b.kind) {
            if !trusted {
                self.log(LogLevel::Warning, "Python block is not trusted; enable the trust checkbox to run it");
                return;
            }
            if !self.project.python.allow_user_code {
                self.log(LogLevel::Warning, "user Python execution is disabled in settings");
                return;
            }
            self.python.config.allow_user_code = true;
            let job = self.python.execute(source, CodeKind::UserTrusted);
            self.pending_python = Some(job);
            self.busy = true;
        }
    }

    pub fn reset_python(&mut self) { self.python.reset(); }

    pub fn apply_runtime_settings(&mut self) {
        self.python.config.allow_user_code = self.project.python.allow_user_code;
        self.python.config.extra_sys_paths =
            self.project.python.extra_paths.iter().map(PathBuf::from).collect();
    }

    // ---------------- persistence and export ----------------

    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        self.project.save(path).map_err(|e| e.to_string())?;
        self.project_path = Some(path.to_path_buf());
        self.log(LogLevel::Info, format!("saved {}", path.display()));
        Ok(())
    }

    pub fn open(path: &Path) -> Result<Session, String> {
        let project = Project::load(path).map_err(|e| e.to_string())?;
        Ok(Session::from_project(project, Some(path.to_path_buf())))
    }

    pub fn export_python(&self) -> String {
        let statements: Vec<Statement> = self.parsed.iter().map(|(_, s)| s.clone()).collect();
        generate(&statements, &CodegenOptions {
            use_units: true, capture_results: false, comments: true, standalone: true,
        }).code
    }

    fn report_input(&self) -> (BTreeMap<String, String>, BTreeMap<String, String>, Vec<(String, String)>) {
        let results: BTreeMap<String, String> =
            self.results.iter().map(|(k, v)| (k.clone(), v.display.clone())).collect();
        let latex: BTreeMap<String, String> =
            self.results.iter().map(|(k, v)| (k.clone(), v.latex.clone())).collect();
        let mut plots = Vec::new();
        for ws in &self.project.worksheets {
            for block in &ws.blocks {
                if let BlockKind::Plot2D(spec) = &block.kind {
                    if let Ok(series) = self.sample_plot2d(spec) {
                        plots.push((
                            spec.title.clone(),
                            aaiyyan_plotting::series_to_svg(&series, 720.0, 420.0, &spec.title),
                        ));
                    }
                }
            }
        }
        (results, latex, plots)
    }

    pub fn export_html(&self, path: &Path) -> Result<(), String> {
        let (results, latex, plots) = self.report_input();
        let python = self.export_python();
        let input = ReportInput { project: &self.project, results: &results, latex: &latex, python: &python, plots };
        std::fs::write(path, aaiyyan_reporting::export_html(&input)).map_err(|e| e.to_string())
    }

    pub fn export_markdown(&self, path: &Path) -> Result<(), String> {
        let (results, latex, plots) = self.report_input();
        let python = self.export_python();
        let input = ReportInput { project: &self.project, results: &results, latex: &latex, python: &python, plots };
        std::fs::write(path, aaiyyan_reporting::export_markdown(&input)).map_err(|e| e.to_string())
    }

    pub fn export_typst(&self, dir: &Path) -> Result<PathBuf, String> {
        let (results, latex, plots) = self.report_input();
        let python = self.export_python();
        let input = ReportInput { project: &self.project, results: &results, latex: &latex, python: &python, plots };
        aaiyyan_reporting::write_typst_report(dir, &input).map_err(|e| e.to_string())
    }

    pub fn export_pdf(&self, dir: &Path) -> Result<PathBuf, String> {
        let typ = self.export_typst(dir)?;
        let pdf = dir.join("report.pdf");
        aaiyyan_reporting::compile_pdf(&typ, &pdf).map_err(|e| e.to_string())?;
        Ok(pdf)
    }

    pub fn shutdown(&self) {
        self.native.shutdown();
        self.python.shutdown();
    }
}

impl Default for Session {
    fn default() -> Self { Session::new() }
}
