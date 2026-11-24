use eframe::egui;
use meval::{Context, Expr};
use std::collections::HashMap;

/// A single Mathcad-style expression: `name := input`, yielding a numeric result.
#[derive(Default)]
pub struct Expression {
    pub label: String,
    pub input: String,
    pub result: Option<f64>,
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

    /// Re-evaluate using `bindings` as the variable environment.
    pub fn evaluate(&mut self, bindings: &HashMap<String, f64>) {
        self.result = None;
        self.error = None;

        if self.input.trim().is_empty() {
            return;
        }

        let expr: Expr = match self.input.parse() {
            Ok(e) => e,
            Err(e) => {
                self.error = Some(format!("Parse error: {}", e));
                return;
            }
        };

        let mut ctx = Context::new();
        for (name, value) in bindings {
            ctx.var(name.as_str(), *value);
        }

        match expr.eval_with_context(&ctx) {
            Ok(v) => self.result = Some(v),
            Err(e) => self.error = Some(format!("Eval error: {}", e)),
        }
    }

    /// Render the pane. Returns `true` if any field changed this frame.
    pub fn render(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Name:");
            if ui
                .add(egui::TextEdit::singleline(&mut self.label).desired_width(70.0))
                .changed()
            {
                changed = true;
            }
            ui.label(":=");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.input)
                        .desired_width(f32::INFINITY)
                        .hint_text("e.g. 2 + 3 * 4"),
                )
                .changed()
            {
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("=");
            if let Some(v) = self.result {
                ui.monospace(format!("{}", v));
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

    fn h(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn simple_arithmetic() {
        let mut e = Expression::new("a", "2 + 3 * 4");
        e.evaluate(&h(&[]));
        assert_eq!(e.result, Some(14.0));
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
        let mut e = Expression::new("a", "x + 1");
        e.evaluate(&h(&[]));
        assert!(e.result.is_none());
        assert!(e.error.as_ref().unwrap().contains("Eval error"));
    }

    #[test]
    fn bindings_resolve() {
        let mut e = Expression::new("b", "a * 2");
        e.evaluate(&h(&[("a", 5.0)]));
        assert_eq!(e.result, Some(10.0));
        assert!(e.error.is_none());
    }

    #[test]
    fn multiple_bindings_resolve() {
        let mut e = Expression::new("c", "a + b");
        e.evaluate(&h(&[("a", 3.0), ("b", 4.0)]));
        assert_eq!(e.result, Some(7.0));
    }

    #[test]
    fn eval_clears_previous_error() {
        let mut e = Expression::new("a", "2 +");
        e.evaluate(&h(&[]));
        assert!(e.error.is_some());
        e.input = "2 + 2".to_owned();
        e.evaluate(&h(&[]));
        assert_eq!(e.result, Some(4.0));
        assert!(e.error.is_none());
    }

    #[test]
    fn eval_clears_previous_result() {
        let mut e = Expression::new("a", "1 + 1");
        e.evaluate(&h(&[]));
        assert_eq!(e.result, Some(2.0));
        e.input = "bad +".to_owned();
        e.evaluate(&h(&[]));
        assert!(e.result.is_none());
        assert!(e.error.is_some());
    }
}