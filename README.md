# AaiyyanMath

A Mathcad-like math app prototype built with Rust, egui and meval.

## Run

    cargo run

## Milestones

1. Expression evaluator (done). Single input, meval::eval_str result.
2. Second expression. Uncomment scaffolding in src/app.rs.
3. Plot. Implement src/plot.rs with egui_plot.
4. Units. Implement src/units.rs with uom.

## Swapping in a CAS

meval handles numeric expressions. For symbolic CAS features
(calculus, simplification, equation solving) swap the
evaluate_expression function in src/app.rs for Symbolica or symplex:

    cargo add symbolica
    cargo add symplex

## Long-term architecture

The modular layout (app, expression, plot, units) is designed to
grow into a folder-based Mathcad/MATLAB-style workspace with
separate worksheets, a Python-code pane, and domain modules
(calculus, electronics, etc.).
