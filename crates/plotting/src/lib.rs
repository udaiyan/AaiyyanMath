//! Plot specifications and native sampling. Rendering lives in the GUI crate;
//! this crate contains no UI code (spec sections 4 and 16).
use aaiyyan_evaluator::{Environment, EvalLimits, Evaluator, Value};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeriesKind { Line, Scatter, Parametric, Polar, Histogram }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeriesSpec {
    pub expr: String,
    pub variable: String,
    pub label: String,
    pub kind: SeriesKind,
    pub color: Option<[u8; 3]>,
}

impl SeriesSpec {
    pub fn line(expr: &str, variable: &str, label: &str) -> SeriesSpec {
        SeriesSpec {
            expr: expr.into(), variable: variable.into(), label: label.into(),
            kind: SeriesKind::Line, color: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plot2DSpec {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub x_min: f64,
    pub x_max: f64,
    pub samples: usize,
    pub show_legend: bool,
    pub series: Vec<SeriesSpec>,
}

impl Default for Plot2DSpec {
    fn default() -> Self {
        Plot2DSpec {
            title: "Plot".into(), x_label: "x".into(), y_label: "y".into(),
            x_min: 0.0, x_max: 10.0, samples: 400, show_legend: true,
            series: vec![SeriesSpec::line("sin(x)", "x", "sin(x)")],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceKind { Surface, Scatter, VectorField }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plot3DSpec {
    pub title: String,
    pub expr: String,
    pub x_var: String,
    pub y_var: String,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub samples: usize,
    pub kind: SurfaceKind,
}

impl Default for Plot3DSpec {
    fn default() -> Self {
        Plot3DSpec {
            title: "Surface".into(), expr: "sin(x)*cos(y)".into(),
            x_var: "x".into(), y_var: "y".into(),
            x_min: -3.0, x_max: 3.0, y_min: -3.0, y_max: 3.0, samples: 40,
            kind: SurfaceKind::Surface,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SeriesData {
    pub label: String,
    pub points: Vec<[f64; 2]>,
    pub kind: SeriesKind,
    pub color: Option<[u8; 3]>,
}

impl SeriesData {
    pub fn is_empty(&self) -> bool { self.points.is_empty() }
}

#[derive(Clone, Debug, Default)]
pub struct SurfaceData {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub z: Vec<Vec<f64>>,
    pub points: Vec<[f64; 3]>,
    pub vectors: Vec<([f64; 3], [f64; 3])>,
}

fn plot_value(v: &Value) -> Option<f64> {
    match v {
        Value::Scalar(x) => Some(*x),
        Value::Quantity(q) => Some(q.si),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

pub fn sample_2d(spec: &Plot2DSpec, env: &Environment) -> Result<Vec<SeriesData>, String> {
    let mut out = Vec::new();
    let n = spec.samples.clamp(2, 20_000);
    for series in &spec.series {
        let expr = aaiyyan_parser::parse_expression(&series.expr).map_err(|e| e.message)?;
        let mut ev = Evaluator::new(EvalLimits {
            max_steps: 5_000_000, timeout: Some(Duration::from_secs(4)), cancel: None,
        });
        let mut local = env.clone();
        let mut data = SeriesData {
            label: if series.label.is_empty() { series.expr.clone() } else { series.label.clone() },
            points: Vec::new(),
            kind: series.kind,
            color: series.color,
        };
        match series.kind {
            SeriesKind::Histogram => {
                local.define_global(series.variable.clone(), Value::Scalar(0.0));
                if let Ok(Value::Vector(values)) = ev.eval(&expr, &mut local) {
                    let bins = 20usize;
                    let lo = values.iter().cloned().fold(f64::INFINITY, f64::min);
                    let hi = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    if hi > lo && !values.is_empty() {
                        let mut counts = vec![0f64; bins];
                        for v in values {
                            let idx = (((v - lo) / (hi - lo)) * (bins as f64 - 1.0)).round() as usize;
                            counts[idx.min(bins - 1)] += 1.0;
                        }
                        for (i, c) in counts.iter().enumerate() {
                            let x = lo + (hi - lo) * (i as f64) / (bins as f64 - 1.0);
                            data.points.push([x, *c]);
                        }
                    }
                }
            }
            _ => {
                for i in 0..n {
                    let t = spec.x_min + (spec.x_max - spec.x_min) * (i as f64) / ((n - 1) as f64);
                    local.define_global(series.variable.clone(), Value::Scalar(t));
                    let value = match ev.eval(&expr, &mut local) { Ok(v) => v, Err(_) => continue };
                    match series.kind {
                        SeriesKind::Polar => {
                            if let Some(r) = plot_value(&value) {
                                data.points.push([r * t.cos(), r * t.sin()]);
                            }
                        }
                        SeriesKind::Parametric => {
                            if let Value::Vector(v) = value {
                                if v.len() >= 2 { data.points.push([v[0], v[1]]); }
                            }
                        }
                        _ => {
                            if let Some(y) = plot_value(&value) {
                                if y.is_finite() { data.points.push([t, y]); }
                            }
                        }
                    }
                }
            }
        }
        out.push(data);
    }
    Ok(out)
}

pub fn sample_3d(spec: &Plot3DSpec, env: &Environment) -> Result<SurfaceData, String> {
    let expr = aaiyyan_parser::parse_expression(&spec.expr).map_err(|e| e.message)?;
    let mut ev = Evaluator::new(EvalLimits {
        max_steps: 5_000_000, timeout: Some(Duration::from_secs(4)), cancel: None,
    });
    let mut local = env.clone();
    let n = spec.samples.clamp(2, 200);
    let mut data = SurfaceData::default();
    for i in 0..n {
        data.xs.push(spec.x_min + (spec.x_max - spec.x_min) * (i as f64) / ((n - 1) as f64));
    }
    for j in 0..n {
        data.ys.push(spec.y_min + (spec.y_max - spec.y_min) * (j as f64) / ((n - 1) as f64));
    }
    for j in 0..n {
        let mut row = Vec::with_capacity(n);
        for i in 0..n {
            local.define_global(spec.x_var.clone(), Value::Scalar(data.xs[i]));
            local.define_global(spec.y_var.clone(), Value::Scalar(data.ys[j]));
            match ev.eval(&expr, &mut local) {
                Ok(Value::Vector(v)) if v.len() >= 2 && spec.kind == SurfaceKind::VectorField => {
                    data.vectors.push(([data.xs[i], data.ys[j], 0.0], [v[0], v[1], 0.0]));
                    row.push(0.0);
                }
                Ok(value) => {
                    let z = plot_value(&value).unwrap_or(f64::NAN);
                    if spec.kind == SurfaceKind::Scatter { data.points.push([data.xs[i], data.ys[j], z]); }
                    row.push(z);
                }
                Err(_) => row.push(f64::NAN),
            }
        }
        data.z.push(row);
    }
    Ok(data)
}

/// Offline SVG export used by HTML/Typst reports.
pub fn series_to_svg(series: &[SeriesData], width: f64, height: f64, title: &str) -> String {
    let mut all: Vec<[f64; 2]> = series.iter().flat_map(|s| s.points.iter().copied()).collect();
    if all.is_empty() { all.push([0.0, 0.0]); all.push([1.0, 1.0]); }
    let xmin = all.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let xmax = all.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
    let ymin = all.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let ymax = all.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
    let dx = (xmax - xmin).max(1e-12);
    let dy = (ymax - ymin).max(1e-12);
    let pad = 40.0;
    let sx = |x: f64| pad + (x - xmin) / dx * (width - 2.0 * pad);
    let sy = |y: f64| height - pad - (y - ymin) / dy * (height - 2.0 * pad);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    out.push_str(&format!("<rect width=\"{width}\" height=\"{height}\" fill=\"white\"/>\n"));
    out.push_str(&format!(
        "<text x=\"{pad}\" y=\"20\" font-size=\"14\" font-family=\"sans-serif\">{}</text>\n", escape_xml(title)
    ));
    out.push_str(&format!(
        "<line x1=\"{pad}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\"/>\n", height - pad, width - pad, height - pad
    ));
    out.push_str(&format!(
        "<line x1=\"{pad}\" y1=\"{pad}\" x2=\"{pad}\" y2=\"{}\" stroke=\"#888\"/>\n", height - pad
    ));
    let palette = ["#1f77b4", "#d62728", "#2ca02c", "#9467bd", "#ff7f0e"];
    for (i, s) in series.iter().enumerate() {
        if s.points.is_empty() { continue; }
        let pts: Vec<String> = s.points.iter().map(|p| format!("{:.2},{:.2}", sx(p[0]), sy(p[1]))).collect();
        out.push_str(&format!(
            "<polyline fill=\"none\" stroke=\"{}\" stroke-width=\"1.6\" points=\"{}\"/>\n",
            palette[i % palette.len()], pts.join(" ")
        ));
    }
    out.push_str("</svg>\n");
    out
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
