#!/usr/bin/env python3
"""Generate the AaiyyanMath Rust egui scaffold in the current directory."""

import os
import sys

# Write everything relative to CWD (i.e. D:\darren\git\AaiyyanMath)
ROOT = ""
CRATE = "aaiyyanmath"
TITLE = "AaiyyanMath"

# Guard rails: refuse to clobber an existing Cargo.toml
if os.path.exists(os.path.join(ROOT, "Cargo.toml")):
    print("ERROR: Cargo.toml already exists in", os.path.abspath(ROOT))
    print("       Move or delete it before running this script.")
    sys.exit(1)


def w(rel, text):
    path = os.path.join(ROOT, rel)
    d = os.path.dirname(path)
    if d:
        os.makedirs(d, exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print("  wrote", os.path.relpath(path, os.getcwd()),
          "(" + str(len(text)) + " bytes)")


def main():
    print("Python:", sys.version.split()[0])
    print("Target :", os.path.abspath(ROOT or "."))
    print()

    w("Cargo.toml", '''[package]
name = "aaiyyanmath"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.29"
egui = "0.29"
egui_plot = "0.29"
meval = "0.2"
uom = "0.35"
''')

    w("src/main.rs", '''mod app;
mod expression;
mod plot;
mod units;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_title("AaiyyanMath"),
        ..Default::default()
    };
    eframe::run_native(
        "AaiyyanMath",
        options,
        Box::new(|_cc| Ok(Box::new(app::MathcadApp::default()))),
    )
}
''')

    w("src/app.rs", '''use eframe::egui;

pub struct MathcadApp {
    input: String,
    result: String,
}

impl Default for MathcadApp {
    fn default() -> Self {
        Self {
            input: "2 + 3 * 4".to_owned(),
            result: String::new(),
        }
    }
}

impl eframe::App for MathcadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("AaiyyanMath - expression evaluator");

            ui.horizontal(|ui| {
                ui.label("Expression:");
                ui.text_edit_singleline(&mut self.input);
            });

            if ui.button("Calculate").clicked() {
                self.result = evaluate_expression(&self.input);
            }

            ui.separator();
            ui.label("Result:");
            ui.monospace(&self.result);

            // ---- Later milestones (uncomment as you go) ----
            // ui.collapsing("Second expression", |ui| {
            //     expression::render_second_expression(ui);
            // });
            // ui.collapsing("Plot", |ui| {
            //     plot::render_plot(ui);
            // });
            // ui.collapsing("Units", |ui| {
            //     units::render_unit_controls(ui);
            // });
        });
    }
}

fn evaluate_expression(input: &str) -> String {
    match meval::eval_str(input) {
        Ok(v) => format!("{}", v),
        Err(e) => format!("Error: {}", e),
    }
}
''')

    w("src/expression.rs", '''// Placeholder for second expression pane.
// Milestone 2: second input and result, then combine expressions.
//
// use eframe::egui;
// pub fn render_second_expression(_ui: &mut egui::Ui) {
//     // TODO
// }
''')

    w("src/plot.rs", '''// Placeholder for egui_plot integration.
// Milestone 3: line plot of a function.
//
// use eframe::egui;
// use egui_plot::{Line, Plot, PlotPoints};
// pub fn render_plot(ui: &mut egui::Ui) {
//     let pts: PlotPoints = (0..500)
//         .map(|i| { let x = i as f64 * 0.02; [x, x.sin()] })
//         .collect();
//     Plot::new("plot").show(ui, |p| p.line(Line::new(pts)));
// }
''')

    w("src/units.rs", '''// Placeholder for uom unit support.
// Milestone 4: attach physical units to results.
//
// use uom::si::f64::*;
// use uom::si::length::meter;
// use uom::si::time::second;
// pub fn demo_units() {
//     let l = Length::new::<meter>(5.0);
//     let t = Time::new::<second>(2.0);
//     let _v = l / t;
// }
''')

    # README assembled line-by-line (no triple quotes, no fences).
    lines = [
        "# AaiyyanMath",
        "",
        "A Mathcad-like math app prototype built with Rust, egui and meval.",
        "",
        "## Run",
        "",
        "    cargo run",
        "",
        "## Milestones",
        "",
        "1. Expression evaluator (done). Single input, meval::eval_str result.",
        "2. Second expression. Uncomment scaffolding in src/app.rs.",
        "3. Plot. Implement src/plot.rs with egui_plot.",
        "4. Units. Implement src/units.rs with uom.",
        "",
        "## Swapping in a CAS",
        "",
        "meval handles numeric expressions. For symbolic CAS features",
        "(calculus, simplification, equation solving) swap the",
        "evaluate_expression function in src/app.rs for Symbolica or symplex:",
        "",
        "    cargo add symbolica",
        "    cargo add symplex",
        "",
        "## Long-term architecture",
        "",
        "The modular layout (app, expression, plot, units) is designed to",
        "grow into a folder-based Mathcad/MATLAB-style workspace with",
        "separate worksheets, a Python-code pane, and domain modules",
        "(calculus, electronics, etc.).",
        "",
    ]
    w("README.md", "\n".join(lines))

    # .gitignore for Rust
    w(".gitignore", "/target\nCargo.lock\n*.rs.bk\n")

    print()
    print("Done.")
    print("  cargo run")


if __name__ == "__main__":
    main()