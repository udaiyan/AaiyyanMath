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
pub enum Drag { Pan, Move(Uuid), Resize(Uuid) }

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
    /// Ribbon visibility (session state) and one-key focus mode.
    pub show_ribbon: bool,
    pub zen: bool,
    /// Structured math objects keyed by region, plus the active slot.
    pub templates: std::collections::HashMap<uuid::Uuid, crate::template::Tpl>,
    pub focused_slot: Option<(uuid::Uuid, u32)>,
    pub slot_buffer: String,
    pub slot_focus: Option<(uuid::Uuid, u32)>,
    pub slot_hits: Vec<(uuid::Uuid, u32, egui::Rect)>,
    pub math_size: f32,
    /// Live typesetting parameters driven by the Equation Studio rail.
    pub script_scale: f32,
    pub math_spacing: f32,
    pub rule_scale: f32,
    pub slot_pad: f32,
    pub studio_zoom: f32,
    pub studio_tab: usize,
    pub studio_open: bool,
    pub studio_height: f32,
    pub style_open: bool,
    pub focused_leaf: Option<(uuid::Uuid, Vec<usize>)>,
    pub leaf_buffer: String,
    pub leaf_focus: Option<(uuid::Uuid, Vec<usize>)>,
    pub leaf_hits: Vec<(uuid::Uuid, Vec<usize>, egui::Rect)>,
    pub def_names: std::collections::HashMap<uuid::Uuid, String>,
    /// Fixed region widths in world units; absent = auto-fit.
    pub widths: std::collections::HashMap<uuid::Uuid, f32>,
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
            show_ribbon: true,
            zen: false,
            templates: std::collections::HashMap::new(),
            focused_slot: None,
            slot_buffer: String::new(),
            slot_focus: None,
            slot_hits: Vec::new(),
            math_size: 20.0,
            script_scale: 0.70,
            math_spacing: 1.0,
            rule_scale: 1.0,
            slot_pad: 1.0,
            studio_zoom: 1.8,
            studio_tab: 0,
            studio_open: true,
            studio_height: 268.0,
            style_open: false,
            focused_leaf: None,
            leaf_buffer: String::new(),
            leaf_focus: None,
            leaf_hits: Vec::new(),
            def_names: std::collections::HashMap::new(),
            widths: std::collections::HashMap::new(),
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
    ToggleZen,
    TemplateNew(crate::template::Make),
    TemplateMake(crate::template::Make, uuid::Uuid, u32),
    TemplateLeaf(String, bool),
    TemplateFocus(uuid::Uuid, u32),
    TemplateNextSlot(uuid::Uuid),
    TemplateCommitSlot(uuid::Uuid, u32),
    TemplateClearSlot(uuid::Uuid, u32),
    SetBlockWidth(uuid::Uuid, f32),
    FitBlockWidth(uuid::Uuid),
    TemplateFocusLeaf(uuid::Uuid, Vec<usize>),
    TemplateCommitLeaf(uuid::Uuid, Vec<usize>),
    DefineSymbol(String, f32, f32),
    SetZoom(f32),
    ToggleGrid,
    ToggleSnap,
    ResetView,
    ExportPython, ExportHtml, ExportMarkdown, ExportTypst, ExportPdf,
    ResetPython, LoadDemo,
}
