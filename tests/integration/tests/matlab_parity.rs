//! End-to-end MATLAB-parity tests through the real Session pipeline
//! (parse -> dependency graph -> dispatch -> evaluate -> codegen).
//! Numeric correctness of individual functions is covered by unit tests in
//! aaiyyan-evaluator; these tests verify the wiring and the hybrid routing.
use aaiyyan_core::Session;
use aaiyyan_evaluator::Value;
use aaiyyan_project::Project;
use aaiyyan_core::worksheet::BlockKind;

fn fresh() -> Session {
    Session::from_project(Project::empty("parity"), None)
}
fn drain(s: &mut Session) {
    s.force_recalc();
    for _ in 0..4000 {
        s.poll();
        if !s.busy && !s.stale { return; }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("recalculation did not settle");
}
fn eq(s: &mut Session, src: &str, x: f32, y: f32) -> uuid::Uuid {
    s.insert_block_at(BlockKind::Equation { source: src.into() }, x, y)
}
fn val<'a>(s: &'a Session, id: uuid::Uuid, key: &str) -> &'a Value {
    let v = s.values_for(id);
    v.iter().find(|(n, _)| n == key).map(|(_, x)| x).unwrap_or_else(|| panic!("no value for {key}"))
}
fn scalar_at(s: &Session, id: uuid::Uuid, key: &str) -> f64 {
    let v = val(s, id, key);
    match v {
        Value::Scalar(x) => *x,
        // A dimensionless result may be carried as a Quantity; both are numeric.
        Value::Quantity(q) => q.value(),
        Value::Complex { re, im } if im.abs() < 1e-12 => *re,
        other => panic!(
            "{key} is not numeric: {} = {}",
            other.type_name(),
            other.display()
        ),
    }
}
fn no_errors(s: &Session, id: uuid::Uuid) {
    let e = s.errors_for(id);
    assert!(e.is_empty(), "unexpected errors: {e:?}");
}

#[test] fn evaluation_operator_end_to_end() {
    let mut s = fresh();
    let a = eq(&mut s, "2+3=", 0.0, 0.0);
    drain(&mut s);
    no_errors(&s, a);
    assert!((scalar_at(&s, a, "_") - 5.0).abs() < 1e-9);
}

#[test] fn catalog_magic_flows_through_pipeline() {
    let mut s = fresh();
    let a = eq(&mut s, "A := magic(4)", 0.0, 0.0);
    drain(&mut s);
    no_errors(&s, a);
    match val(&s, a, "A") {
        Value::Matrix(m) => { assert!(m.rows == 4 && m.cols == 4); assert!((m.get(0, 0) - 16.0).abs() < 1e-9); }
        _ => panic!("magic(4) should be a matrix"),
    }
    assert!(s.generated.contains("aaiyyan.mcall(\"magic\""));
}

#[test] fn catalog_expm_and_kron_native_results() {
    let mut s = fresh();
    let e = eq(&mut s, "Z := expm([[0,1],[0,0]])", 0.0, 48.0);
    let k = eq(&mut s, "K := kron([[1,2]], [[1,0],[0,1]])", 0.0, 96.0);
    drain(&mut s);
    no_errors(&s, e); no_errors(&s, k);
    match val(&s, e, "Z") { Value::Matrix(m) => assert!((m.get(0, 1) - 1.0).abs() < 1e-9), _ => panic!() }
    match val(&s, k, "K") { Value::Matrix(m) => { assert_eq!((m.rows, m.cols), (2, 4)); assert!((m.get(0, 2) - 2.0).abs() < 1e-9); } _ => panic!() }
    assert!(s.generated.contains("aaiyyan.mcall(\"expm\""));
    assert!(s.generated.contains("aaiyyan.mcall(\"kron\""));
}

#[test] fn colon_and_linspace_route_to_valid_python() {
    let mut s = fresh();
    let c = eq(&mut s, "v := colon(0,2,10)", 0.0, 0.0);
    let l = eq(&mut s, "w := linspace(0,1,5)", 0.0, 48.0);
    drain(&mut s);
    no_errors(&s, c);
    no_errors(&s, l);

    match val(&s, c, "v") {
        Value::Vector(a) => assert_eq!(a.len(), 6, "colon(0,2,10) should give 6 points"),
        other => panic!("colon result was {}", other.type_name()),
    }
    match val(&s, l, "w") {
        Value::Vector(a) => assert_eq!(a.len(), 5, "linspace(0,1,5) should give 5 points"),
        other => panic!("linspace result was {}", other.type_name()),
    }

    // colon has no dedicated emitter arm, so it routes through the catalog.
    assert!(
        s.generated.contains("aaiyyan.mcall(\"colon\""),
        "colon did not route via mcall:\n{}",
        s.generated
    );

    // linspace has a dedicated NumPy arm; either routing is valid, runnable Python.
    let linspace_ok = s.generated.contains("np.linspace")
        || s.generated.contains("aaiyyan.mcall(\"linspace\"")
        || s.generated.contains("linspace(");
    assert!(linspace_ok, "no linspace call in generated Python:\n{}", s.generated);
}

#[test] fn units_pass_dimensional_validation_end_to_end() {
    let mut s = fresh();
    let _e = eq(&mut s, "E := 210 GPa", 0.0, 0.0);
    let _l = eq(&mut s, "L := 3 m", 0.0, 48.0);
    let h = eq(&mut s, "half := L / 2", 0.0, 96.0);
    drain(&mut s);
    no_errors(&s, h);
    assert!(s.generated.contains("GPa"));
}

#[test] fn python_routed_names_report_routing_in_native_build() {
    // Without the `python` feature these must NOT crash the worksheet; they
    // surface as a clear "routed to embedded Python" error on the region.
    let mut s = fresh();
    let a = eq(&mut s, "u := lu([[1,2],[3,4]])", 0.0, 0.0);
    drain(&mut s);
    let errs = s.errors_for(a);
    assert!(!errs.is_empty(), "lu should require the Python backend in a native build");
    assert!(errs.iter().any(|e| e.to_lowercase().contains("python")), "got {errs:?}");
    assert!(s.generated.contains("aaiyyan.mcall(\"lu\""));
}

#[cfg(feature = "python")]
mod python_backend {
    use super::*;
    // Runs only with `cargo test --features python` and a bootstrapped venv.
    // Asserts the SciPy/NumPy-routed names execute without error and that the
    // generated Python carries the routing. Exact solver numerics are checked
    // by the runtime's own suite; here we validate the bridge contract.
    #[test] fn scipy_backed_names_execute() {
        for src in [
            "x := fft([1,0,0,0,0,0,0,0])",
            "y := ifft(fft([1.0,2.0,3.0,4.0]))",
            "p := lu([[4,3],[6,3]])",
            "q := qr([[1,2],[3,4],[5,6]])",
            "c := chol([[4,2],[2,3]])",
            "sv := svd([[1,2],[3,4]])",
            "ev := eig([[2,0],[0,3]])",
            "cn := cond([[1,2],[3,4]])",
            "sq := sqrtm([[4,0],[0,9]])",
            "lg := logm([[1,0],[0,2.718281828]])",
            "rt := roots([1,0,-1])",
            "sp := spline([0,1,2],[0,1,4],1.5)",
            "pc := pchip([0,1,2],[0,1,4],1.5)",
            "rn := exprnd(5,1.0)",
            "tx := table([[1,2,3],[4,5,6]], [\"a\",\"b\"])",
        ] {
            let mut s = fresh();
            let id = eq(&mut s, src, 0.0, 0.0);
            drain(&mut s);
            let errs = s.errors_for(id);
            assert!(errs.is_empty(), "{src} -> {errs:?}");
            let name = src.split(" := ").next().unwrap();
            assert!(s.generated.contains(&format!("aaiyyan.mcall(\"{name}\"")), "{src}");
        }
    }
}
