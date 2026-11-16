//! Versioned JSON project format with backups and migration (spec section 24).
use aaiyyan_plotting::{Plot2DSpec, SeriesSpec};
use aaiyyan_worksheet::{Block, BlockKind, ControlSpec, Worksheet};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const FORMAT_VERSION: u32 = 1;
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported project format version {0}")]
    UnsupportedVersion(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalcMode { Auto, Manual }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backend { Native, Python, Auto }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalcSettings {
    pub mode: CalcMode,
    pub backend: Backend,
    pub tolerance: f64,
    pub max_iterations: u32,
    pub timeout_ms: u64,
    pub display_precision: u32,
    pub angle_unit: String,
}

impl Default for CalcSettings {
    fn default() -> Self {
        CalcSettings {
            mode: CalcMode::Auto, backend: Backend::Auto, tolerance: 1e-9,
            max_iterations: 1000, timeout_ms: 15000, display_precision: 6, angle_unit: "rad".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PythonSettings {
    pub extra_paths: Vec<String>,
    pub allow_user_code: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutSettings {
    pub theme: String,
    pub show_left: bool,
    pub show_right: bool,
    pub show_bottom: bool,
    pub font_size: f32,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        LayoutSettings { theme: "mathcad".into(), show_left: true, show_right: true, show_bottom: true, font_size: 15.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub author: String,
    pub description: String,
    pub created: String,
    pub modified: String,
    pub app_version: String,
}

impl Default for Metadata {
    fn default() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Metadata {
            name: "Untitled project".into(), author: String::new(),
            description: String::new(), created: now.clone(), modified: now,
            app_version: APP_VERSION.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    pub metadata: Metadata,
    pub worksheets: Vec<Worksheet>,
    pub settings: CalcSettings,
    pub python: PythonSettings,
    pub layout: LayoutSettings,
}

impl Default for Project {
    fn default() -> Self { Project::new_demo() }
}

impl Project {
    pub fn empty(name: &str) -> Project {
        Project {
            format_version: FORMAT_VERSION,
            metadata: Metadata { name: name.into(), ..Default::default() },
            worksheets: vec![Worksheet::new("Worksheet 1")],
            settings: CalcSettings::default(),
            python: PythonSettings::default(),
            layout: LayoutSettings::default(),
        }
    }

    /// The MVP acceptance worksheet from spec section 28 plus an interactive demo.
    pub fn new_demo() -> Project {
        let mut main = Worksheet::new("Beam deflection");
        main.blocks.push(Block::heading(1, "Simply supported beam - uniform load"));
        main.blocks.push(Block::text(
            "MVP acceptance worksheet (spec section 28). Units are validated natively; \
             the generated Python is shown in the bottom panel.",
        ));
        main.blocks.push(Block::equation("E := 210 GPa"));
        main.blocks.push(Block::equation("I := 8.5e-6 m^4"));
        main.blocks.push(Block::equation("L := 3 m"));
        main.blocks.push(Block::equation("w := 10 kN/m"));
        main.blocks.push(Block::equation("delta := 5*w*L^4/(384*E*I)"));
        main.blocks.push(Block::equation("delta_mm := convert(delta, mm)"));
        main.blocks.push(Block::text("Bending moment along the span, x is the normalised position x/L:"));
        main.blocks.push(Block::equation("M(x) := w*L^2*x*(1-x)/2"));
        main.blocks.push(Block::new(BlockKind::Plot2D(Plot2DSpec {
            title: "Bending moment M(x)".into(),
            x_label: "x / L".into(),
            y_label: "M [N*m]".into(),
            x_min: 0.0, x_max: 1.0, samples: 200, show_legend: true,
            series: vec![SeriesSpec {
                expr: "M(x)".into(), variable: "x".into(), label: "M(x)".into(),
                kind: aaiyyan_plotting::SeriesKind::Line, color: Some([90, 170, 230]),
            }],
        })));

        let mut demo = Worksheet::new("Interactive demo");
        demo.blocks.push(Block::heading(1, "Interactive parameter control"));
        demo.blocks.push(Block::equation("a := slider(1, 100)"));
        demo.blocks.push(Block::new(BlockKind::Control(ControlSpec::slider("a", 1.0, 100.0))));
        demo.blocks.push(Block::equation("f(x) := a*sin(x)"));
        demo.blocks.push(Block::new(BlockKind::Plot2D(Plot2DSpec {
            title: "f(x) = a sin(x)".into(), x_label: "x".into(), y_label: "f(x)".into(),
            x_min: 0.0, x_max: 6.283185307, samples: 300, show_legend: true,
            series: vec![SeriesSpec::line("f(x)", "x", "a sin(x)")],
        })));
        demo.blocks.push(Block::new(BlockKind::Plot3D(Default::default())));

        Project {
            format_version: FORMAT_VERSION,
            metadata: Metadata {
                name: "AaiyyanMath demo".into(),
                description: "MVP acceptance test".into(),
                ..Default::default()
            },
            worksheets: vec![main, demo],
            settings: CalcSettings::default(),
            python: PythonSettings::default(),
            layout: LayoutSettings::default(),
        }
    }

    pub fn to_json(&self) -> Result<String, ProjectError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(text: &str) -> Result<Project, ProjectError> {
        let mut project: Project = serde_json::from_str(text)?;
        if project.format_version > FORMAT_VERSION {
            return Err(ProjectError::UnsupportedVersion(project.format_version));
        }
        project.migrate();
        Ok(project)
    }

    fn migrate(&mut self) {
        // Future migrations are applied stepwise here.
        if self.format_version < FORMAT_VERSION {
            self.format_version = FORMAT_VERSION;
        }
        if self.worksheets.is_empty() {
            self.worksheets.push(Worksheet::new("Worksheet 1"));
        }
    }

    pub fn load(path: &Path) -> Result<Project, ProjectError> {
        let text = fs::read_to_string(path)?;
        Project::from_json(&text)
    }

    /// Atomic save with a rotating backup.
    pub fn save(&mut self, path: &Path) -> Result<(), ProjectError> {
        self.metadata.modified = chrono::Utc::now().to_rfc3339();
        self.metadata.app_version = APP_VERSION.into();
        let json = self.to_json()?;
        if path.exists() {
            let _ = fs::copy(path, backup_path(path));
        }
        let tmp = path.with_extension("aaiyyan.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn save_recovery(&self, dir: &Path) -> Result<PathBuf, ProjectError> {
        fs::create_dir_all(dir)?;
        let path = dir.join("recovery.aaiyyan.json");
        fs::write(&path, self.to_json()?)?;
        Ok(path)
    }
}

pub fn backup_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".bak");
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip() {
        let p = Project::new_demo();
        let json = p.to_json().unwrap();
        let back = Project::from_json(&json).unwrap();
        assert_eq!(back.worksheets.len(), p.worksheets.len());
        assert_eq!(back.format_version, FORMAT_VERSION);
        assert_eq!(back.metadata.name, "AaiyyanMath demo");
    }

    #[test]
    fn save_and_load_with_backup() {
        let dir = std::env::temp_dir().join(format!("aaiyyan-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("p.aaiyyan.json");
        let mut p = Project::empty("test");
        p.save(&path).unwrap();
        p.metadata.name = "second".into();
        p.save(&path).unwrap();
        assert!(backup_path(&path).exists());
        let loaded = Project::load(&path).unwrap();
        assert_eq!(loaded.metadata.name, "second");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
