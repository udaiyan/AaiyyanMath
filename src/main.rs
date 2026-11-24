mod app;
mod expression;
mod plot;
mod python; // <-- ADD THIS
mod quantity;
mod worksheet;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0]) // Made window wider for the code pane
            .with_title("AaiyyanMath"),
        ..Default::default()
    };
    eframe::run_native(
        "AaiyyanMath",
        options,
        Box::new(|_cc| Ok(Box::new(app::MathcadApp::default()))),
    )
}