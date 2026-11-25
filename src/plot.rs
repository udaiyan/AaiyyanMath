use eframe::egui;
use egui_plot::{Legend, Line, Plot};
use exmex::{Express, FlatEx};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::quantity::{Quantity, QuantityOpsFactory};

#[derive(Serialize, Deserialize)]
pub struct PlotPane {
    pub var: String,
    pub expr: String,
    pub x_min: f64,
    pub x_max: f64,
    pub samples: usize,
    #[serde(skip)]
    last_key: Option<PlotKey>,
    #[serde(skip)]
    points: Vec<[f64; 2]>,
    #[serde(skip)]
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
            ui.add(egui::TextEdit::singleline(&mut self.expr).desired_width(220.0).hint_text("e.g. deriv(x^2, x)"));
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

        let mut local_bindings: HashMap<String, Quantity> = key.bindings.iter().cloned().collect();
        for (name, q) in crate::quantity::default_bindings() {
            local_bindings.entry(name.to_string()).or_insert(q);
        }

        // deriv/integ results depend on the CURRENT value of the plot variable,
        // so calculus must be expanded per sample with the variable bound to that
        // sample. Without calculus, keep the fast path: parse once, eval per sample.
        let preprocessed = crate::quantity::preprocess(&key.expr);
        let has_calculus = preprocessed.contains("deriv(") || preprocessed.contains("integ(");

        let cached: Option<FlatEx<Quantity, QuantityOpsFactory>> = if has_calculus {
            None
        } else {
            match FlatEx::<Quantity, QuantityOpsFactory>::parse(&preprocessed) {
                Ok(e) => Some(e),
                Err(e) => {
                    self.error = Some(format!("Parse error: {}", e));
                    return;
                }
            }
        };

        let n = key.samples.max(2);
        let mut first_err: Option<String> = None;
        let mut points: Vec<[f64; 2]> = Vec::with_capacity(n);

        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            let x = key.x_min + (key.x_max - key.x_min) * t;

            // Calculus path builds a fresh expression for this sample;
            // the fast path leaves this None and reuses `cached`.
            let sample_expr: Option<FlatEx<Quantity, QuantityOpsFactory>> = if has_calculus {
                let mut sb = local_bindings.clone();
                sb.insert(key.var.clone(), Quantity::new(x, crate::quantity::Dim::ZERO));
                let processed = match crate::quantity::expand_calculus(&key.expr, &sb) {
                    Ok(p) => p,
                    Err(e) => {
                        if first_err.is_none() { first_err = Some(e); }
                        continue;
                    }
                };
                match FlatEx::<Quantity, QuantityOpsFactory>::parse(&processed) {
                    Ok(e) => Some(e),
                    Err(e) => {
                        if first_err.is_none() { first_err = Some(format!("Parse error: {}", e)); }
                        continue;
                    }
                }
            } else {
                None
            };

            let expr: &FlatEx<Quantity, QuantityOpsFactory> = match sample_expr.as_ref() {
                Some(e) => e,
                None => cached.as_ref().unwrap(),
            };

            let mut vars = Vec::new();
            for name in expr.var_names() {
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

    #[test]
    fn plot_calculus() {
        let mut p = PlotPane::default();
        let key = make_key("deriv(x^2, x)", 0.0, 10.0, 11, &[]);
        p.rebuild(&key);
        assert!(p.error.is_none(), "Expected no error, got: {:?}", p.error);
        assert_eq!(p.points.len(), 11);
        // d/dx x^2 = 2x at each sample (central difference is exact for quadratics)
        assert!((p.points[0][1] - 0.0).abs() < 1e-6);
        assert!((p.points[5][1] - 10.0).abs() < 1e-6);
        assert!((p.points[10][1] - 20.0).abs() < 1e-6);
    }

    #[test]
    fn plot_axis_variable_overrides_binding() {
        let mut p = PlotPane::default();
        let key = make_key("deriv(x^2, x)", 0.0, 4.0, 5, &[("x", 100.0)]);
        p.rebuild(&key);
        assert!(p.error.is_none());
        // the axis value wins over the worksheet binding x = 100
        assert!((p.points[4][1] - 8.0).abs() < 1e-6);
    }

    #[test]
    fn plot_integral_with_variable_bound() {
        let mut p = PlotPane::default();
        let key = make_key("integ(t^2, t, 0, x)", 0.0, 3.0, 4, &[]);
        p.rebuild(&key);
        assert!(p.error.is_none(), "Expected no error, got: {:?}", p.error);
        assert_eq!(p.points.len(), 4);
        // integral of t^2 from 0 to x = x^3/3 (Simpson is exact for quadratics)
        assert!((p.points[1][1] - 1.0 / 3.0).abs() < 1e-6);
        assert!((p.points[3][1] - 9.0).abs() < 1e-6);
    }
}