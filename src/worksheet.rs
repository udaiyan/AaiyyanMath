use eframe::egui;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::expression::Expression;
use crate::plot::PlotPane;
use crate::quantity::Quantity;
use crate::region::Region;

#[derive(Serialize, Deserialize)]
pub struct Worksheet {
    pub name: String,
    pub regions: Vec<Region>,
    #[serde(skip)]
    pub bindings: HashMap<String, Quantity>,
}

impl Worksheet {
    pub fn new(name: &str) -> Self {
        let mut ws = Self {
            name: name.to_string(),
            regions: vec![
                Region::Text("Welcome to AaiyyanMath.\nThis is a Text Region. You can write notes here.".to_string()),
                Region::Math(Expression::new("a", "2 + 3 * 4")),
                Region::Math(Expression::new("b", "a * 2 m")),
                Region::Plot(PlotPane::default()),
            ],
            bindings: HashMap::new(),
        };
        ws.evaluate_all();
        ws
    }

    pub fn evaluate_all(&mut self) {
        self.bindings.clear();
        let mut current_bindings = HashMap::new();

        for region in &mut self.regions {
            if let Region::Math(expr) = region {
                expr.evaluate(&current_bindings);
                if let Some(q) = &expr.result {
                    let name = expr.label.trim().to_owned();
                    if !name.is_empty() {
                        current_bindings.insert(name, q.clone());
                    }
                }
            }
        }

        self.bindings = current_bindings;
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let mut any_changed = false;
        let mut remove_idx = None;
        let mut move_up_idx = None;
        let mut move_down_idx = None;

        let len = self.regions.len();

        for i in 0..len {
            ui.horizontal(|ui| {
                ui.strong(format!("Region {}", i + 1));
                if ui.button("⬆").clicked() && i > 0 { move_up_idx = Some(i); }
                if ui.button("⬇").clicked() && i < len - 1 { move_down_idx = Some(i); }
                if ui.button("❌ Remove").clicked() { remove_idx = Some(i); }
            });

            match &mut self.regions[i] {
                Region::Text(txt) => {
                    ui.add(
                        egui::TextEdit::multiline(txt)
                            .desired_width(f32::INFINITY)
                            .hint_text("Type your notes here...")
                    );
                }
                Region::Math(expr) => {
                    if expr.render(ui) {
                        any_changed = true;
                    }
                }
                Region::Plot(plot) => {
                    plot.render(ui, &self.bindings);
                }
            }
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(5.0);
        }

        if let Some(idx) = remove_idx {
            self.regions.remove(idx);
            any_changed = true;
        }
        if let Some(idx) = move_up_idx {
            self.regions.swap(idx, idx - 1);
            any_changed = true;
        }
        if let Some(idx) = move_down_idx {
            self.regions.swap(idx, idx + 1);
            any_changed = true;
        }

        ui.horizontal(|ui| {
            if ui.button("➕ Add Math").clicked() {
                let new_label = format!("v{}", len + 1);
                self.regions.push(Region::Math(Expression::new(&new_label, "")));
                any_changed = true;
            }
            if ui.button("➕ Add Text").clicked() {
                self.regions.push(Region::Text(String::new()));
                any_changed = true;
            }
            if ui.button("➕ Add Plot").clicked() {
                self.regions.push(Region::Plot(PlotPane::default()));
            }
        });

        if any_changed {
            self.evaluate_all();
        }

        ui.add_space(10.0);
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
    use crate::quantity::Dim;

    fn get_math_mut(ws: &mut Worksheet, index: usize) -> &mut Expression {
        match &mut ws.regions[index] {
            Region::Math(e) => e,
            _ => panic!("Not a math region"),
        }
    }

    #[test]
    fn default_state_binds_a_and_b() {
        let mut ws = Worksheet::new("Test");
        ws.evaluate_all();
        assert_eq!(ws.bindings.get("a").unwrap().value, 14.0);
        assert_eq!(ws.bindings.get("b").unwrap().value, 28.0);
        assert_eq!(ws.bindings.get("b").unwrap().dim, Dim::LENGTH);
    }

    #[test]
    fn expression_chain_updates() {
        let mut ws = Worksheet::new("Test");
        get_math_mut(&mut ws, 1).input = "5 m".to_owned();
        ws.evaluate_all();
        assert_eq!(ws.bindings.get("a").unwrap().value, 5.0);
        assert_eq!(ws.bindings.get("b").unwrap().value, 10.0);
        assert_eq!(ws.bindings.get("b").unwrap().dim, Dim::LENGTH * Dim::LENGTH);
    }

    #[test]
    fn failure_in_pane1_breaks_pane2() {
        let mut ws = Worksheet::new("Test");
        get_math_mut(&mut ws, 1).input = "2 +".to_owned();
        ws.evaluate_all();
        assert!(!ws.bindings.contains_key("a"));
        assert!(!ws.bindings.contains_key("b"));
        if let Region::Math(e) = &ws.regions[2] {
            assert!(e.error.is_some());
        }
    }

    #[test]
    fn empty_label_skips_binding() {
        let mut ws = Worksheet::new("Test");
        get_math_mut(&mut ws, 1).label = String::new();
        ws.evaluate_all();
        assert!(!ws.bindings.contains_key("a"));
        assert!(get_math_mut(&mut ws, 1).result.is_some());
        if let Region::Math(e) = &ws.regions[2] {
            assert!(e.error.is_some());
        }
    }

    #[test]
    fn stale_bindings_cleared_between_runs() {
        let mut ws = Worksheet::new("Test");
        ws.evaluate_all();
        assert!(ws.bindings.contains_key("a"));
        get_math_mut(&mut ws, 1).input = "bad +".to_owned();
        ws.evaluate_all();
        assert!(!ws.bindings.contains_key("a"));
    }

    #[test]
    fn later_pane_with_same_name_overwrites() {
        let mut ws = Worksheet::new("Test");
        get_math_mut(&mut ws, 2).label = "a".to_owned();
        get_math_mut(&mut ws, 2).input = "100".to_owned();
        ws.evaluate_all();
        assert_eq!(ws.bindings.get("a").unwrap().value, 100.0);
        assert!(!ws.bindings.contains_key("b"));
    }

    #[test]
    fn can_add_and_remove_regions() {
        let mut ws = Worksheet::new("Test");
        let initial_len = ws.regions.len();

        ws.regions.push(Region::Math(Expression::new("c", "b * 2")));
        ws.evaluate_all();
        assert_eq!(ws.regions.len(), initial_len + 1);
        assert_eq!(ws.bindings.get("c").unwrap().value, 56.0);

        ws.regions.remove(0);
        ws.evaluate_all();
        assert_eq!(ws.regions.len(), initial_len);
    }
}