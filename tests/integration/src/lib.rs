//! Integration test harness for the MVP acceptance criteria (spec section 28).
pub fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("aaiyyan-{tag}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
