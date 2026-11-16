use aaiyyan_core::worksheet::BlockKind;
use aaiyyan_core::Session;
use aaiyyan_evaluator::Value;
use aaiyyan_project::{Backend, CalcMode, Project};
use aaiyyan_units::Dimension;
use std::time::Duration;

fn equation(source: &str) -> BlockKind {
    BlockKind::Equation { source: source.to_string() }
}

#[test]
fn mvp_beam_deflection_end_to_end() {
    let mut session = Session::new();
    session.project.settings.mode = CalcMode::Manual;
    session.project.settings.backend = Backend::Native;
    assert!(session.wait_for_idle(Duration::from_secs(30)), "calculation did not finish");

    let delta = session.result("delta").expect("delta must be defined");
    match &delta.value {
        Value::Quantity(q) => {
            assert!((q.si - 0.0059086).abs() < 1e-6, "delta SI = {}", q.si);
            assert!(q.dim().compatible(&Dimension::length()), "delta must be a length");
            assert_eq!(q.unit.name, "m");
        }
        other => panic!("expected a quantity, got {}", other.type_name()),
    }

    let mm = session.result("delta_mm").expect("delta_mm must be defined");
    match &mm.value {
        Value::Quantity(q) => assert!((q.value() - 5.9086).abs() < 1e-3),
        other => panic!("expected a quantity, got {}", other.type_name()),
    }

    assert!(session.generated.contains("ureg."), "generated Python must carry units");
    assert!(session.generated.contains("delta ="), "generated Python must define delta");
    assert!(session.block_errors.is_empty(), "unexpected errors: {:?}", session.block_errors);

    let script = session.export_python();
    assert!(script.contains("import pint"));
    assert!(script.contains("__main__"));
}

#[test]
fn plot_sampling_is_connected_to_variables() {
    let mut session = Session::new();
    session.project.settings.backend = Backend::Native;
    assert!(session.wait_for_idle(Duration::from_secs(30)));
    let spec = session.worksheet().blocks.iter().find_map(|b| match &b.kind {
        BlockKind::Plot2D(spec) => Some(spec.clone()),
        _ => None,
    }).expect("demo has a 2D plot");
    let series = session.sample_plot2d(&spec).expect("sampling works");
    assert!(!series.is_empty());
    assert!(series[0].points.len() > 10, "plot must contain sampled points");
}

#[test]
fn incremental_recalculation_updates_dependents() {
    let mut session = Session::from_project(Project::empty("incremental"), None);
    session.project.settings.mode = CalcMode::Manual;
    session.project.settings.backend = Backend::Native;
    session.insert_block(equation("a := 5"), None);
    session.insert_block(equation("b := 10"), None);
    session.insert_block(equation("c := a + b"), None);
    session.insert_block(equation("d := c * 2"), None);
    assert!(session.wait_for_idle(Duration::from_secs(30)));
    assert_eq!(session.result("d").unwrap().value.as_f64(), Some(30.0));

    let first = session.worksheet().blocks.first().unwrap().id;
    session.update_source(first, "a := 7".to_string());
    assert!(session.wait_for_idle(Duration::from_secs(30)));
    assert_eq!(session.result("c").unwrap().value.as_f64(), Some(17.0));
    assert_eq!(session.result("d").unwrap().value.as_f64(), Some(34.0));
}

#[test]
fn project_save_load_and_reports() {
    let dir = aaiyyan_integration_tests::temp_dir("reports");
    let mut session = Session::new();
    session.project.settings.backend = Backend::Native;
    assert!(session.wait_for_idle(Duration::from_secs(30)));

    let path = dir.join("project.aaiyyan.json");
    session.save(&path).unwrap();
    let mut reopened = Session::open(&path).unwrap();
    assert!(reopened.wait_for_idle(Duration::from_secs(30)));
    assert!(reopened.result("delta").is_some());

    let html = dir.join("report.html");
    session.export_html(&html).unwrap();
    let text = std::fs::read_to_string(&html).unwrap();
    assert!(text.contains("delta"));
    assert!(text.contains("<svg"));

    let typst_dir = dir.join("typst");
    let typ = session.export_typst(&typst_dir).unwrap();
    assert!(typ.exists());

    let py = dir.join("worksheet.py");
    std::fs::write(&py, session.export_python()).unwrap();
    assert!(std::fs::read_to_string(&py).unwrap().contains("delta"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn circular_dependency_is_reported_not_hung() {
    let mut session = Session::from_project(Project::empty("cycle"), None);
    session.project.settings.backend = Backend::Native;
    session.insert_block(equation("x := y + 1"), None);
    session.insert_block(equation("y := x + 1"), None);
    assert!(session.wait_for_idle(Duration::from_secs(15)));
    assert!(!session.block_errors.is_empty(), "cycle must produce an error");
}
