//! AaiyyanMath visual language.
//!
//! Art direction: an engineer's blueprint desk. Cool tinted chrome frames a pure
//! white worksheet so the mathematics is the brightest thing on screen. One
//! committed electric blue carries interaction; teal is reserved for physical
//! units, amber for warnings, red for dimensional errors, green for solved state.
use aaiyyan_project::LayoutSettings;
use egui::{Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy)]
pub struct Theme {
    pub night: bool,
    pub desk: Color32,
    pub page: Color32,
    pub ribbon: Color32,
    pub ribbon_edge: Color32,
    pub chrome_alt: Color32,
    pub ink: Color32,
    pub ink_soft: Color32,
    pub ink_faint: Color32,
    pub accent: Color32,
    pub accent_deep: Color32,
    pub accent_soft: Color32,
    pub accent_line: Color32,
    pub unit: Color32,
    pub grid: Color32,
    pub grid_major: Color32,
    pub ok: Color32,
    pub warn: Color32,
    pub danger: Color32,
    pub hairline: Color32,
    pub card: Color32,
    pub card_edge: Color32,
    pub scroll_track: Color32,
    pub scroll_thumb: Color32,
    pub scroll_thumb_hover: Color32,
}

impl Theme {
    pub fn faint_hover(&self) -> Color32 {
        if self.night { Color32::from_rgb(38, 47, 60) } else { Color32::from_rgb(236, 242, 250) }
    }
    pub fn glow(&self) -> Color32 { self.accent.linear_multiply(if self.night { 0.22 } else { 0.10 }) }
}

pub fn theme(night: bool) -> Theme {
    if night {
        Theme {
            night: true,
            desk: Color32::from_rgb(14, 18, 25),
            page: Color32::from_rgb(24, 30, 40),
            ribbon: Color32::from_rgb(20, 26, 35),
            ribbon_edge: Color32::from_rgb(41, 51, 66),
            chrome_alt: Color32::from_rgb(26, 33, 44),
            ink: Color32::from_rgb(226, 233, 242),
            ink_soft: Color32::from_rgb(160, 174, 192),
            ink_faint: Color32::from_rgb(112, 127, 146),
            accent: Color32::from_rgb(76, 154, 255),
            accent_deep: Color32::from_rgb(120, 182, 255),
            accent_soft: Color32::from_rgb(28, 48, 76),
            accent_line: Color32::from_rgb(58, 96, 148),
            unit: Color32::from_rgb(53, 196, 207),
            grid: Color32::from_rgb(33, 42, 55),
            grid_major: Color32::from_rgb(45, 58, 76),
            ok: Color32::from_rgb(62, 198, 140),
            warn: Color32::from_rgb(226, 170, 70),
            danger: Color32::from_rgb(240, 112, 96),
            hairline: Color32::from_rgb(38, 48, 62),
            card: Color32::from_rgb(28, 35, 46),
            card_edge: Color32::from_rgb(44, 56, 72),
            scroll_track: Color32::from_rgb(26, 33, 44),
            scroll_thumb: Color32::from_rgb(58, 72, 92),
            scroll_thumb_hover: Color32::from_rgb(82, 104, 134),
        }
    } else {
        Theme {
            night: false,
            desk: Color32::from_rgb(233, 238, 245),
            page: Color32::from_rgb(255, 255, 255),
            ribbon: Color32::from_rgb(244, 247, 251),
            ribbon_edge: Color32::from_rgb(214, 224, 236),
            chrome_alt: Color32::from_rgb(237, 242, 249),
            ink: Color32::from_rgb(20, 30, 45),
            ink_soft: Color32::from_rgb(74, 90, 110),
            ink_faint: Color32::from_rgb(132, 148, 166),
            accent: Color32::from_rgb(11, 99, 206),
            accent_deep: Color32::from_rgb(8, 66, 145),
            accent_soft: Color32::from_rgb(230, 240, 253),
            accent_line: Color32::from_rgb(168, 203, 245),
            unit: Color32::from_rgb(14, 124, 134),
            grid: Color32::from_rgb(234, 241, 248),
            grid_major: Color32::from_rgb(210, 226, 240),
            ok: Color32::from_rgb(18, 128, 92),
            warn: Color32::from_rgb(178, 106, 0),
            danger: Color32::from_rgb(196, 61, 46),
            hairline: Color32::from_rgb(226, 233, 242),
            card: Color32::from_rgb(251, 253, 255),
            card_edge: Color32::from_rgb(232, 239, 247),
            scroll_track: Color32::from_rgb(238, 243, 249),
            scroll_thumb: Color32::from_rgb(196, 210, 226),
            scroll_thumb_hover: Color32::from_rgb(158, 180, 205),
        }
    }
}

pub fn current(ctx: &egui::Context) -> Theme { theme(ctx.style().visuals.dark_mode) }

pub fn is_night(layout: &LayoutSettings) -> bool { layout.theme == "night" }

pub fn apply(ctx: &egui::Context, layout: &LayoutSettings) {
    let t = theme(is_night(layout));
    let mut v = if t.night { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.override_text_color = Some(t.ink);
    v.panel_fill = t.ribbon;
    v.window_fill = t.page;
    v.extreme_bg_color = t.page;
    v.faint_bg_color = t.chrome_alt;
    v.code_bg_color = if t.night { Color32::from_rgb(19, 25, 33) } else { Color32::from_rgb(248, 251, 254) };
    v.selection.bg_fill = t.accent_soft;
    v.selection.stroke = Stroke::new(1.0_f32, t.accent);
    v.hyperlink_color = t.accent;
    v.window_rounding = 6.0.into();
    v.menu_rounding = 6.0.into();
    v.window_stroke = Stroke::new(1.0_f32, t.ribbon_edge);
    v.window_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 6.0), blur: 18.0, spread: 0.0,
        color: Color32::from_black_alpha(if t.night { 110 } else { 34 }),
    };
    v.popup_shadow = v.window_shadow;

    v.widgets.noninteractive.bg_fill = t.chrome_alt;
    v.widgets.noninteractive.weak_bg_fill = t.chrome_alt;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, t.ribbon_edge);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, t.ink_soft);
    v.widgets.inactive.bg_fill = t.page;
    v.widgets.inactive.weak_bg_fill = t.page;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, t.ribbon_edge);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, t.ink);
    v.widgets.hovered.bg_fill = t.accent_soft;
    v.widgets.hovered.weak_bg_fill = t.accent_soft;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, t.accent_line);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, t.accent_deep);
    v.widgets.active.bg_fill = t.accent;
    v.widgets.active.weak_bg_fill = t.accent;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, t.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.widgets.open.bg_fill = t.accent_soft;
    v.widgets.open.weak_bg_fill = t.accent_soft;
    v.widgets.open.bg_stroke = Stroke::new(1.0_f32, t.accent_line);
    v.widgets.open.fg_stroke = Stroke::new(1.0_f32, t.accent_deep);
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    let ui_size = layout.font_size.clamp(12.0, 16.0) - 1.5; // compact density
    style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(ui_size));
    style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(ui_size));
    style.text_styles.insert(egui::TextStyle::Small, FontId::proportional(ui_size - 2.0));
    style.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(ui_size - 0.5));
    style.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(ui_size + 4.0));
    style.spacing.item_spacing = Vec2::new(6.0, 4.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.menu_margin = 8.0.into();
    style.spacing.indent = 14.0;
    style.spacing.scroll.bar_width = 7.0;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.spacing.scroll.bar_outer_margin = 2.0;
    style.spacing.scroll.floating = true;
    style.visuals.widgets.noninteractive.rounding = 4.0.into();
    style.visuals.widgets.inactive.rounding = 4.0.into();
    style.visuals.widgets.hovered.rounding = 4.0.into();
    style.visuals.widgets.active.rounding = 4.0.into();
    style.visuals.widgets.open.rounding = 4.0.into();
    ctx.set_style(style);
}

// ---------------------------------------------------------------------------
// Typography: Cambria for mathematics (falls back to Times, then DejaVu Serif).
// ---------------------------------------------------------------------------
const SERIF_REGULAR: &[&str] = &[
    "C:\\Windows\\Fonts\\cambria.ttc",
    "C:\\Windows\\Fonts\\times.ttf",
    "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    "/usr/share/fonts/TTF/DejaVuSerif.ttf",
];
const SERIF_ITALIC: &[&str] = &[
    "C:\\Windows\\Fonts\\cambriai.ttf",
    "C:\\Windows\\Fonts\\timesi.ttf",
    "/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf",
    "/usr/share/fonts/TTF/DejaVuSerif-Italic.ttf",
];
const SANS_REGULAR: &[&str] = &[
    "C:\\Windows\\Fonts\\segoeui.ttf",
    "/System/Library/Fonts/SFNS.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
];
const MONO_REGULAR: &[&str] = &[
    "C:\\Windows\\Fonts\\consola.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
];

fn load_first(paths: &[&str]) -> Option<Vec<u8>> {
    for p in paths { if let Ok(b) = std::fs::read(p) { return Some(b); } }
    None
}

static MATH_FONTS_READY: AtomicBool = AtomicBool::new(false);

pub fn math_fonts_ready() -> bool { MATH_FONTS_READY.load(Ordering::Relaxed) }

pub fn install_fonts(ctx: &egui::Context) {
    if MATH_FONTS_READY.load(Ordering::Relaxed) { return; }
    let mut fonts = egui::FontDefinitions::default();
    let fallback: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned()
        .unwrap_or_else(|| vec!["Ubuntu-Light".into()]);

    let mut math_chain: Vec<String> = Vec::new();
    if let Some(b) = load_first(SERIF_REGULAR) {
        fonts.font_data.insert("aaiyyan-serif".into(), egui::FontData::from_owned(b).into());
        math_chain.push("aaiyyan-serif".into());
    }
    let mut italic_chain: Vec<String> = Vec::new();
    if let Some(b) = load_first(SERIF_ITALIC) {
        fonts.font_data.insert("aaiyyan-serif-italic".into(), egui::FontData::from_owned(b).into());
        italic_chain.push("aaiyyan-serif-italic".into());
    }
    italic_chain.extend(math_chain.iter().cloned());
    math_chain.extend(fallback.iter().cloned());
    italic_chain.extend(fallback.iter().cloned());
    fonts.families.insert(FontFamily::Name("math".into()), math_chain);
    fonts.families.insert(FontFamily::Name("math-italic".into()), italic_chain);

    if let Some(b) = load_first(SANS_REGULAR) {
        fonts.font_data.insert("aaiyyan-sans".into(), egui::FontData::from_owned(b).into());
        if let Some(c) = fonts.families.get_mut(&FontFamily::Proportional) { c.insert(0, "aaiyyan-sans".into()); }
    }
    if let Some(b) = load_first(MONO_REGULAR) {
        fonts.font_data.insert("aaiyyan-mono".into(), egui::FontData::from_owned(b).into());
        if let Some(c) = fonts.families.get_mut(&FontFamily::Monospace) { c.insert(0, "aaiyyan-mono".into()); }
    }
    ctx.set_fonts(fonts);
    MATH_FONTS_READY.store(true, Ordering::Relaxed);
}

pub fn math(size: f32) -> FontId {
    if math_fonts_ready() { FontId::new(size, FontFamily::Name("math".into())) } else { FontId::proportional(size) }
}
pub fn math_italic(size: f32) -> FontId {
    if math_fonts_ready() { FontId::new(size, FontFamily::Name("math-italic".into())) } else { FontId::proportional(size) }
}
pub fn mono(size: f32) -> FontId { FontId::monospace(size) }

/// Compact engineering number formatting (Rust has no `{:g}`).
pub fn num(v: f64) -> String {
    if !v.is_finite() { return format!("{v}"); }
    if v == 0.0 { return "0".to_string(); }
    let a = v.abs();
    if a >= 1e6 || a < 1e-4 {
        let s = format!("{:.4e}", v);
        let mut parts = s.split('e');
        let mant = parts.next().unwrap_or("0").trim_end_matches('0').trim_end_matches('.').to_string();
        let exp: i32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        return format!("{mant}e{exp}");
    }
    let decimals = (5.0 - a.log10().floor()).max(0.0).min(10.0) as usize;
    let s = format!("{:.*}", decimals, v);
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

pub fn series_color(i: usize, night: bool) -> Color32 {
    const LIGHT: [(u8, u8, u8); 8] = [
        (11, 99, 206), (196, 61, 46), (18, 128, 92), (124, 58, 173),
        (178, 106, 0), (14, 124, 134), (176, 38, 106), (72, 90, 112),
    ];
    const NIGHT: [(u8, u8, u8); 8] = [
        (76, 154, 255), (240, 112, 96), (62, 198, 140), (186, 130, 245),
        (232, 178, 74), (53, 196, 207), (240, 118, 172), (150, 168, 190),
    ];
    let table = if night { NIGHT } else { LIGHT };
    let c = table[i % table.len()];
    Color32::from_rgb(c.0, c.1, c.2)
}

// ---------------------------------------------------------------------------
// Scrollbars in the artifact's own language (never the default black bar).
// ---------------------------------------------------------------------------
pub fn scroll(ui: &mut Ui, t: &Theme, add: impl FnOnce(&mut Ui)) {
    ui.scope(|ui| {
        let v = ui.visuals_mut();
        v.widgets.noninteractive.bg_fill = t.scroll_thumb;
        v.widgets.noninteractive.weak_bg_fill = t.scroll_track;
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, t.scroll_thumb);
        v.widgets.hovered.bg_fill = t.scroll_thumb_hover;
        v.widgets.hovered.weak_bg_fill = t.scroll_thumb_hover;
        v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, t.scroll_thumb_hover);
        v.widgets.active.bg_fill = t.accent;
        v.widgets.active.weak_bg_fill = t.accent;
        v.widgets.active.fg_stroke = Stroke::new(1.0_f32, t.accent);
        ui.spacing_mut().scroll.bar_width = 7.0;
        ui.spacing_mut().scroll.floating = true;
        ui.spacing_mut().scroll.bar_outer_margin = 2.0;
        ui.spacing_mut().scroll.bar_inner_margin = 2.0;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, add);
    });
}

// ---------------------------------------------------------------------------
// Vector icons — one stroke weight across the whole set.
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    New, Open, Save, Undo, Redo, Bolt, Stop, Equation, Equals, Text, Heading,
    Matrix, Vector, Plot2D, Plot3D, Table, Slider, Python, Export, Trash,
    Up, Down, Pencil, Plus, Minus, Grid, Zoom, Sine, Integral, Derivative,
    Sigma, Root, Pi, Gear, Check, Doc, Chevron, Reset, Copy, Info, Keyboard,
}

pub fn paint_icon(painter: &egui::Painter, rect: Rect, kind: Icon, color: Color32) {
    let (w, h) = (rect.width(), rect.height());
    let s = w.min(h);
    let sw = (s * 0.075).clamp(1.15_f32, 1.85_f32);
    let stroke = Stroke::new(sw, color);
    let thin = Stroke::new((sw * 0.82).max(0.9_f32), color);
    let p = |x: f32, y: f32| Pos2::new(rect.left() + x * w, rect.top() + y * h);
    let line = |pts: Vec<Pos2>, st: Stroke| { painter.add(egui::Shape::line(pts, st)); };
    let dot = |x: f32, y: f32, r: f32| { painter.circle_filled(p(x, y), r * s, color); };
    let frame = |x0: f32, y0: f32, x1: f32, y1: f32, st: Stroke| {
        painter.rect_stroke(Rect::from_min_max(p(x0, y0), p(x1, y1)), 1.5, st);
    };

    match kind {
        Icon::New | Icon::Doc => {
            line(vec![p(0.27, 0.13), p(0.61, 0.13), p(0.77, 0.29), p(0.77, 0.87), p(0.27, 0.87), p(0.27, 0.13)], stroke);
            line(vec![p(0.61, 0.13), p(0.61, 0.29), p(0.77, 0.29)], thin);
            if kind == Icon::Doc {
                for (i, y) in [0.47_f32, 0.61, 0.75].iter().enumerate() {
                    line(vec![p(0.37, *y), p(if i == 2 { 0.59 } else { 0.67 }, *y)], thin);
                }
            }
        }
        Icon::Open => {
            line(vec![p(0.15, 0.77), p(0.15, 0.25), p(0.40, 0.25), p(0.48, 0.35), p(0.85, 0.35), p(0.85, 0.77), p(0.15, 0.77)], stroke);
            line(vec![p(0.15, 0.47), p(0.85, 0.47)], thin);
        }
        Icon::Save => {
            frame(0.17, 0.17, 0.83, 0.83, stroke);
            painter.rect_filled(Rect::from_min_max(p(0.33, 0.17), p(0.67, 0.40)), 0.0, color);
            frame(0.31, 0.57, 0.69, 0.83, thin);
        }
        Icon::Undo => {
            line(vec![p(0.22, 0.50), p(0.44, 0.29), p(0.67, 0.37), p(0.76, 0.61), p(0.60, 0.79)], stroke);
            line(vec![p(0.22, 0.50), p(0.43, 0.53)], thin);
            line(vec![p(0.22, 0.50), p(0.28, 0.30)], thin);
        }
        Icon::Redo => {
            line(vec![p(0.78, 0.50), p(0.56, 0.29), p(0.33, 0.37), p(0.24, 0.61), p(0.40, 0.79)], stroke);
            line(vec![p(0.78, 0.50), p(0.57, 0.53)], thin);
            line(vec![p(0.78, 0.50), p(0.72, 0.30)], thin);
        }
        Icon::Bolt => {
            painter.add(egui::Shape::convex_polygon(
                vec![p(0.57, 0.10), p(0.28, 0.56), p(0.48, 0.56), p(0.42, 0.90), p(0.74, 0.42), p(0.52, 0.42)],
                color, Stroke::NONE));
        }
        Icon::Stop => { painter.rect_filled(Rect::from_min_max(p(0.29, 0.29), p(0.71, 0.71)), 1.5, color); }
        Icon::Equation => {
            dot(0.5, 0.25, 0.072);
            line(vec![p(0.23, 0.50), p(0.77, 0.50)], stroke);
            dot(0.5, 0.75, 0.072);
        }
        Icon::Equals => {
            line(vec![p(0.23, 0.40), p(0.77, 0.40)], stroke);
            line(vec![p(0.23, 0.62), p(0.77, 0.62)], stroke);
        }
        Icon::Text => {
            for (y, x1) in [(0.28_f32, 0.82_f32), (0.46, 0.70), (0.64, 0.80), (0.82, 0.56)] {
                line(vec![p(0.18, y), p(x1, y)], thin);
            }
        }
        Icon::Heading => {
            line(vec![p(0.29, 0.21), p(0.29, 0.79)], stroke);
            line(vec![p(0.71, 0.21), p(0.71, 0.79)], stroke);
            line(vec![p(0.29, 0.50), p(0.71, 0.50)], stroke);
        }
        Icon::Matrix => {
            line(vec![p(0.31, 0.16), p(0.21, 0.16), p(0.21, 0.84), p(0.31, 0.84)], thin);
            line(vec![p(0.69, 0.16), p(0.79, 0.16), p(0.79, 0.84), p(0.69, 0.84)], thin);
            for x in [0.39_f32, 0.61] { for y in [0.32_f32, 0.52, 0.72] { dot(x, y, 0.052); } }
        }
        Icon::Vector => {
            line(vec![p(0.37, 0.16), p(0.27, 0.16), p(0.27, 0.84), p(0.37, 0.84)], thin);
            line(vec![p(0.63, 0.16), p(0.73, 0.16), p(0.73, 0.84), p(0.63, 0.84)], thin);
            for y in [0.30_f32, 0.50, 0.70] { dot(0.5, y, 0.058); }
        }
        Icon::Plot2D => {
            line(vec![p(0.18, 0.16), p(0.18, 0.82), p(0.86, 0.82)], thin);
            let mut pts = Vec::new();
            for i in 0..=18 {
                let t = i as f32 / 18.0;
                pts.push(p(0.20 + t * 0.64, 0.70 - 0.46 * (t * std::f32::consts::PI).sin().max(0.0)));
            }
            line(pts, stroke);
        }
        Icon::Plot3D => {
            let a = [p(0.5, 0.14), p(0.84, 0.32), p(0.84, 0.70), p(0.5, 0.88), p(0.16, 0.70), p(0.16, 0.32)];
            painter.add(egui::Shape::closed_line(a.to_vec(), thin));
            line(vec![p(0.5, 0.50), p(0.5, 0.88)], thin);
            line(vec![p(0.5, 0.50), p(0.16, 0.32)], thin);
            line(vec![p(0.5, 0.50), p(0.84, 0.32)], thin);
        }
        Icon::Table => {
            frame(0.16, 0.23, 0.84, 0.79, thin);
            line(vec![p(0.16, 0.42), p(0.84, 0.42)], thin);
            line(vec![p(0.16, 0.61), p(0.84, 0.61)], thin);
            line(vec![p(0.42, 0.23), p(0.42, 0.79)], thin);
            line(vec![p(0.63, 0.23), p(0.63, 0.79)], thin);
        }
        Icon::Slider => {
            line(vec![p(0.16, 0.5), p(0.84, 0.5)], thin);
            painter.circle_filled(p(0.60, 0.5), s * 0.14, color);
        }
        Icon::Python => {
            frame(0.18, 0.18, 0.58, 0.58, thin);
            frame(0.42, 0.42, 0.82, 0.82, thin);
            dot(0.30, 0.30, 0.048);
            dot(0.70, 0.70, 0.048);
        }
        Icon::Export => {
            line(vec![p(0.20, 0.62), p(0.20, 0.84), p(0.80, 0.84), p(0.80, 0.62)], thin);
            line(vec![p(0.50, 0.72), p(0.50, 0.18)], stroke);
            line(vec![p(0.32, 0.36), p(0.50, 0.16), p(0.68, 0.36)], stroke);
        }
        Icon::Trash => {
            line(vec![p(0.20, 0.28), p(0.80, 0.28)], stroke);
            line(vec![p(0.38, 0.28), p(0.38, 0.17), p(0.62, 0.17), p(0.62, 0.28)], thin);
            line(vec![p(0.28, 0.28), p(0.34, 0.86), p(0.66, 0.86), p(0.72, 0.28)], thin);
            line(vec![p(0.44, 0.42), p(0.46, 0.74)], thin);
            line(vec![p(0.56, 0.42), p(0.54, 0.74)], thin);
        }
        Icon::Up => { line(vec![p(0.27, 0.62), p(0.50, 0.37), p(0.73, 0.62)], stroke); }
        Icon::Down => { line(vec![p(0.27, 0.38), p(0.50, 0.63), p(0.73, 0.38)], stroke); }
        Icon::Chevron => { line(vec![p(0.41, 0.27), p(0.64, 0.50), p(0.41, 0.73)], stroke); }
        Icon::Pencil => {
            line(vec![p(0.24, 0.78), p(0.30, 0.58), p(0.66, 0.20), p(0.80, 0.34), p(0.44, 0.72), p(0.24, 0.78)], thin);
            line(vec![p(0.60, 0.26), p(0.74, 0.40)], thin);
        }
        Icon::Plus => {
            line(vec![p(0.50, 0.23), p(0.50, 0.77)], stroke);
            line(vec![p(0.23, 0.50), p(0.77, 0.50)], stroke);
        }
        Icon::Minus => { line(vec![p(0.23, 0.50), p(0.77, 0.50)], stroke); }
        Icon::Grid => {
            for x in [0.27_f32, 0.50, 0.73] { for y in [0.27_f32, 0.50, 0.73] { dot(x, y, 0.052); } }
        }
        Icon::Zoom => {
            painter.circle_stroke(p(0.44, 0.44), s * 0.25, stroke);
            line(vec![p(0.62, 0.62), p(0.84, 0.84)], stroke);
        }
        Icon::Sine => {
            let mut pts = Vec::new();
            for i in 0..=26 {
                let t = i as f32 / 26.0;
                pts.push(p(0.14 + t * 0.72, 0.5 - 0.29 * (t * std::f32::consts::TAU).sin()));
            }
            line(pts, stroke);
        }
        Icon::Integral => {
            let mut pts = Vec::new();
            for i in 0..=26 {
                let t = i as f32 / 26.0;
                let bend = ((t - 0.5) * std::f32::consts::PI).cos();
                pts.push(p(0.5 + 0.19 * bend * if t < 0.5 { 1.0 } else { -1.0 }, 0.14 + t * 0.72));
            }
            line(pts, stroke);
        }
        Icon::Derivative => {
            dot(0.34, 0.26, 0.058);
            line(vec![p(0.20, 0.50), p(0.80, 0.50)], stroke);
            dot(0.34, 0.74, 0.058);
            line(vec![p(0.58, 0.66), p(0.58, 0.84)], thin);
            line(vec![p(0.70, 0.66), p(0.70, 0.84)], thin);
        }
        Icon::Sigma => { line(vec![p(0.76, 0.19), p(0.26, 0.19), p(0.54, 0.50), p(0.26, 0.81), p(0.76, 0.81)], stroke); }
        Icon::Root => {
            line(vec![p(0.14, 0.56), p(0.28, 0.80), p(0.46, 0.20), p(0.86, 0.20)], stroke);
            line(vec![p(0.53, 0.45), p(0.74, 0.75)], thin);
            line(vec![p(0.74, 0.45), p(0.53, 0.75)], thin);
        }
        Icon::Pi => {
            line(vec![p(0.20, 0.31), p(0.80, 0.31)], stroke);
            line(vec![p(0.37, 0.31), p(0.33, 0.80)], stroke);
            line(vec![p(0.63, 0.31), p(0.67, 0.80)], stroke);
        }
        Icon::Gear | Icon::Reset => {
            painter.circle_stroke(p(0.5, 0.5), s * 0.19, stroke);
            for k in 0..6 {
                let a = k as f32 * std::f32::consts::TAU / 6.0;
                let (sy, cy) = a.sin_cos();
                line(vec![
                    Pos2::new(rect.center().x + cy * s * 0.25, rect.center().y + sy * s * 0.25),
                    Pos2::new(rect.center().x + cy * s * 0.39, rect.center().y + sy * s * 0.39),
                ], thin);
            }
        }
        Icon::Check => { line(vec![p(0.23, 0.54), p(0.42, 0.73), p(0.79, 0.29)], stroke); }
        Icon::Copy => {
            frame(0.16, 0.16, 0.62, 0.62, thin);
            frame(0.38, 0.38, 0.84, 0.84, stroke);
        }
        Icon::Info => {
            painter.circle_stroke(p(0.5, 0.5), s * 0.34, thin);
            dot(0.5, 0.31, 0.045);
            line(vec![p(0.5, 0.44), p(0.5, 0.72)], stroke);
        }
        Icon::Keyboard => {
            frame(0.13, 0.29, 0.87, 0.73, thin);
            for y in [0.42_f32, 0.55] { for x in [0.25_f32, 0.40, 0.55, 0.70] { dot(x, y, 0.035); } }
            line(vec![p(0.33, 0.65), p(0.67, 0.65)], thin);
        }
    }
}

pub fn icon(ui: &mut Ui, kind: Icon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_icon(&ui.painter_at(rect), rect, kind, color);
}

/// Ribbon button. `big` = icon over caption; a trailing ▾ renders a drawn caret.
pub fn ribbon_button(ui: &mut Ui, kind: Icon, label: &str, big: bool, enabled: bool, t: &Theme) -> egui::Response {
    let caret = label.ends_with('\u{25BE}');
    let text: &str = if caret { label.trim_end_matches('\u{25BE}').trim_end() } else { label };
    let font = FontId::proportional(if big { 10.5 } else { 12.0 });
    let text_w = if text.is_empty() { 0.0 } else {
        ui.fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), t.ink).size().x)
    };
    let caret_w = if caret { 10.0 } else { 0.0 };
    let size = if big {
        Vec2::new((text_w + caret_w + 14.0).max(50.0), 52.0)
    } else if text.is_empty() {
        Vec2::new(26.0, 24.0)
    } else {
        Vec2::new(text_w + caret_w + 32.0, 24.0)
    };
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let anim = ui.ctx().animate_bool_with_time(response.id.with("hover"), enabled && response.hovered(), 0.12);
    if anim > 0.001 && anim < 0.999 { ui.ctx().request_repaint(); }
    let painter = ui.painter_at(rect);
    let down = enabled && response.hovered() && ui.input(|i| i.pointer.primary_down());

    if anim > 0.001 {
        let fill = if down { t.accent_line.linear_multiply(0.40) } else { t.accent_soft };
        painter.rect_filled(rect, 4.0, fill.linear_multiply(anim));
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, t.accent_line.linear_multiply(anim)));
    }
    let fg = if enabled { t.ink } else { t.ink_faint };
    let ic = if enabled { t.accent } else { t.ink_faint };

    if big {
        let ir = Rect::from_center_size(Pos2::new(rect.center().x, rect.top() + 18.0), Vec2::splat(21.0));
        paint_icon(&painter, ir, kind, ic);
        let tx = rect.center().x - caret_w * 0.5;
        let ty = rect.bottom() - 12.0;
        if !text.is_empty() { painter.text(Pos2::new(tx, ty), egui::Align2::CENTER_CENTER, text, font, fg); }
        if caret {
            let cx = tx + text_w * 0.5 + 5.0;
            painter.add(egui::Shape::convex_polygon(
                vec![Pos2::new(cx - 3.0, ty - 1.5), Pos2::new(cx + 3.0, ty - 1.5), Pos2::new(cx, ty + 2.5)], fg, Stroke::NONE));
        }
    } else if text.is_empty() {
        let ir = Rect::from_center_size(rect.center(), Vec2::splat(15.0));
        paint_icon(&painter, ir, kind, ic);
    } else {
        let ir = Rect::from_center_size(Pos2::new(rect.left() + 14.0, rect.center().y), Vec2::splat(15.0));
        paint_icon(&painter, ir, kind, ic);
        let tx = rect.left() + 26.0;
        let ty = rect.center().y;
        painter.text(Pos2::new(tx, ty), egui::Align2::LEFT_CENTER, text, font, fg);
        if caret {
            let cx = tx + text_w + 5.0;
            painter.add(egui::Shape::convex_polygon(
                vec![Pos2::new(cx - 3.0, ty - 1.5), Pos2::new(cx + 3.0, ty - 1.5), Pos2::new(cx, ty + 2.5)], fg, Stroke::NONE));
        }
    }
    response
}

/// Ribbon group: fixed 52px button row + a caption on a shared baseline.
pub fn ribbon_group(ui: &mut Ui, title: &str, t: &Theme, contents: impl FnOnce(&mut Ui)) {
    // A horizontal child is sized to its content; a vertical one would expand to
    // the whole remaining ribbon width and hide every following group.
    let inner = ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
        ui.set_min_height(52.0);
        contents(ui);
    });
    let r = inner.response.rect;
    let painter = ui.painter();
    painter.text(
        Pos2::new(r.center().x, r.bottom() + 9.0),
        egui::Align2::CENTER_CENTER,
        title,
        FontId::proportional(9.5),
        t.ink_faint,
    );
    painter.line_segment(
        [Pos2::new(r.right() + 6.0, r.top() + 2.0), Pos2::new(r.right() + 6.0, r.bottom() + 15.0)],
        Stroke::new(1.0_f32, t.ribbon_edge),
    );
    ui.add_space(13.0);
}

/// Bounded vertical stack for ribbon groups (Mode, Backend, settings, …).
pub fn ribbon_stack(ui: &mut Ui, width: f32, contents: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 52.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
            contents(ui);
        },
    );
}

pub fn chip(ui: &mut Ui, text: &str, color: Color32, t: &Theme) {
    let font = FontId::proportional(10.5);
    let w = ui.fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), color).size().x) + 16.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 19.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 9.5, color.linear_multiply(if t.night { 0.20 } else { 0.11 }));
    painter.rect_stroke(rect, 9.5, Stroke::new(1.0_f32, color.linear_multiply(0.40)));
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, text, font, color);
}

pub fn key_chip(ui: &mut Ui, text: &str, t: &Theme) {
    let font = mono(10.5);
    let w = ui.fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), t.ink).size().x) + 12.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w.max(20.0), 18.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, t.chrome_alt);
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, t.ribbon_edge));
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, text, font, t.ink_soft);
}

pub fn hairline(ui: &mut Ui, t: &Theme) {
    let r = ui.available_rect_before_wrap();
    ui.painter().line_segment([Pos2::new(r.left(), r.top()), Pos2::new(r.right(), r.top())],
        Stroke::new(1.0_f32, t.hairline));
    ui.add_space(1.0);
}

pub fn section(ui: &mut Ui, title: &str, t: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new(title.to_uppercase()).size(9.5).strong().color(t.ink_faint));
    ui.add_space(3.0);
}

pub fn card(ui: &mut Ui, t: &Theme, add: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(t.card)
        .stroke(Stroke::new(1.0_f32, t.card_edge))
        .rounding(5.0)
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| { ui.set_min_width(ui.available_width()); add(ui); });
}

pub fn accent() -> Color32 { Color32::from_rgb(11, 99, 206) }
pub fn error_color() -> Color32 { Color32::from_rgb(196, 61, 46) }
pub fn ok_color() -> Color32 { Color32::from_rgb(18, 128, 92) }
