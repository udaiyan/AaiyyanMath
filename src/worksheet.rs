use eframe::egui;
use std::collections::HashMap;

use crate::expression::Expression;
use crate::plot::PlotPane;
use crate::quantity::Quantity;

pub struct Worksheet {
    pub name: String,
    pub exprs: Vec<Expression>,
    pub plot: PlotPane,
    pub bindings: HashMap<String, Quantity>,
}

impl Worksheet {
    pub fn new(name: &str) -> Self {
        let mut ws = Self {
            name: name.to_string(),
            exprs: vec![
                Expression::new("a", "2 + 3 * 4"),
                Expression::new("b", "a * 2 m"),
            ],
            plot: PlotPane::default(),
            bindings: HashMap::new(),
        };
        ws.evaluate_all();
        ws
    }

    pub fn evaluate_all(&mut self) {
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

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let mut any_changed = false;
        let mut remove_idx = None;

        let expr_count = self.exprs.len();

        for (i, expr) in self.exprs.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("Expression {}", i + 1));
                    if expr_count > 1 {
                        if ui.button("❌ Remove").clicked() {
                            remove_idx = Some(i);
                        }
                    }
                });
                ui.add_space(2.0);
                if expr.render(ui) {
                    any_changed = true;
                }
            });
            ui.add_space(6.0);
        }

        if let Some(idx) = remove_idx {
            self.exprs.remove(idx);
            any_changed = true;
        }

        if ui.button("➕ Add Expression").clicked() {
            let new_label = format!("v{}", self.exprs.len() + 1);
            self.exprs.push(Expression::new(&new_label, ""));
            any_changed = true;
        }

        if any_changed {
            self.evaluate_all();
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(5.0);

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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_binds_a_and_b() {
        let mut ws = Worksheet::new("Test");
        ws.evaluate_all();
        assert_eq!(ws.bindings.get("a").unwrap().value, 14.0);
        assert_eq!(ws.bindings.get("b").unwrap().value, 28.0);
        assert_eq!(ws.bindings.get("b").unwrap().dim, crate::quantity::Dim::LENGTH);
    }

    #[test]
    fn expression_chain_updates() {
        let mut ws = Worksheet::new("Test");
        ws.exprs[0].input = "5 m".to_owned();
        ws.evaluate_all();
        assert_eq!(ws.bindings.get("a").unwrap().value, 5.0);
        assert_eq!(ws.bindings.get("b").unwrap().value, 10.0);
        assert_eq!(ws.bindings.get("b").unwrap().dim, crate::quantity::Dim::LENGTH * crate::quantity::Dim::LENGTH);
    }

    #[test]
    fn failure_in_pane1_breaks_pane2() {
        let mut ws = Worksheet::new("Test");
        ws.exprs[0].input = "2 +".to_owned();
        ws.evaluate_all();
        assert!(!ws.bindings.contains_key("a"));
        assert!(!ws.bindings.contains_key("b"));
        assert!(ws.exprs[1].error.is_some());
    }

    #[test]
    fn empty_label_skips_binding() {
        let mut ws = Worksheet::new("Test");
        ws.exprs[0].label = String::new();
        ws.evaluate_all();
        assert!(!ws.bindings.contains_key("a"));
        assert!(ws.exprs[0].result.is_some());
        assert!(ws.exprs[1].error.is_some());
    }

    #[test]
    fn stale_bindings_cleared_between_runs() {
        let mut ws = Worksheet::new("Test");
        ws.evaluate_all();
        assert!(ws.bindings.contains_key("a"));
        ws.exprs[0].input = "bad +".to_owned();
        ws.evaluate_all();
        assert!(!ws.bindings.contains_key("a"));
    }

    #[test]
    fn later_pane_with_same_name_overwrites() {
        let mut ws = Worksheet::new("Test");
        ws.exprs[1].label = "a".to_owned();
        ws.exprs[1].input = "100".to_owned();
        ws.evaluate_all();
        assert_eq!(ws.bindings.get("a").unwrap().value, 100.0);
        assert!(!ws.bindings.contains_key("b"));
    }

    #[test]
    fn can_add_and_remove_expressions() {
        let mut ws = Worksheet::new("Test");
        assert_eq!(ws.exprs.len(), 2);

        // FIXED: Use b * 2 instead of a + b to avoid dimension mismatch
        ws.exprs.push(Expression::new("c", "b * 2"));
        ws.evaluate_all();
        assert_eq!(ws.bindings.get("c").unwrap().value, 56.0);
        assert_eq!(ws.bindings.get("c").unwrap().dim, crate::quantity::Dim::LENGTH);

        ws.exprs.remove(0); // Remove 'a'
        ws.evaluate_all();
        assert!(!ws.bindings.contains_key("a"));
        assert!(!ws.bindings.contains_key("b"));
        assert!(!ws.bindings.contains_key("c")); // because 'c' depends on 'b'
    }
}