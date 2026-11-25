use eframe::egui;
use serde::{Serialize, Deserialize};
use std::fs;
use crate::worksheet::Worksheet;
use crate::python::generate_python;

const SAVE_FILE: &str = "aaiyyanmath_doc.json";

#[derive(Serialize, Deserialize)]
pub struct MathcadApp {
    worksheets: Vec<Worksheet>,
    active_idx: usize,
    #[serde(skip)]
    status_message: Option<String>,
}

impl Default for MathcadApp {
    fn default() -> Self {
        if let Ok(json) = fs::read_to_string(SAVE_FILE) {
            if let Ok(mut app) = serde_json::from_str::<MathcadApp>(&json) {
                for ws in &mut app.worksheets {
                    ws.evaluate_all();
                }
                app.status_message = Some(format!("Loaded {}", SAVE_FILE));
                return app;
            }
        }

        Self {
            worksheets: vec![Worksheet::new("Scratchpad")],
            active_idx: 0,
            status_message: None,
        }
    }
}

impl MathcadApp {
    fn save_to_file(&mut self) {
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
            Ok(json) => {
                match serde_json::from_str::<MathcadApp>(&json) {
                    Ok(mut loaded) => {
                        for ws in &mut loaded.worksheets {
                            ws.evaluate_all();
                        }
                        self.worksheets = loaded.worksheets;
                        self.active_idx = loaded.active_idx;
                        if self.active_idx >= self.worksheets.len() {
                            self.active_idx = 0;
                        }
                        self.status_message = Some(format!("✅ Loaded from {}", SAVE_FILE));
                    }
                    Err(e) => self.status_message = Some(format!("❌ Parse failed: {}", e)),
                }
            }
            Err(e) => self.status_message = Some(format!("❌ Read failed: {}", e)),
        }
    }
}

impl eframe::App for MathcadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                            .text_color(if self.active_idx == i { egui::Color32::LIGHT_BLUE } else { ui.style().visuals.text_color() })
                    );

                    if response.changed() { ws.name = name; }
                    if response.clicked() { clicked_idx = Some(i); }
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
                ui.heading("Python Code");
                ui.separator();

                if let Some(ws) = self.worksheets.get(self.active_idx) {
                    let code = generate_python(ws);
                    let mut code_view = code.clone();

                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut code_view)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .interactive(false)
                        );
                    });
                }
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
    use crate::region::Region;
    use crate::expression::Expression;

    fn make_test_app() -> MathcadApp {
        let mut ws2 = Worksheet::new("Test WS");
        ws2.regions = vec![
            Region::Text("Test text".to_string()),
            Region::Math(Expression::new("x", "999 m")),
        ];

        MathcadApp {
            worksheets: vec![Worksheet::new("Scratchpad"), ws2],
            active_idx: 1,
            status_message: None,
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let app = make_test_app();
        let json = serde_json::to_string(&app).unwrap();

        assert!(!json.contains("bindings"), "Bindings should not be saved to disk!");
        assert!(!json.contains("last_key"), "Plot cache should not be saved!");

        let mut loaded: MathcadApp = serde_json::from_str(&json).unwrap();
        loaded.worksheets[1].evaluate_all();

        assert_eq!(loaded.worksheets.len(), 2);
        assert_eq!(loaded.active_idx, 1);
        assert_eq!(loaded.worksheets[1].name, "Test WS");

        let x_binding = loaded.worksheets[1].bindings.get("x").expect("Math failed to re-evaluate on load");
        assert_eq!(x_binding.value, 999.0);
        assert_eq!(x_binding.dim, crate::quantity::Dim::LENGTH);
    }

    #[test]
    fn test_file_io_roundtrip() {
        let app = make_test_app();

        // FIX: Use a unique filename to prevent parallel test race conditions
        let temp_path = "test_temp_doc_io_roundtrip.json";

        let json = serde_json::to_string_pretty(&app).unwrap();
        std::fs::write(temp_path, &json).unwrap();

        let read_json = std::fs::read_to_string(temp_path).unwrap();
        let mut loaded: MathcadApp = serde_json::from_str(&read_json).unwrap();

        loaded.worksheets[1].evaluate_all();

        assert_eq!(loaded.worksheets[1].regions.len(), 2);
        assert_eq!(loaded.worksheets[1].bindings.get("x").unwrap().value, 999.0);

        let _ = std::fs::remove_file(temp_path);
    }
}