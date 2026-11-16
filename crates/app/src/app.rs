use aaiyyan_core::{LogLevel, Session};
use aaiyyan_gui::UiState;
use aaiyyan_project::Backend;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct AaiyyanApp {
    session: Session,
    ui: UiState,
    last_autosave: Instant,
}

impl AaiyyanApp {
    pub fn new(cc: &eframe::CreationContext<'_>, path: Option<PathBuf>, backend: Option<Backend>) -> AaiyyanApp {
        // Register the serif math families before the first frame is drawn.
        aaiyyan_gui::theme::install_fonts(&cc.egui_ctx);

        let mut session = match &path {
            Some(p) => match Session::open(p) {
                Ok(s) => s,
                Err(e) => {
                    let mut s = Session::new();
                    s.log(LogLevel::Error, format!("could not open {}: {e}", p.display()));
                    s
                }
            },
            None => Session::new(),
        };
        if let Some(b) = backend { session.project.settings.backend = b; }
        session.apply_runtime_settings();
        session.request_recalc();
        AaiyyanApp { session, ui: UiState::default(), last_autosave: Instant::now() }
    }
}

impl eframe::App for AaiyyanApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        aaiyyan_gui::render(ctx, &mut self.session, &mut self.ui);
        if self.last_autosave.elapsed() > Duration::from_secs(120) {
            self.last_autosave = Instant::now();
            let dir = std::env::temp_dir().join("aaiyyan-recovery");
            if let Err(e) = self.session.project.save_recovery(&dir) {
                log::warn!("recovery save failed: {e}");
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.session.shutdown();
        }
    }
}
