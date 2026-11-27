use crate::python::generate_python;
use crate::runner::PythonRunner;
use crate::worksheet::Worksheet;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;

const SAVE_FILE: &str = "aaiyyanmath_doc.json";

/// On-disk document format version. `from_json` accepts documents at or below
/// this version and rejects anything newer. Bump only with a migration plan.
const DOC_VERSION: u32 = 1;

/// Legacy documents written before versioning have no `version` field; they
/// are version 1 by definition. Used as the serde default.
fn default_doc_version() -> u32 {
    DOC_VERSION
}

#[derive(Serialize, Deserialize)]
pub struct MathcadApp {
    #[serde(default = "default_doc_version")]
    version: u32,
    worksheets: Vec<Worksheet>,
    active_idx: usize,
    #[serde(skip)]
    status_message: Option<String>,
    #[serde(skip)]
    runner: PythonRunner,
    #[serde(skip)]
    plot_textures: Vec<egui::TextureHandle>,
}

impl Default for MathcadApp {
    fn default() -> Self {
        match fs::read_to_string(SAVE_FILE) {
            Ok(json) => match Self::from_json(&json) {
                Ok(mut app) => {
                    app.status_message = Some(format!("Loaded {}", SAVE_FILE));
                    app
                }
                Err(e) => {
                    let mut app = Self::fresh();
                    app.status_message = Some(format!("⚠ Could not load {}: {}", SAVE_FILE, e));
                    app
                }
            },
            Err(_) => Self::fresh(),
        }
    }
}

impl MathcadApp {
    /// A brand-new document: one evaluated Scratchpad worksheet.
    fn fresh() -> Self {
        Self {
            version: DOC_VERSION,
            worksheets: vec![Worksheet::new("Scratchpad")],
            active_idx: 0,
            status_message: None,
            runner: PythonRunner::default(),
            plot_textures: Vec::new(),
        }
    }

    /// Single load path for saved documents (auto-load, Load button, tests).
    /// Parses, rejects future versions, normalizes the version, repairs empty
    /// worksheet lists and out-of-range `active_idx`, then re-evaluates every
    /// worksheet top-down so bindings are never trusted from disk.
    fn from_json(json: &str) -> Result<MathcadApp, String> {
        let mut app: MathcadApp =
            serde_json::from_str(json).map_err(|e| format!("Parse failed: {}", e))?;

        if app.version > DOC_VERSION {
            return Err(format!(
                "Document version {} is newer than this app supports (max {}). Update AaiyyanMath to open it.",
                app.version, DOC_VERSION
            ));
        }
        app.version = DOC_VERSION;

        if app.worksheets.is_empty() {
            app.worksheets.push(Worksheet::new("Scratchpad"));
        }
        if app.active_idx >= app.worksheets.len() {
            app.active_idx = 0;
        }
        for ws in &mut app.worksheets {
            ws.evaluate_all();
        }
        Ok(app)
    }

    fn load_plot_textures(&mut self, ctx: &egui::Context) {
        self.plot_textures.clear();
        let dir = match &self.runner.last {
            Some(last) => last.plot_dir.clone(),
            None => return,
        };
        let Ok(entries) = fs::read_dir(&dir) else {
            return;
        };
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "png").unwrap_or(false))
            .collect();
        paths.sort();
        for path in paths {
            if let Ok(img) = image::open(&path) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                self.plot_textures.push(ctx.load_texture(
                    path.to_string_lossy(),
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
    }

    fn save_to_file(&mut self) {
        self.version = DOC_VERSION;
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = fs::write(SAVE_FILE, json) {
                    self.status_message = Some(format!("❌ Save failed: {}", e));
                } else {
                    self.status_message = Some(format!("✅ Saved to {}", SAVE_FILE));
                }
            }
            Err(e) => self.status_message = Some(format!("❌ Serialize failed: {}", e)),
        }
    }

    fn load_from_file(&mut self) {
        match fs::read_to_string(SAVE_FILE) {
            Ok(json) => match Self::from_json(&json) {
                Ok(loaded) => {
                    self.version = loaded.version;
                    self.worksheets = loaded.worksheets;
                    self.active_idx = loaded.active_idx;
                    self.status_message = Some(format!("✅ Loaded from {}", SAVE_FILE));
                }
                Err(e) => self.status_message = Some(format!("❌ {}", e)),
            },
            Err(e) => self.status_message = Some(format!("❌ Read failed: {}", e)),
        }
    }
}

impl eframe::App for MathcadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.runner.poll() {
            self.load_plot_textures(ctx);
        }
        if self.runner.running {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("AaiyyanMath");
                ui.separator();
                if ui.button("💾 Save Document").clicked() {
                    self.save_to_file();
                }
                if ui.button("📂 Load Document").clicked() {
                    self.load_from_file();
                }
                if let Some(msg) = &self.status_message {
                    ui.label(msg);
                }
            });
        });

        egui::SidePanel::left("workspace_sidebar")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Workspaces");
                ui.separator();

                let mut clicked_idx = None;
                for (i, ws) in self.worksheets.iter_mut().enumerate() {
                    let mut name = ws.name.clone();
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut name)
                            .desired_width(ui.available_width() - 10.0)
                            .text_color(if self.active_idx == i {
                                egui::Color32::LIGHT_BLUE
                            } else {
                                ui.style().visuals.text_color()
                            }),
                    );

                    if response.changed() {
                        ws.name = name;
                    }
                    if response.clicked() {
                        clicked_idx = Some(i);
                    }
                    ui.add_space(2.0);
                }

                if let Some(idx) = clicked_idx {
                    self.active_idx = idx;
                }

                ui.separator();
                if ui.button("➕ New Workspace").clicked() {
                    let new_name = format!("Worksheet {}", self.worksheets.len() + 1);
                    self.worksheets.push(Worksheet::new(&new_name));
                    self.active_idx = self.worksheets.len() - 1;
                }
            });

        egui::SidePanel::right("python_code_panel")
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Python");
                    if ui
                        .add_enabled(!self.runner.running, egui::Button::new("▶ Run"))
                        .clicked()
                    {
                        if let Some(ws) = self.worksheets.get(self.active_idx) {
                            self.runner.start(ws);
                        }
                    }
                    if self.runner.running {
                        ui.spinner();
                        if ui.button("⏹ Cancel").clicked() {
                            self.runner.cancel();
                        }
                    }
                });

                if let Some(last) = &self.runner.last {
                    if last.success {
                        ui.colored_label(
                            egui::Color32::from_rgb(120, 200, 120),
                            format!("✅ exit 0 · {:.2?}", last.duration),
                        );
                    } else {
                        let why = last
                            .problem
                            .clone()
                            .unwrap_or_else(|| format!("exit {:?}", last.exit_code));
                        ui.colored_label(
                            egui::Color32::from_rgb(230, 120, 120),
                            format!("❌ {} · {:.2?}", why, last.duration),
                        );
                    }
                }
                ui.separator();

                if let Some(ws) = self.worksheets.get(self.active_idx) {
                    let code = generate_python(ws);
                    let mut code_view = code.clone();
                    egui::ScrollArea::vertical()
                        .id_salt("py_code")
                        .max_height(ui.available_height() * 0.45)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut code_view)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .interactive(false),
                            );
                        });
                }

                ui.separator();
                ui.strong("Output");
                egui::ScrollArea::vertical()
                    .id_salt("py_output")
                    .show(ui, |ui| {
                        if let Some(last) = &self.runner.last {
                            if !last.stdout.is_empty() {
                                ui.monospace(last.stdout.as_str());
                            }
                            if !last.stderr.is_empty() {
                                ui.colored_label(
                                    egui::Color32::from_rgb(230, 120, 120),
                                    last.stderr.as_str(),
                                );
                            }
                            for tex in &self.plot_textures {
                                ui.add(
                                    egui::Image::new(egui::load::SizedTexture::new(
                                        tex.id(),
                                        tex.size_vec2(),
                                    ))
                                    .max_width(ui.available_width()),
                                );
                            }
                            if last.stdout.is_empty()
                                && last.stderr.is_empty()
                                && self.plot_textures.is_empty()
                            {
                                ui.weak("(no output)");
                            }
                        } else {
                            ui.weak("Press ▶ Run to execute this script with your local Python.");
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ws) = self.worksheets.get_mut(self.active_idx) {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ws.render(ui);
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::Expression;
    use crate::quantity::Dim;
    use crate::region::Region;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

    /// Unique path under the OS temp dir — tests must never write into the
    /// repo working directory (the app's live save file lives there).
    fn temp_json_path(tag: &str) -> std::path::PathBuf {
        let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "aaiyyanmath_test_{}_{}_{}.json",
            tag,
            std::process::id(),
            seq
        ))
    }

    fn make_test_app() -> MathcadApp {
        let mut ws2 = Worksheet::new("Test WS");
        ws2.regions = vec![
            Region::Text("Test text".to_string()),
            Region::Math(Expression::new("x", "999 m")),
        ];

        let mut app = MathcadApp::fresh();
        app.worksheets.push(ws2);
        app.active_idx = 1;
        app
    }

    /// One worksheet, two chained math regions, NO version field — the exact
    /// shape of documents written before versioning existed.
    fn legacy_doc() -> serde_json::Value {
        serde_json::json!({
            "worksheets": [
                {
                    "name": "Legacy",
                    "regions": [
                        { "Math": { "label": "a", "input": "2 + 3 * 4" } },
                        { "Math": { "label": "b", "input": "a * 2 m" } }
                    ]
                }
            ],
            "active_idx": 0
        })
    }

    /// `unwrap_err()` would require `MathcadApp: Debug`, which we deliberately
    /// don't derive (runner / GPU texture state). `.err()` drops the Ok value,
    /// so no Debug bound is needed.
    fn expect_rejected(json: &str) -> String {
        MathcadApp::from_json(json)
            .err()
            .expect("document should have been rejected")
    }

    #[test]
    fn doc_version_constant_and_default_fn_agree() {
        assert_eq!(DOC_VERSION, 1);
        assert_eq!(default_doc_version(), DOC_VERSION);
    }

    #[test]
    fn fresh_app_has_one_evaluated_scratchpad() {
        let app = MathcadApp::fresh();
        assert_eq!(app.version, DOC_VERSION);
        assert_eq!(app.worksheets.len(), 1);
        assert_eq!(app.worksheets[0].name, "Scratchpad");
        assert_eq!(app.active_idx, 0);
        assert!(app.status_message.is_none());
        assert_eq!(app.worksheets[0].bindings.get("a").unwrap().value, 14.0);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let app = make_test_app();
        let json = serde_json::to_string(&app).unwrap();

        assert!(
            !json.contains("bindings"),
            "Bindings should not be saved to disk!"
        );
        assert!(
            !json.contains("last_key"),
            "Plot cache should not be saved!"
        );
        assert!(
            !json.contains("status_message"),
            "Transient UI state should not be saved!"
        );
        assert!(
            !json.contains("plot_textures"),
            "GPU textures should not be saved!"
        );

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["version"], serde_json::json!(DOC_VERSION));

        let loaded = MathcadApp::from_json(&json).unwrap();
        assert_eq!(loaded.version, DOC_VERSION);
        assert_eq!(loaded.worksheets.len(), 2);
        assert_eq!(loaded.active_idx, 1);
        assert_eq!(loaded.worksheets[1].name, "Test WS");

        let x_binding = loaded.worksheets[1]
            .bindings
            .get("x")
            .expect("Math failed to re-evaluate on load");
        assert_eq!(x_binding.value, 999.0);
        assert_eq!(x_binding.dim, Dim::LENGTH);
    }

    #[test]
    fn test_file_io_roundtrip() {
        let app = make_test_app();
        let path = temp_json_path("file_io");

        let json = serde_json::to_string_pretty(&app).unwrap();
        std::fs::write(&path, &json).unwrap();

        let read_json = std::fs::read_to_string(&path).unwrap();
        let loaded = MathcadApp::from_json(&read_json).unwrap();

        assert_eq!(loaded.version, DOC_VERSION);
        assert_eq!(loaded.worksheets[1].regions.len(), 2);
        assert_eq!(loaded.worksheets[1].bindings.get("x").unwrap().value, 999.0);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_json_without_version_loads_and_evaluates() {
        let raw = legacy_doc().to_string();
        assert!(!raw.contains("version"), "fixture must be version-less");

        let app = MathcadApp::from_json(&raw).unwrap();
        assert_eq!(app.version, DOC_VERSION);
        assert_eq!(app.worksheets.len(), 1);
        assert_eq!(app.worksheets[0].name, "Legacy");
        assert_eq!(app.worksheets[0].bindings.get("a").unwrap().value, 14.0);
        let b = app.worksheets[0].bindings.get("b").unwrap();
        assert_eq!(b.value, 28.0);
        assert_eq!(b.dim, Dim::LENGTH);
    }

    #[test]
    fn legacy_doc_is_upgraded_on_serialize() {
        let app = MathcadApp::from_json(&legacy_doc().to_string()).unwrap();
        let json = serde_json::to_string(&app).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["version"], serde_json::json!(DOC_VERSION));
    }

    #[test]
    fn committed_example_loads() {
        let raw = include_str!("../examples/sample_doc.json");
        assert!(
            !raw.contains("\"version\""),
            "committed fixture must stay version-less"
        );

        let app = MathcadApp::from_json(raw).unwrap();
        assert_eq!(app.version, DOC_VERSION);
        assert_eq!(app.worksheets.len(), 3);
        assert_eq!(app.active_idx, 2);
        assert_eq!(app.worksheets[0].name, "Scratchpad");
        assert_eq!(app.worksheets[1].name, "Worksheet 2");
        assert_eq!(app.worksheets[2].name, "Worksheet 3");

        for ws in &app.worksheets {
            assert_eq!(ws.bindings.get("a").unwrap().value, 14.0);
            let b = ws.bindings.get("b").unwrap();
            assert_eq!(b.value, 28.0);
            assert_eq!(b.dim, Dim::LENGTH);
            assert!(matches!(ws.regions.last(), Some(Region::Plot(_))));
        }
        assert!(matches!(app.worksheets[0].regions[0], Region::Math(_)));
        assert!(matches!(app.worksheets[0].regions[1], Region::Text(_)));
    }

    #[test]
    fn future_version_is_rejected() {
        let mut doc = legacy_doc();
        doc["version"] = serde_json::json!(999);
        let err = expect_rejected(&doc.to_string());
        assert!(err.contains("999"), "got: {err}");
        assert!(err.contains("newer"), "got: {err}");
    }

    #[test]
    fn next_version_is_rejected() {
        let mut doc = legacy_doc();
        doc["version"] = serde_json::json!(DOC_VERSION + 1);
        assert!(MathcadApp::from_json(&doc.to_string()).is_err());
    }

    #[test]
    fn current_version_is_accepted() {
        let mut doc = legacy_doc();
        doc["version"] = serde_json::json!(DOC_VERSION);
        let app = MathcadApp::from_json(&doc.to_string()).unwrap();
        assert_eq!(app.version, DOC_VERSION);
    }

    #[test]
    fn version_zero_is_normalized() {
        let mut doc = legacy_doc();
        doc["version"] = serde_json::json!(0);
        let app = MathcadApp::from_json(&doc.to_string()).unwrap();
        assert_eq!(app.version, DOC_VERSION);
    }

    #[test]
    fn malformed_json_is_rejected() {
        let err = expect_rejected("{ this is not json");
        assert!(err.contains("Parse failed"), "got: {err}");
    }

    #[test]
    fn empty_string_is_rejected() {
        let err = expect_rejected("");
        assert!(err.contains("Parse failed"), "got: {err}");
    }

    #[test]
    fn non_numeric_version_is_rejected() {
        let mut doc = legacy_doc();
        doc["version"] = serde_json::json!("one");
        let err = expect_rejected(&doc.to_string());
        assert!(err.contains("Parse failed"), "got: {err}");
    }

    #[test]
    fn negative_version_is_rejected() {
        let mut doc = legacy_doc();
        doc["version"] = serde_json::json!(-1);
        let err = expect_rejected(&doc.to_string());
        assert!(err.contains("Parse failed"), "got: {err}");
    }

    #[test]
    fn out_of_range_active_idx_clamps_to_zero() {
        let mut doc = legacy_doc();
        doc["active_idx"] = serde_json::json!(42);
        let app = MathcadApp::from_json(&doc.to_string()).unwrap();
        assert_eq!(app.worksheets.len(), 1);
        assert_eq!(app.active_idx, 0);
    }

    #[test]
    fn active_idx_equal_to_len_clamps_to_zero() {
        let mut doc = legacy_doc();
        // One worksheet: index 1 is out of range.
        doc["active_idx"] = serde_json::json!(1);
        let app = MathcadApp::from_json(&doc.to_string()).unwrap();
        assert_eq!(app.active_idx, 0);
    }

    #[test]
    fn valid_active_idx_is_preserved() {
        let app = MathcadApp::from_json(include_str!("../examples/sample_doc.json")).unwrap();
        assert_eq!(app.worksheets.len(), 3);
        assert_eq!(app.active_idx, 2);
    }

    #[test]
    fn empty_worksheets_repaired_to_scratchpad() {
        let mut doc = legacy_doc();
        doc["worksheets"] = serde_json::json!([]);
        let app = MathcadApp::from_json(&doc.to_string()).unwrap();
        assert_eq!(app.worksheets.len(), 1);
        assert_eq!(app.worksheets[0].name, "Scratchpad");
        assert_eq!(app.active_idx, 0);
        assert_eq!(app.worksheets[0].bindings.get("a").unwrap().value, 14.0);
    }

    #[test]
    fn empty_worksheets_with_bad_active_idx_repaired() {
        let mut doc = legacy_doc();
        doc["worksheets"] = serde_json::json!([]);
        doc["active_idx"] = serde_json::json!(7);
        let app = MathcadApp::from_json(&doc.to_string()).unwrap();
        assert_eq!(app.worksheets.len(), 1);
        assert_eq!(app.active_idx, 0);
    }

    #[test]
    fn bindings_are_recomputed_top_down_on_load() {
        let doc = serde_json::json!({
            "worksheets": [
                {
                    "name": "Chain",
                    "regions": [
                        { "Math": { "label": "x", "input": "999 m" } },
                        { "Math": { "label": "y", "input": "x * 2" } }
                    ]
                }
            ],
            "active_idx": 0
        });
        let app = MathcadApp::from_json(&doc.to_string()).unwrap();
        let ws = &app.worksheets[0];
        assert_eq!(ws.bindings.get("x").unwrap().value, 999.0);
        assert_eq!(ws.bindings.get("x").unwrap().dim, Dim::LENGTH);
        assert_eq!(ws.bindings.get("y").unwrap().value, 1998.0);
        assert_eq!(ws.bindings.get("y").unwrap().dim, Dim::LENGTH);
    }

    #[test]
    fn serialized_bindings_in_file_are_ignored() {
        let mut doc = legacy_doc();
        doc["worksheets"][0]["bindings"] = serde_json::json!({ "a": 12345.0 });
        let app = MathcadApp::from_json(&doc.to_string()).unwrap();
        assert_eq!(app.worksheets[0].bindings.get("a").unwrap().value, 14.0);
    }
}
