//! Mathcad-style grouped palettes rendered as a foreground dropdown.
use crate::state::{UiAction, UiState};
use crate::theme::{self, Theme};
use egui::{Color32, Pos2, Sense, Stroke, Vec2};

pub struct Entry { pub glyph: &'static str, pub snippet: &'static str, pub tip: &'static str }
pub struct Group { pub title: &'static str, pub entries: &'static [Entry] }

macro_rules! grp {
    ($title:expr, [ $(($g:expr, $s:expr, $t:expr)),* $(,)? ]) => {
        Group { title: $title, entries: &[ $(Entry { glyph: $g, snippet: $s, tip: $t }),* ] }
    };
}

pub fn groups(kind: usize) -> Vec<Group> {
    match kind {
        0 => vec![
            grp!("Algebra", [
                ("+", " + ", "Add"), ("\u{2212}", " - ", "Subtract"), ("\u{00B7}", "*", "Multiply"),
                ("\u{00F7}", "/", "Divide"), ("x\u{207F}", "^", "Power"), ("\u{221A}", "sqrt(", "Square root"),
                ("|x|", "abs(", "Absolute value"), ("x!", "!", "Factorial"), ("%", " mod ", "Modulus"),
                ("( )", "()", "Parentheses"), ("\u{207F}\u{221A}", "nroot(", "n-th root"), ("x\u{00B2}", "^2", "Square"),
            ]),
            grp!("Calculus", [
                ("d/dx", "diff(f(x), x)", "Derivative"), ("\u{222B}", "integrate(f, 0, 1)", "Definite integral"),
                ("\u{222B}dx", "integrate(f(x), x)", "Indefinite integral"), ("\u{03A3}", "sum(v)", "Summation"),
                ("\u{03A0}", "prod(v)", "Product"), ("lim", "limit(f(x), x, 0)", "Limit"),
                ("f\u{2032}", "nderiv(f, 1)", "Numeric derivative"),
            ]),
            grp!("Comparison", [
                ("=", " = ", "Equal"), ("\u{2260}", " != ", "Not equal"), ("<", " < ", "Less than"),
                ("\u{2264}", " <= ", "Less or equal"), (">", " > ", "Greater than"), ("\u{2265}", " >= ", "Greater or equal"),
                ("\u{2227}", " and ", "Logical and"), ("\u{2228}", " or ", "Logical or"), ("\u{00AC}", "not ", "Logical not"),
            ]),
            grp!("Definition and evaluation", [
                (":=", " := ", "Definition"), ("=", " = ", "Evaluation"),
                ("f(x)", "f(x) := ", "Function definition"), ("\u{2192}", "convert(", "Convert units"),
            ]),
            grp!("Vector and matrix", [
                ("\u{00D7}", "cross(", "Cross product"), ("\u{2016}v\u{2016}", "norm(", "Norm"),
                ("|M|", "det(", "Determinant"), ("M\u{1D40}", "transpose(", "Transpose"),
                ("M\u{207B}\u{00B9}", "inv(", "Inverse"), ("[ ]", "[[1, 2], [3, 4]]", "Matrix"),
                ("1..n", "1..10", "Range"), ("rank", "rank(", "Rank"),
            ]),
        ],
        1 => vec![
            grp!("Greek (lower)", [
                ("\u{03B1}", "alpha", "alpha"), ("\u{03B2}", "beta", "beta"), ("\u{03B3}", "gamma", "gamma"),
                ("\u{03B4}", "delta", "delta"), ("\u{03B5}", "epsilon", "epsilon"), ("\u{03B8}", "theta", "theta"),
                ("\u{03BB}", "lambda", "lambda"), ("\u{03BC}", "mu", "mu"), ("\u{03BD}", "nu", "nu"),
                ("\u{03BE}", "xi", "xi"), ("\u{03C0}", "pi", "pi"), ("\u{03C1}", "rho", "rho"),
                ("\u{03C3}", "sigma", "sigma"), ("\u{03C4}", "tau", "tau"), ("\u{03C6}", "phi", "phi"),
                ("\u{03C7}", "chi", "chi"), ("\u{03C8}", "psi", "psi"), ("\u{03C9}", "omega", "omega"),
            ]),
            grp!("Greek (upper)", [
                ("\u{0393}", "Gamma", "Gamma"), ("\u{0394}", "Delta", "Delta"), ("\u{0398}", "Theta", "Theta"),
                ("\u{039B}", "Lambda", "Lambda"), ("\u{039E}", "Xi", "Xi"), ("\u{03A3}", "Sigma", "Sigma"),
                ("\u{03A6}", "Phi", "Phi"), ("\u{03A8}", "Psi", "Psi"), ("\u{03A9}", "Omega", "Omega"),
            ]),
            grp!("Special", [
                ("\u{221E}", "inf", "Infinity"), ("\u{2202}", "diff(", "Partial derivative"),
                ("x\u{2081}", "_1", "Subscript"), ("\u{00B0}", " deg", "Degree"), ("i", "i", "Imaginary unit"),
            ]),
        ],
        2 => vec![
            grp!("Mathematical", [
                ("\u{03C0}", "pi", "3.14159265…"), ("e", "e", "2.71828182…"),
                ("i", "i", "Imaginary unit"), ("\u{221E}", "inf", "Infinity"),
            ]),
            grp!("Physical (inserts a definition)", [
                ("g", "g := 9.80665 m/s^2", "Standard gravity"),
                ("c", "c0 := 299792458 m/s", "Speed of light"),
                ("R", "R := 8.314462618 J/(mol*K)", "Gas constant"),
                ("k", "k := 1.380649e-23 J/K", "Boltzmann"),
                ("N\u{2090}", "NA := 6.02214076e23 1/mol", "Avogadro"),
                ("\u{03C3}", "sb := 5.670374419e-8 W/(m^2*K^4)", "Stefan-Boltzmann"),
            ]),
        ],
        3 => vec![
            grp!("Conditional", [
                ("if", "if x > 0 then 1 else 0", "Conditional"), ("if()", "if(x > 0, 1, 0)", "Inline conditional"),
                ("and", " and ", "Logical and"), ("or", " or ", "Logical or"), ("not", "not ", "Logical not"),
            ]),
            grp!("Iteration helpers", [
                ("range", "range(0, 1, 10)", "Sequence"), ("linspace", "linspace(0, 1, 100)", "Samples"),
                ("\u{03A3}", "sum(v)", "Sum"), ("\u{03A0}", "prod(v)", "Product"),
            ]),
            grp!("Scripting", [
                ("import", "data := read_csv(\"data.csv\")", "Import dataset"),
            ]),
        ],
        4 => vec![
            grp!("Length · volume", [
                ("m", " m", "metre"), ("mm", " mm", "millimetre"), ("cm", " cm", "centimetre"),
                ("km", " km", "kilometre"), ("in", " in", "inch"), ("ft", " ft", "foot"),
                ("mi", " mi", "mile"), ("L", " L", "litre"), ("gal", " gal", "US gallon"),
            ]),
            grp!("Mass · time", [
                ("kg", " kg", "kilogram"), ("g", " g", "gram"), ("lb", " lb", "pound"),
                ("t", " t", "tonne"), ("s", " s", "second"), ("ms", " ms", "millisecond"),
                ("min", " min", "minute"), ("h", " h", "hour"), ("day", " day", "day"),
            ]),
            grp!("Force · pressure · energy", [
                ("N", " N", "newton"), ("kN", " kN", "kilonewton"), ("Pa", " Pa", "pascal"),
                ("kPa", " kPa", "kilopascal"), ("MPa", " MPa", "megapascal"), ("GPa", " GPa", "gigapascal"),
                ("bar", " bar", "bar"), ("psi", " psi", "psi"), ("J", " J", "joule"),
                ("kJ", " kJ", "kilojoule"), ("kWh", " kWh", "kilowatt hour"), ("W", " W", "watt"),
                ("kW", " kW", "kilowatt"), ("hp", " hp", "horsepower"),
            ]),
            grp!("Electrical · thermal · angle", [
                ("A", " A", "ampere"), ("V", " V", "volt"), ("\u{03A9}", " ohm", "ohm"),
                ("F", " F", "farad"), ("H", " H", "henry"), ("Hz", " Hz", "hertz"),
                ("K", " K", "kelvin"), ("degC", " degC", "Celsius"), ("degF", " degF", "Fahrenheit"),
                ("rad", " rad", "radian"), ("deg", " deg", "degree"),
            ]),
            grp!("Conversion", [
                ("\u{2192}mm", "convert(delta, mm)", "To millimetres"), ("SI", "si(x)", "SI base value"),
                ("strip", "strip_units(x)", "Numeric value"), ("unit", "unit_of(x)", "Unit of"),
            ]),
        ],
        _ => vec![
            grp!("Trigonometric", [
                ("sin", "sin(", "Sine"), ("cos", "cos(", "Cosine"), ("tan", "tan(", "Tangent"),
                ("asin", "asin(", "Inverse sine"), ("acos", "acos(", "Inverse cosine"), ("atan", "atan(", "Inverse tangent"),
                ("atan2", "atan2(", "Two-argument atan"), ("sec", "sec(", "Secant"),
                ("csc", "csc(", "Cosecant"), ("cot", "cot(", "Cotangent"), ("sinc", "sinc(", "sin(x)/x"),
            ]),
            grp!("Hyperbolic", [
                ("sinh", "sinh(", "sinh"), ("cosh", "cosh(", "cosh"), ("tanh", "tanh(", "tanh"),
                ("sech", "sech(", "sech"), ("csch", "csch(", "csch"), ("coth", "coth(", "coth"),
                ("asinh", "asinh(", "asinh"), ("acosh", "acosh(", "acosh"), ("atanh", "atanh(", "atanh"),
            ]),
            grp!("Exponential · logarithmic", [
                ("exp", "exp(", "Exponential"), ("ln", "ln(", "Natural log"), ("log", "log(", "Log base b"),
                ("log10", "log10(", "Base-10"), ("log2", "log2(", "Base-2"),
                ("log1p", "log1p(", "ln(1+x)"), ("expm1", "expm1(", "exp(x)-1"),
            ]),
            grp!("Rounding · number theory", [
                ("floor", "floor(", "Round down"), ("ceil", "ceil(", "Round up"), ("round", "round(", "Round"),
                ("trunc", "trunc(", "Truncate"), ("frac", "frac(", "Fractional part"), ("sign", "sign(", "Sign"),
                ("clamp", "clamp(", "Clamp"), ("gcd", "gcd(", "GCD"), ("lcm", "lcm(", "LCM"),
            ]),
            grp!("Statistics", [
                ("mean", "mean(", "Mean"), ("median", "median(", "Median"), ("std", "std(", "Std deviation"),
                ("var", "var(", "Variance"), ("geomean", "geomean(", "Geometric mean"), ("harmmean", "harmmean(", "Harmonic mean"),
                ("mode", "mode(", "Mode"), ("rms", "rms(", "RMS"), ("percentile", "percentile(v, 95)", "Percentile"),
                ("corr", "corr(", "Correlation"), ("cov", "cov(", "Covariance"), ("skew", "skew(", "Skewness"),
                ("kurt", "kurt(", "Kurtosis"),
            ]),
            grp!("Vector · matrix", [
                ("augment", "augment(A, B)", "Join side by side"), ("stack", "stack(A, B)", "Join vertically"),
                ("submatrix", "submatrix(A, 0, 1, 0, 1)", "Extract block"), ("row", "row(A, 0)", "Row"),
                ("col", "col(A, 0)", "Column"), ("vec", "vec(A)", "Flatten"), ("reshape", "reshape(v, 2, 3)", "Reshape"),
                ("cumsum", "cumsum(", "Cumulative sum"), ("cumprod", "cumprod(", "Cumulative product"),
                ("logspace", "logspace(0, 3, 10)", "Log samples"), ("last", "last(", "Last element"),
                ("diag", "diag(", "Diagonal"), ("identity", "identity(3)", "Identity"),
            ]),
            grp!("Interpolation · fitting", [
                ("linterp", "linterp(x, y, 0.5)", "Linear interpolation"), ("polyfit", "polyfit(x, y, 2)", "Polynomial fit"),
                ("polyval", "polyval(c, 1.5)", "Evaluate polynomial"), ("linfit", "linfit(x, y)", "Line fit"),
                ("expfit", "expfit(x, y)", "Exponential fit"), ("logfit", "logfit(x, y)", "Log fit"),
                ("pwrfit", "pwrfit(x, y)", "Power-law fit"),
            ]),
            grp!("Solvers · ODE", [
                ("root", "root(f, 0, 1)", "Bracketed root"), ("newton", "newton(f, 1)", "Newton root"),
                ("minimize", "minimize(f, 0, 1)", "Minimise"), ("maximize", "maximize(f, 0, 1)", "Maximise"),
                ("odesolve", "odesolve(f, 0, 1, 1, 200)", "Initial-value ODE"), ("integrate", "integrate(f, 0, 1)", "Integral"),
            ]),
            grp!("Random", [
                ("seed", "seed(42)", "Seed"), ("runif", "runif(100, 0, 1)", "Uniform"), ("rnorm", "rnorm(100, 0, 1)", "Normal"),
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
                .shadow(egui::epaint::Shadow {
                    offset: Vec2::new(0.0, 8.0), blur: 22.0, spread: 0.0,
                    color: Color32::from_black_alpha(if t.night { 120 } else { 38 }),
                })
                .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                .show(ui, |ui| {
                    ui.set_max_width(420.0);
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                    theme::scroll(ui, &t, |ui| {
                        ui.set_max_height(420.0);
                        for group in groups(kind) {
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(group.title.to_uppercase()).size(9.5).strong().color(t.ink_faint));
                            ui.add_space(3.0);
                            ui.horizontal_wrapped(|ui| {
                                for e in group.entries {
                                    if cell(ui, e, &t).clicked() {
                                        actions.push(UiAction::InsertSnippet(e.snippet.to_string()));
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
    let font = theme::math(14.5);
    let w = ui.fonts(|f| f.layout_no_wrap(e.glyph.to_string(), font.clone(), t.ink).size().x).max(20.0) + 16.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 29.0), Sense::click());
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
    resp.on_hover_text(format!("{}\n{}", e.tip, e.snippet.trim()))
}

pub fn anchor_for(_ui: &egui::Ui, resp: &egui::Response) -> Pos2 {
    Pos2::new(resp.rect.left(), resp.rect.bottom() + 5.0)
}
