//! Mathcad palettes. Clicking an entry places a structured math object with
//! fillable slots on the canvas — never a text snippet.
use crate::state::{UiAction, UiState};
use crate::template::Make;
use crate::theme::{self, Theme};
use egui::{Color32, Pos2, Sense, Stroke, Vec2};

pub struct Entry {
    pub glyph: &'static str,
    pub tip: &'static str,
    pub act: Act,
}

#[derive(Clone)]
pub enum Act {
    Make(Make),
    Sym(&'static str),
    Unit(&'static str),
}

pub struct Group { pub title: &'static str, pub entries: &'static [Entry] }

macro_rules! grp {
    ($title:expr, [ $(($g:expr, $a:expr, $t:expr)),* $(,)? ]) => {
        Group { title: $title, entries: &[ $(Entry { glyph: $g, act: $a, tip: $t }),* ] }
    };
}

pub fn groups(kind: usize) -> Vec<Group> {
    match kind {
        0 => vec![
            grp!("Structure — click to place, then fill the slots", [
                ("\u{222B}", Act::Make(Make::Integral), "Integral with limit slots"),
                ("\u{2211}", Act::Make(Make::Sum), "Summation with range slots"),
                ("\u{220F}", Act::Make(Make::Prod), "Product with range slots"),
                ("lim", Act::Make(Make::Limit), "Limit"),
                ("d/dx", Act::Make(Make::Deriv), "Derivative"),
                ("a/b", Act::Make(Make::Frac), "Fraction"),
                ("x\u{207F}", Act::Make(Make::Pow), "Power"),
                ("x\u{2099}", Act::Make(Make::Sub), "Subscript"),
                ("\u{221A}", Act::Make(Make::Sqrt), "Square root"),
                ("\u{207F}\u{221A}", Act::Make(Make::NthRoot), "n-th root"),
                ("|x|", Act::Make(Make::Abs), "Absolute value"),
                ("[ ]", Act::Make(Make::Matrix2), "2\u{00D7}2 matrix"),
                ("[3]", Act::Make(Make::Matrix3), "3\u{00D7}3 matrix"),
            ]),
            grp!("Operators — wrap the selected slot", [
                ("+", Act::Make(Make::Plus), "Addition"),
                ("\u{2212}", Act::Make(Make::Minus), "Subtraction"),
                ("\u{00B7}", Act::Make(Make::Times), "Multiplication"),
                ("\u{00F7}", Act::Make(Make::Divide), "Division"),
            ]),
            grp!("Definition and evaluation", [
                (":=", Act::Make(Make::Define), "Define a name"),
                ("=", Act::Make(Make::Evaluate), "Evaluate the object"),
                ("\u{2192}", Act::Make(Make::Convert), "Convert units"),
            ]),
        ],
        1 => vec![
            grp!("Greek — drops into the selected slot", [
                ("\u{03B1}", Act::Sym("alpha"), "alpha"), ("\u{03B2}", Act::Sym("beta"), "beta"),
                ("\u{03B3}", Act::Sym("gamma"), "gamma"), ("\u{03B4}", Act::Sym("delta"), "delta"),
                ("\u{03B5}", Act::Sym("epsilon"), "epsilon"), ("\u{03B8}", Act::Sym("theta"), "theta"),
                ("\u{03BB}", Act::Sym("lambda"), "lambda"), ("\u{03BC}", Act::Sym("mu"), "mu"),
                ("\u{03BD}", Act::Sym("nu"), "nu"), ("\u{03BE}", Act::Sym("xi"), "xi"),
                ("\u{03C0}", Act::Sym("pi"), "pi"), ("\u{03C1}", Act::Sym("rho"), "rho"),
                ("\u{03C3}", Act::Sym("sigma"), "sigma"), ("\u{03C4}", Act::Sym("tau"), "tau"),
                ("\u{03C6}", Act::Sym("phi"), "phi"), ("\u{03C7}", Act::Sym("chi"), "chi"),
                ("\u{03C8}", Act::Sym("psi"), "psi"), ("\u{03C9}", Act::Sym("omega"), "omega"),
            ]),
            grp!("Upper case", [
                ("\u{0393}", Act::Sym("Gamma"), "Gamma"), ("\u{0394}", Act::Sym("Delta"), "Delta"),
                ("\u{0398}", Act::Sym("Theta"), "Theta"), ("\u{039B}", Act::Sym("Lambda"), "Lambda"),
                ("\u{039E}", Act::Sym("Xi"), "Xi"), ("\u{03A3}", Act::Sym("Sigma"), "Sigma"),
                ("\u{03A6}", Act::Sym("Phi"), "Phi"), ("\u{03A8}", Act::Sym("Psi"), "Psi"),
                ("\u{03A9}", Act::Sym("Omega"), "Omega"),
            ]),
            grp!("Variables and constants", [
                ("x", Act::Sym("x"), "x"), ("y", Act::Sym("y"), "y"), ("z", Act::Sym("z"), "z"),
                ("t", Act::Sym("t"), "t"), ("n", Act::Sym("n"), "n"), ("i", Act::Sym("i"), "imaginary unit"),
                ("e", Act::Sym("e"), "Euler number"), ("\u{221E}", Act::Sym("inf"), "infinity"),
            ]),
        ],
        2 => vec![
            grp!("Mathematical", [
                ("\u{03C0}", Act::Sym("pi"), "3.14159265…"), ("e", Act::Sym("e"), "2.71828182…"),
                ("i", Act::Sym("i"), "imaginary unit"), ("\u{221E}", Act::Sym("inf"), "infinity"),
            ]),
            grp!("Numbers", [
                ("0", Act::Sym("0"), "zero"), ("1", Act::Sym("1"), "one"), ("2", Act::Sym("2"), "two"),
                ("3", Act::Sym("3"), "three"), ("4", Act::Sym("4"), "four"), ("5", Act::Sym("5"), "five"),
                ("6", Act::Sym("6"), "six"), ("8", Act::Sym("8"), "eight"), ("10", Act::Sym("10"), "ten"),
                ("100", Act::Sym("100"), "hundred"), ("\u{00BD}", Act::Sym("0.5"), "one half"),
            ]),
        ],
        3 => vec![
            grp!("Calculus objects", [
                ("\u{222B}", Act::Make(Make::Integral), "Integral"),
                ("\u{2211}", Act::Make(Make::Sum), "Sum"),
                ("\u{220F}", Act::Make(Make::Prod), "Product"),
                ("lim", Act::Make(Make::Limit), "Limit"),
                ("d/dx", Act::Make(Make::Deriv), "Derivative"),
            ]),
            grp!("Algebra objects", [
                ("a/b", Act::Make(Make::Frac), "Fraction"), ("x\u{207F}", Act::Make(Make::Pow), "Power"),
                ("\u{221A}", Act::Make(Make::Sqrt), "Root"), ("\u{207F}\u{221A}", Act::Make(Make::NthRoot), "n-th root"),
                ("|x|", Act::Make(Make::Abs), "Magnitude"), ("[ ]", Act::Make(Make::Matrix2), "Matrix"),
            ]),
        ],
        4 => vec![
            grp!("Length · volume", [
                ("m", Act::Unit(" m"), "metre"), ("mm", Act::Unit(" mm"), "millimetre"),
                ("cm", Act::Unit(" cm"), "centimetre"), ("km", Act::Unit(" km"), "kilometre"),
                ("in", Act::Unit(" in"), "inch"), ("ft", Act::Unit(" ft"), "foot"), ("L", Act::Unit(" L"), "litre"),
            ]),
            grp!("Mass · time", [
                ("kg", Act::Unit(" kg"), "kilogram"), ("g", Act::Unit(" g"), "gram"),
                ("lb", Act::Unit(" lb"), "pound"), ("s", Act::Unit(" s"), "second"),
                ("ms", Act::Unit(" ms"), "millisecond"), ("min", Act::Unit(" min"), "minute"),
                ("h", Act::Unit(" h"), "hour"),
            ]),
            grp!("Force · pressure · energy", [
                ("N", Act::Unit(" N"), "newton"), ("kN", Act::Unit(" kN"), "kilonewton"),
                ("Pa", Act::Unit(" Pa"), "pascal"), ("MPa", Act::Unit(" MPa"), "megapascal"),
                ("GPa", Act::Unit(" GPa"), "gigapascal"), ("J", Act::Unit(" J"), "joule"),
                ("kJ", Act::Unit(" kJ"), "kilojoule"), ("W", Act::Unit(" W"), "watt"),
                ("kW", Act::Unit(" kW"), "kilowatt"),
            ]),
            grp!("Electrical · thermal · angle", [
                ("A", Act::Unit(" A"), "ampere"), ("V", Act::Unit(" V"), "volt"),
                ("\u{03A9}", Act::Unit(" ohm"), "ohm"), ("Hz", Act::Unit(" Hz"), "hertz"),
                ("K", Act::Unit(" K"), "kelvin"), ("degC", Act::Unit(" degC"), "Celsius"),
                ("rad", Act::Unit(" rad"), "radian"), ("deg", Act::Unit(" deg"), "degree"),
            ]),
        ],
        _ => vec![
            grp!("Trigonometric — fills the selected slot", [
                ("sin", Act::Sym("sin"), "sine"), ("cos", Act::Sym("cos"), "cosine"), ("tan", Act::Sym("tan"), "tangent"),
                ("asin", Act::Sym("asin"), "inverse sine"), ("acos", Act::Sym("acos"), "inverse cosine"),
                ("atan", Act::Sym("atan"), "inverse tangent"), ("sinh", Act::Sym("sinh"), "hyperbolic sine"),
                ("cosh", Act::Sym("cosh"), "hyperbolic cosine"), ("tanh", Act::Sym("tanh"), "hyperbolic tangent"),
            ]),
            grp!("Exponential · logarithmic", [
                ("exp", Act::Sym("exp"), "exponential"), ("ln", Act::Sym("ln"), "natural log"),
                ("log10", Act::Sym("log10"), "base-10 log"), ("log2", Act::Sym("log2"), "base-2 log"),
            ]),
            grp!("Structure", [
                ("\u{222B}", Act::Make(Make::Integral), "Integral"), ("a/b", Act::Make(Make::Frac), "Fraction"),
                ("\u{221A}", Act::Make(Make::Sqrt), "Root"), ("x\u{207F}", Act::Make(Make::Pow), "Power"),
                ("[ ]", Act::Make(Make::Matrix2), "Matrix"), ("\u{2211}", Act::Make(Make::Sum), "Sum"),
            ]),
            grp!("Statistics", [
                ("mean", Act::Sym("mean"), "mean"), ("median", Act::Sym("median"), "median"),
                ("std", Act::Sym("std"), "standard deviation"), ("sum", Act::Sym("sum"), "sum"),
                ("min", Act::Sym("min"), "minimum"), ("max", Act::Sym("max"), "maximum"),
            ]),
        ],
    }
}

pub fn show(ctx: &egui::Context, state: &mut UiState, actions: &mut Vec<UiAction>) {
    let Some(kind) = state.palette else { state.palette_rect = None; return };
    let t = theme::current(ctx);
    let anchor = state.palette_anchor;
    let mut close = false;

    let area = egui::Area::new(egui::Id::new(("palette", kind)))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(t.page)
                .stroke(Stroke::new(1.0_f32, t.ribbon_edge))
                .rounding(6.0)
                .shadow(egui::epaint::Shadow { offset: Vec2::new(0.0, 8.0), blur: 22.0, spread: 0.0,
                    color: Color32::from_black_alpha(if t.night { 120 } else { 38 }) })
                .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                .show(ui, |ui| {
                    ui.set_max_width(430.0);
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                    theme::scroll(ui, &t, |ui| {
                        ui.set_max_height(430.0);
                        for group in groups(kind) {
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(group.title.to_uppercase()).size(9.5).strong().color(t.ink_faint));
                            ui.add_space(3.0);
                            ui.horizontal_wrapped(|ui| {
                                for e in group.entries {
                                    if cell(ui, e, &t).clicked() {
                                        match &e.act {
                                            Act::Make(m) => actions.push(UiAction::TemplateNew(*m)),
                                            Act::Sym(s) => actions.push(UiAction::TemplateLeaf(s.to_string(), false)),
                                            Act::Unit(s) => actions.push(UiAction::TemplateLeaf(s.to_string(), true)),
                                        }
                                    }
                                }
                            });
                            ui.add_space(7.0);
                        }
                    });
                });
        });

    let rect = area.response.rect;
    state.palette_rect = Some(rect);
    let buttons = state.palette_buttons.clone();
    if ctx.input(|i| i.pointer.any_click()) {
        if let Some(p) = ctx.pointer_interact_pos() {
            let on_toggle = buttons.iter().any(|(_, r)| r.contains(p));
            if !rect.expand(2.0).contains(p) && !on_toggle { close = true; }
        }
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { close = true; }
    if close { state.palette = None; state.palette_rect = None; }
}

fn cell(ui: &mut egui::Ui, e: &Entry, t: &Theme) -> egui::Response {
    let font = theme::math(15.0);
    let w = ui.fonts(|f| f.layout_no_wrap(e.glyph.to_string(), font.clone(), t.ink).size().x).max(22.0) + 18.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 32.0), Sense::click());
    let anim = ui.ctx().animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.1);
    if anim > 0.001 && anim < 0.999 { ui.ctx().request_repaint(); }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, t.card);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, t.card_edge));
    if anim > 0.001 {
        painter.rect_filled(rect, 4.0, t.accent_soft.linear_multiply(anim));
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, t.accent_line.linear_multiply(anim)));
    }
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, e.glyph, font, if resp.hovered() { t.accent_deep } else { t.ink });
    resp.on_hover_text(e.tip)
}

pub fn anchor_for(_ui: &egui::Ui, resp: &egui::Response) -> Pos2 {
    Pos2::new(resp.rect.left(), resp.rect.bottom() + 5.0)
}
