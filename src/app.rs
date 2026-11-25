use eframe::egui;
use crate::worksheet::Worksheet;
use crate::python::generate_python;

pub struct MathcadApp {
    worksheets: Vec<Worksheet>,
    active_idx: usize,
}

impl Default for MathcadApp {
    fn default() -> Self {
        Self {
            worksheets: vec![Worksheet::new("Scratchpad")],
            active_idx: 0,
        }
    }
}

impl eframe::App for MathcadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

    #[test]
    fn app_starts_with_one_workspace() {
        let app = MathcadApp::default();
        assert_eq!(app.worksheets.len(), 1);
        assert_eq!(app.active_idx, 0);
    }

    #[test]
    fn can_add_workspaces() {
        let mut app = MathcadApp::default();
        app.worksheets.push(Worksheet::new("Test WS"));
        app.active_idx = 1;
        assert_eq!(app.worksheets.len(), 2);
        assert_eq!(app.worksheets[1].name, "Test WS");
    }

    #[test]
    fn workspace_bindings_are_isolated() {
        let mut ws1 = Worksheet::new("WS1");
        if let Region::Math(e) = &mut ws1.regions[1] { e.input = "100".to_owned(); }
        ws1.evaluate_all();

        let mut ws2 = Worksheet::new("WS2");
        if let Region::Math(e) = &mut ws2.regions[1] { e.input = "200".to_owned(); }
        ws2.evaluate_all();

        assert_eq!(ws1.bindings.get("a").unwrap().value, 100.0);
        assert_eq!(ws2.bindings.get("a").unwrap().value, 200.0);
    }

    #[test]
    fn switching_workspace_updates_python_output() {
        let mut app = MathcadApp::default();

        let code_ws1 = generate_python(&app.worksheets[0]);
        assert!(code_ws1.contains("a = 2+3*4"));

        let mut ws2 = Worksheet::new("WS2");
        ws2.regions = vec![Region::Math(Expression::new("x", "999 m"))];
        app.worksheets.push(ws2);

        app.active_idx = 1;
        let code_ws2 = generate_python(&app.worksheets[app.active_idx]);

        assert!(code_ws2.contains("x = 999*m"));
        assert!(!code_ws2.contains("2+3*4"));
    }
}