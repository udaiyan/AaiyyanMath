use eframe::egui;
use std::collections::HashMap;

use crate::expression::Expression;
use crate::plot::PlotPane;
use crate::quantity::Quantity;

pub struct MathcadApp {
    exprs: [Expression; 2],
    plot: PlotPane,
    bindings: HashMap<String, Quantity>,
    first_frame: bool,
}

impl Default for MathcadApp {
    fn default() -> Self {
        Self {
            exprs: [
                Expression::new("a", "2 + 3 * 4"),
                Expression::new("b", "a * 2 m"),
            ],
            plot: PlotPane::default(),
            bindings: HashMap::new(),
            first_frame: true,
        }
    }
}

impl MathcadApp {
    fn evaluate_all(&mut self) {
        self.bindings.clear();
        for i in 0..self.exprs.len() {
            let snapshot = self.bindings.clone();
            self.exprs[i].evaluate(&snapshot);
            if let Some(q) = &self.exprs[i].result {
                let name = self.exprs[i].label.trim().to_owned();
                if !name.is_empty() {
                    self.bindings.insert(name, q.clone());
                }
            }
        }
    }
}

impl eframe::App for MathcadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("AaiyyanMath");

                let mut any_changed = false;
                for (i, expr) in self.exprs.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.label(format!("Expression {}", i + 1));
                        if expr.render(ui) {
                            any_changed = true;
                        }
                    });
                    ui.add_space(6.0);
                }

                if self.first_frame {
                    self.first_frame = false;
                    self.evaluate_all();
                } else if any_changed {
                    self.evaluate_all();
                }

                ui.separator();
                ui.group(|ui| {
                    ui.label("Plot");
                    self.plot.render(ui, &self.bindings);
                });

                ui.add_space(6.0);
                ui.separator();
                ui.heading("Bindings");
                if self.bindings.is_empty() {
                    ui.weak("(none)");
                } else {
                    let mut names: Vec<_> = self.bindings.keys().cloned().collect();
                    names.sort();
                    for name in names {
                        let q = &self.bindings[&name];
                        ui.monospace(format!("{} = {}", name, q));
                    }
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_binds_a_and_b() {
        let mut app = MathcadApp::default();
        app.evaluate_all();
        assert_eq!(app.bindings.get("a").unwrap().value, 14.0);
        assert_eq!(app.bindings.get("b").unwrap().value, 28.0);
        assert_eq!(app.bindings.get("b").unwrap().dim, crate::quantity::Dim::LENGTH);
    }

    #[test]
    fn expression_chain_updates() {
        let mut app = MathcadApp::default();
        app.exprs[0].input = "5 m".to_owned();
        app.evaluate_all();
        assert_eq!(app.bindings.get("a").unwrap().value, 5.0);
        assert_eq!(app.bindings.get("b").unwrap().value, 10.0);
        // FIXED: b = a * 2 m. Since a = 5 m, b = 10 m^2.
        assert_eq!(app.bindings.get("b").unwrap().dim, crate::quantity::Dim::LENGTH * crate::quantity::Dim::LENGTH);
    }

    #[test]
    fn failure_in_pane1_breaks_pane2() {
        let mut app = MathcadApp::default();
        app.exprs[0].input = "2 +".to_owned();
        app.evaluate_all();
        assert!(!app.bindings.contains_key("a"));
        assert!(!app.bindings.contains_key("b"));
        assert!(app.exprs[1].error.is_some());
    }

    #[test]
    fn empty_label_skips_binding() {
        let mut app = MathcadApp::default();
        app.exprs[0].label = String::new();
        app.evaluate_all();
        assert!(!app.bindings.contains_key("a"));
        assert!(app.exprs[0].result.is_some());
        assert!(app.exprs[1].error.is_some());
    }

    #[test]
    fn stale_bindings_cleared_between_runs() {
        let mut app = MathcadApp::default();
        app.evaluate_all();
        assert!(app.bindings.contains_key("a"));
        app.exprs[0].input = "bad +".to_owned();
        app.evaluate_all();
        assert!(!app.bindings.contains_key("a"));
    }

    #[test]
    fn later_pane_with_same_name_overwrites() {
        let mut app = MathcadApp::default();
        app.exprs[1].label = "a".to_owned();
        app.exprs[1].input = "100".to_owned();
        app.evaluate_all();
        assert_eq!(app.bindings.get("a").unwrap().value, 100.0);
        assert!(!app.bindings.contains_key("b"));
    }
}