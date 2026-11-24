use eframe::egui;
use egui_plot::{Legend, Line, Plot};
use exmex::{Express, FlatEx};
use std::collections::HashMap;
use crate::quantity::{Quantity, QuantityOpsFactory};

pub struct PlotPane {
    pub var: String,
    pub expr: String,
    pub x_min: f64,
    pub x_max: f64,
    pub samples: usize,
    last_key: Option<PlotKey>,
    points: Vec<[f64; 2]>,
    error: Option<String>,
}

#[derive(Clone, PartialEq)]
struct PlotKey {
    var: String,
    expr: String,
    x_min: f64,
    x_max: f64,
    samples: usize,
    bindings: Vec<(String, Quantity)>,
}

impl Default for PlotPane {
    fn default() -> Self {
        Self {
            var: "x".to_owned(),
            expr: "sin(x)".to_owned(),
            x_min: -6.28,
            x_max: 6.28,
            samples: 500,
            last_key: None,
            points: Vec::new(),
            error: None,
        }
    }
}

impl PlotPane {
    pub fn render(&mut self, ui: &mut egui::Ui, bindings: &HashMap<String, Quantity>) {
        ui.horizontal(|ui| {
            ui.label("y =");
            ui.add(egui::TextEdit::singleline(&mut self.expr).desired_width(220.0).hint_text("e.g. sin(x) + a"));
            ui.label("in");
            ui.add(egui::TextEdit::singleline(&mut self.var).desired_width(40.0));
        });
        ui.horizontal(|ui| {
            ui.label("x from");
            ui.add(egui::DragValue::new(&mut self.x_min).speed(0.1));
            ui.label("to");
            ui.add(egui::DragValue::new(&mut self.x_max).speed(0.1));
            ui.label("samples");
            ui.add(egui::DragValue::new(&mut self.samples).range(10..=10_000));
        });

        let mut bkeys: Vec<(String, Quantity)> = bindings.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        bkeys.sort_by(|a, b| a.0.cmp(&b.0));

        let key = PlotKey {
            var: self.var.clone(),
            expr: self.expr.clone(),
            x_min: self.x_min,
            x_max: self.x_max,
            samples: self.samples,
            bindings: bkeys,
        };

        if self.last_key.as_ref() != Some(&key) {
            self.rebuild(&key);
            self.last_key = Some(key);
        }

        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::RED, err);
        }

        if self.points.len() >= 2 {
            let line = Line::new(self.points.clone()).name(self.expr.clone());
            Plot::new("aaiyyan_plot")
                .height(300.0)
                .allow_scroll(false)
                .legend(Legend::default())
                .show(ui, |plot_ui| {
                    plot_ui.line(line);
                });
        }
    }

    fn rebuild(&mut self, key: &PlotKey) {
        self.points.clear();
        self.error = None;

        let processed = crate::quantity::preprocess(&key.expr);
        let expr: FlatEx<Quantity, QuantityOpsFactory> = match FlatEx::<Quantity, QuantityOpsFactory>::parse(&processed) {
            Ok(e) => e,
            Err(e) => {
                self.error = Some(format!("Parse error: {}", e));
                return;
            }
        };

        let mut local_bindings: HashMap<String, Quantity> = key.bindings.iter().cloned().collect();
        for (name, q) in crate::quantity::default_bindings() {
            local_bindings.entry(name.to_string()).or_insert(q);
        }

        let n = key.samples.max(2);
        let var_names = expr.var_names();
        let mut first_err: Option<String> = None;
        let mut points: Vec<[f64; 2]> = Vec::with_capacity(n);

        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            let x = key.x_min + (key.x_max - key.x_min) * t;

            let mut vars = Vec::new();
            for name in var_names {
                if name == &key.var {
                    vars.push(Quantity::new(x, crate::quantity::Dim::ZERO));
                } else if let Some(q) = local_bindings.get(name) {
                    vars.push(q.clone());
                } else {
                    if first_err.is_none() {
                        first_err = Some(format!("Unknown variable '{}'", name));
                    }
                    vars.push(Quantity::new(f64::NAN, crate::quantity::Dim::ZERO));
                }
            }

            match expr.eval(&vars) {
                Ok(q) if q.error.is_none() && q.value.is_finite() => points.push([x, q.value]),
                Ok(q) => {
                    if first_err.is_none() {
                        if let Some(err) = q.error {
                            first_err = Some(format!("Dimension error: {}", err));
                        }
                    }
                }
                Err(e) => {
                    if first_err.is_none() {
                        first_err = Some(format!("Eval error: {}", e));
                    }
                }
            }
        }

        if points.len() < 2 {
            self.error = first_err.or_else(|| Some("not enough valid points".to_owned()));
        }
        self.points = points;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(
        expr: &str,
        x_min: f64,
        x_max: f64,
        samples: usize,
        bindings: &[(&str, f64)],
    ) -> PlotKey {
        PlotKey {
            var: "x".to_owned(),
            expr: expr.to_owned(),
            x_min,
            x_max,
            samples,
            bindings: bindings.iter().map(|(k, v)| (k.to_string(), Quantity::new(*v, crate::quantity::Dim::ZERO))).collect(),
        }
    }

    #[test]
    fn linear_plot_is_linear() {
        let mut p = PlotPane::default();
        let key = make_key("x", 0.0, 10.0, 11, &[]);
        p.rebuild(&key);
        assert!(p.error.is_none());
        assert_eq!(p.points.len(), 11);
        assert!((p.points[0][0] - 0.0).abs() < 1e-9);
        assert!((p.points[0][1] - 0.0).abs() < 1e-9);
        assert!((p.points[5][0] - 5.0).abs() < 1e-9);
        assert!((p.points[5][1] - 5.0).abs() < 1e-9);
    }

    #[test]
    fn plot_uses_bindings() {
        let mut p = PlotPane::default();
        let key = make_key("a * x", 0.0, 10.0, 11, &[("a", 2.0)]);
        p.rebuild(&key);
        assert!(p.error.is_none());
        assert_eq!(p.points.len(), 11);
        assert!((p.points[10][1] - 20.0).abs() < 1e-9);
    }

    #[test]
    fn plot_sin_matches_math() {
        let mut p = PlotPane::default();
        let key = make_key("sin(x)", 0.0, std::f64::consts::PI, 5, &[]);
        p.rebuild(&key);
        assert!(p.error.is_none(), "Expected no error, got: {:?}", p.error);
        assert_eq!(p.points.len(), 5);
        assert!((p.points[0][1] - 0.0).abs() < 1e-9);
        assert!((p.points[2][1] - 1.0).abs() < 1e-9);
        assert!((p.points[4][1] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn plot_parse_error_sets_error_and_no_points() {
        let mut p = PlotPane::default();
        let key = make_key("2 +", 0.0, 1.0, 10, &[]);
        p.rebuild(&key);
        assert!(p.error.as_ref().unwrap().contains("Parse error"));
        assert!(p.points.is_empty());
    }

    #[test]
    fn plot_filters_non_finite_points() {
        let mut p = PlotPane::default();
        let key = make_key("1 / x", -1.0, 1.0, 3, &[]);
        p.rebuild(&key);
        assert_eq!(p.points.len(), 2);
        assert!(p.error.is_none());
    }

    #[test]
    fn plot_var_can_be_renamed() {
        let mut p = PlotPane::default();
        let mut key = make_key("t * 2", 0.0, 1.0, 2, &[]);
        key.var = "t".to_owned();
        p.rebuild(&key);
        assert!(p.error.is_none());
        assert!((p.points[1][1] - 2.0).abs() < 1e-9);
    }
}