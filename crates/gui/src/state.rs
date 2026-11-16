use aaiyyan_plotting::{Plot2DSpec, Plot3DSpec};
use aaiyyan_project::{Backend, CalcMode};
use aaiyyan_worksheet::{BlockKind, ControlKind};
use egui::{Pos2, Vec2};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub struct View3d { pub yaw: f32, pub pitch: f32, pub zoom: f32 }

impl Default for View3d {
    fn default() -> Self { View3d { yaw: 0.7, pitch: 0.6, zoom: 1.0 } }
}
impl View3d { pub fn new() -> View3d { View3d::default() } }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drag { Pan, Move(Uuid) }

pub const RIBBON_TABS: &[&str] = &["Math", "Plots", "Matrices", "Functions", "Calculation", "Document", "View"];
pub const PALETTES: &[&str] = &["Operators", "Symbols", "Constants", "Programming", "Units", "Functions"];

pub struct UiState {
    pub selected: Option<Uuid>,
    pub editing: Option<Uuid>,
    pub hovered: Option<Uuid>,
    pub focus_request: Option<Uuid>,
    pub bottom_tab: usize,
    pub views3d: HashMap<Uuid, View3d>,
    /// World-space size of each rendered region (screen size / zoom).
    pub sizes: HashMap<Uuid, Vec2>,
    pub status: String,
    pub ribbon: usize,
    pub pan: Vec2,
    pub zoom: f32,
    pub cursor: Option<Vec2>,
    pub show_grid: bool,
    pub snap: bool,
    pub palette: Option<usize>,
    pub palette_anchor: Pos2,
    /// Screen rects of the palette toggle buttons for the current frame.
    pub palette_buttons: Vec<(usize, egui::Rect)>,
    /// Screen rect of the open palette popup (so the canvas ignores clicks on it).
    pub palette_rect: Option<egui::Rect>,
    /// Panel sizes owned by the app (drag handles live at the canvas edges).
    pub dock_height: f32,
    pub left_width: f32,
    pub right_width: f32,
    pub drag: Option<Drag>,
    pub press: Option<Pos2>,
    pub moved: bool,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            selected: None, editing: None, hovered: None, focus_request: None,
            bottom_tab: 0, views3d: HashMap::new(), sizes: HashMap::new(),
            status: "Ready".into(), ribbon: 0,
            pan: Vec2::ZERO, zoom: 1.0, cursor: Some(Vec2::new(120.0, 96.0)),
            show_grid: true, snap: true, palette: None, palette_anchor: Pos2::ZERO,
            palette_buttons: Vec::new(),
            palette_rect: None,
            dock_height: 186.0,
            left_width: 236.0,
            right_width: 292.0,
            drag: None, press: None, moved: false,
        }
    }
}

pub enum UiAction {
    New, Open, Save, SaveAs, Undo, Redo, Recalculate, Cancel,
    Select(Uuid),
    Edit(Uuid, bool),
    UpdateSource(Uuid, String),
    InsertSnippet(String),
    InsertSnippetAt(String, f32, f32),
    InsertBlockAt(BlockKind, f32, f32),
    SetBlockPos(Uuid, f32, f32),
    SetBlockPosDelta(Uuid, f32, f32),
    UpdatePlot2D(Uuid, Plot2DSpec),
    UpdatePlot3D(Uuid, Plot3DSpec),
    UpdateControl(Uuid, ControlKind),
    Insert(BlockKind),
    Delete(Uuid),
    Move(Uuid, i32),
    ToggleEnabled(Uuid),
    RunPython(Uuid),
    SetTrust(Uuid, bool),
    AddWorksheet,
    SelectWorksheet(usize),
    SetMode(CalcMode),
    SetBackend(Backend),
    SetTheme(String),
    SetAllowUserCode(bool),
    TogglePanel(u8),
    SetZoom(f32),
    ToggleGrid,
    ToggleSnap,
    ResetView,
    ExportPython, ExportHtml, ExportMarkdown, ExportTypst, ExportPdf,
    ResetPython, LoadDemo,
}
