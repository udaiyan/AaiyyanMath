use eframe::egui;
use exmex::{Express, FlatEx};
use std::collections::HashMap;
use crate::quantity::{Quantity, QuantityOpsFactory};

pub struct Expression {
    pub label: String,
    pub input: String,
    pub result: Option<Quantity>,
    pub error: Option<String>,
}

impl Expression {
    pub fn new(label: &str, initial_input: &str) -> Self {
        Self {
            label: label.to_owned(),
            input: initial_input.to_owned(),
            result: None,
            error: None,
        }
    }

    pub fn evaluate(&mut self, bindings: &HashMap<String, Quantity>) {
        self.result = None;
        self.error = None;

        if self.input.trim().is_empty() {
            return;
        }

        let processed = crate::quantity::preprocess(&self.input);
        let expr: FlatEx<Quantity, QuantityOpsFactory> = match FlatEx::<Quantity, QuantityOpsFactory>::parse(&processed) {
            Ok(e) => e,
            Err(e) => {
                self.error = Some(format!("Parse error: {}", e));
                return;
            }
        };

        let mut local_bindings = bindings.clone();
        for (name, q) in crate::quantity::default_bindings() {
            local_bindings.entry(name.to_string()).or_insert(q);
        }

        let var_names = expr.var_names();
        let mut vars = Vec::new();
        for name in var_names {
            if let Some(q) = local_bindings.get(name) {
                vars.push(q.clone());
            } else {
                self.error = Some(format!("Unknown variable '{}'", name));
                return;
            }
        }

        match expr.eval(&vars) {
            Ok(q) => {
                if let Some(err) = q.error {
                    self.error = Some(format!("Dimension error: {}", err));
                } else {
                    self.result = Some(q);
                }
            }
            Err(e) => {
                self.error = Some(format!("Eval error: {}", e));
            }
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Name:");
            if ui.add(egui::TextEdit::singleline(&mut self.label).desired_width(70.0)).changed() {
                changed = true;
            }
            ui.label(":=");
            if ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .desired_width(f32::INFINITY)
                    .hint_text("e.g. 5 m / 2 s"),
            ).changed() {
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("=");
            if let Some(q) = &self.result {
                ui.monospace(format!("{}", q));
            } else if let Some(err) = &self.error {
                ui.colored_label(egui::Color32::RED, err);
            } else {
                ui.weak("(empty)");
            }
        });

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::{Dim, Quantity};

    fn h(pairs: &[(&str, Quantity)]) -> HashMap<String, Quantity> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    fn q(v: f64) -> Quantity {
        Quantity::new(v, Dim::ZERO)
    }

    #[test]
    fn simple_arithmetic() {
        let mut e = Expression::new("a", "2 + 3 * 4");
        e.evaluate(&h(&[]));
        assert_eq!(e.result.unwrap().value, 14.0);
        assert!(e.error.is_none());
    }

    #[test]
    fn empty_input_gives_no_result_no_error() {
        let mut e = Expression::new("a", "");
        e.evaluate(&h(&[]));
        assert!(e.result.is_none());
        assert!(e.error.is_none());
    }

    #[test]
    fn whitespace_only_input_is_treated_as_empty() {
        let mut e = Expression::new("a", "   \t  ");
        e.evaluate(&h(&[]));
        assert!(e.result.is_none());
        assert!(e.error.is_none());
    }

    #[test]
    fn parse_error_sets_error_not_result() {
        let mut e = Expression::new("a", "2 +");
        e.evaluate(&h(&[]));
        assert!(e.result.is_none());
        assert!(e.error.as_ref().unwrap().contains("Parse error"));
    }

    #[test]
    fn unknown_variable_sets_eval_error() {
        let mut e = Expression::new("a", "z + 1");
        e.evaluate(&h(&[]));
        assert!(e.result.is_none());
        assert!(e.error.as_ref().unwrap().contains("Unknown variable"));
    }

    #[test]
    fn bindings_resolve() {
        let mut e = Expression::new("b", "a * 2");
        e.evaluate(&h(&[("a", q(5.0))]));
        assert_eq!(e.result.unwrap().value, 10.0);
        assert!(e.error.is_none());
    }

    #[test]
    fn multiple_bindings_resolve() {
        let mut e = Expression::new("c", "a + b");
        e.evaluate(&h(&[("a", q(3.0)), ("b", q(4.0))]));
        assert_eq!(e.result.unwrap().value, 7.0);
    }

    #[test]
    fn eval_clears_previous_error() {
        let mut e = Expression::new("a", "2 +");
        e.evaluate(&h(&[]));
        assert!(e.error.is_some());
        e.input = "2 + 2".to_owned();
        e.evaluate(&h(&[]));
        assert_eq!(e.result.unwrap().value, 4.0);
        assert!(e.error.is_none());
    }

    #[test]
    fn eval_clears_previous_result() {
        let mut e = Expression::new("a", "1 + 1");
        e.evaluate(&h(&[]));
        assert_eq!(e.result.as_ref().unwrap().value, 2.0);
        e.input = "bad +".to_owned();
        e.evaluate(&h(&[]));
        assert!(e.result.is_none());
        assert!(e.error.is_some());
    }

    #[test]
    fn unit_arithmetic() {
        let mut e = Expression::new("v", "5 m / (2 s)");
        e.evaluate(&h(&[]));
        let res = e.result.unwrap();
        assert_eq!(res.value, 2.5);
        assert_eq!(res.dim, Dim::LENGTH / Dim::TIME);
    }

    #[test]
    fn dimension_error() {
        let mut e = Expression::new("bad", "2 + 3 m");
        e.evaluate(&h(&[]));
        assert!(e.result.is_none());
        assert!(e.error.unwrap().contains("Dimension mismatch"));
    }

    #[test]
    fn sin_function() {
        let mut e = Expression::new("v", "sin(pi / 2)");
        e.evaluate(&h(&[]));
        assert_eq!(e.result.unwrap().value, 1.0);
        assert!(e.error.is_none());
    }
}