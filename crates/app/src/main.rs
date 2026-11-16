mod app;

use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut project_path: Option<PathBuf> = None;
    let mut backend: Option<aaiyyan_project::Backend> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" | "-p" => { project_path = args.next().map(PathBuf::from); }
            "--backend" => match args.next().as_deref() {
                Some("native") => backend = Some(aaiyyan_project::Backend::Native),
                Some("python") => backend = Some(aaiyyan_project::Backend::Python),
                _ => backend = Some(aaiyyan_project::Backend::Auto),
            },
            "--help" | "-h" => {
                println!("AaiyyanMath - native engineering mathematics environment");
                println!("  --project <file>   open a project at startup");
                println!("  --backend <mode>   native | python | auto");
                return Ok(());
            }
            other => { project_path = Some(PathBuf::from(other)); }
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("AaiyyanMath"),
        ..Default::default()
    };

    eframe::run_native(
        "AaiyyanMath",
        options,
        Box::new(move |cc| Ok(Box::new(app::AaiyyanApp::new(cc, project_path, backend)))),
    )
}
