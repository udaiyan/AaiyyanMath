//! Structured mathematics, typeset inline, with every leaf editable in place.
//!
//! Box model about a math axis; scripts attach by rule; operators and delimiters
//! are drawn so ink extents are exact. Symbols and numbers are click-to-edit
//! leaves: typing `x := 3` into a leaf promotes it to a definition, and a
//! persistent name rail wraps any object as `name := object`.
use crate::state::{UiAction, UiState};
use crate::theme::{self, Theme};
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use uuid::Uuid;

pub type Slot = u32;

#[derive(Clone, Debug, PartialEq)]
pub enum Tpl {
    Hole(Slot),
    Num(String),
    Sym(String),
    Unit(String),
    Frac(Box<Tpl>, Box<Tpl>),
    Pow(Box<Tpl>, Box<Tpl>),
    Sub(Box<Tpl>, Box<Tpl>),
    Sqrt(Box<Tpl>),
    NthRoot(Box<Tpl>, Box<Tpl>),
    Abs(Box<Tpl>),
    Paren(Box<Tpl>),
    Integral(Box<Tpl>, Box<Tpl>, Box<Tpl>, Box<Tpl>),
    Sum(Box<Tpl>, Box<Tpl>, Box<Tpl>),
    Prod(Box<Tpl>, Box<Tpl>, Box<Tpl>),
    Limit(Box<Tpl>, Box<Tpl>, Box<Tpl>),
    Deriv(Box<Tpl>, Box<Tpl>, u32),
    Matrix(Vec<Vec<Tpl>>),
    Bin(String, Box<Tpl>, Box<Tpl>),
    Assign(String, Box<Tpl>),
    Eval(Box<Tpl>),
    Convert(Box<Tpl>, Box<Tpl>),
}

impl Tpl {
    pub fn slots(&self, out: &mut Vec<Slot>) {
        match self {
            Tpl::Hole(n) => out.push(*n),
            Tpl::Num(_) | Tpl::Sym(_) | Tpl::Unit(_) => {}
            Tpl::Frac(a, b) | Tpl::Pow(a, b) | Tpl::Sub(a, b) | Tpl::NthRoot(a, b)
            | Tpl::Bin(_, a, b) | Tpl::Convert(a, b) => { a.slots(out); b.slots(out); }
            Tpl::Sqrt(a) | Tpl::Abs(a) | Tpl::Paren(a) | Tpl::Eval(a) | Tpl::Assign(_, a) => a.slots(out),
            Tpl::Integral(a, b, c, d) => { a.slots(out); b.slots(out); c.slots(out); d.slots(out); }
            Tpl::Sum(a, b, c) | Tpl::Prod(a, b, c) | Tpl::Limit(a, b, c) => { a.slots(out); b.slots(out); c.slots(out); }
            Tpl::Deriv(a, b, _) => { a.slots(out); b.slots(out); }
            Tpl::Matrix(rows) => for r in rows { for c in r { c.slots(out); } },
        }
    }
    pub fn complete(&self) -> bool { let mut v = Vec::new(); self.slots(&mut v); v.is_empty() }
    pub fn max_slot(&self) -> Slot { let mut v = Vec::new(); self.slots(&mut v); v.into_iter().max().unwrap_or(0) }

    pub fn fill(&mut self, target: Slot, with: Tpl) -> bool {
        if let Tpl::Hole(n) = self { if *n == target { *self = with; return true; } return false; }
        match self {
            Tpl::Frac(a, b) | Tpl::Pow(a, b) | Tpl::Sub(a, b) | Tpl::NthRoot(a, b)
            | Tpl::Bin(_, a, b) | Tpl::Convert(a, b) => a.fill(target, with.clone()) || b.fill(target, with),
            Tpl::Sqrt(a) | Tpl::Abs(a) | Tpl::Paren(a) | Tpl::Eval(a) | Tpl::Assign(_, a) => a.fill(target, with),
            Tpl::Integral(a, b, c, d) => a.fill(target, with.clone()) || b.fill(target, with.clone()) || c.fill(target, with.clone()) || d.fill(target, with),
            Tpl::Sum(a, b, c) | Tpl::Prod(a, b, c) | Tpl::Limit(a, b, c) => a.fill(target, with.clone()) || b.fill(target, with.clone()) || c.fill(target, with),
            Tpl::Deriv(a, b, _) => a.fill(target, with.clone()) || b.fill(target, with),
            Tpl::Matrix(rows) => { for r in rows { for c in r { if c.fill(target, with.clone()) { return true; } } } false }
            _ => false,
        }
    }

    pub fn get_path(&self, path: &[usize]) -> Option<&Tpl> {
        if path.is_empty() { return Some(self); }
        let kids: Vec<&Tpl> = match self {
            Tpl::Frac(a, b) | Tpl::Pow(a, b) | Tpl::Sub(a, b) | Tpl::NthRoot(a, b)
            | Tpl::Bin(_, a, b) | Tpl::Convert(a, b) => vec![a, b],
            Tpl::Sqrt(a) | Tpl::Abs(a) | Tpl::Paren(a) | Tpl::Eval(a) | Tpl::Assign(_, a) => vec![a],
            Tpl::Integral(a, b, c, d) => vec![a, b, c, d],
            Tpl::Sum(a, b, c) | Tpl::Prod(a, b, c) | Tpl::Limit(a, b, c) => vec![a, b, c],
            Tpl::Deriv(a, b, _) => vec![a, b],
            Tpl::Matrix(rows) => rows.iter().flatten().collect(),
            _ => vec![],
        };
        kids.get(path[0]).and_then(|k| k.get_path(&path[1..]))
    }

    pub fn set_path(&mut self, path: &[usize], with: Tpl) -> bool {
        if path.is_empty() { *self = with; return true; }
        let i = path[0];
        let rest = &path[1..];
        match self {
            Tpl::Frac(a, b) | Tpl::Pow(a, b) | Tpl::Sub(a, b) | Tpl::NthRoot(a, b)
            | Tpl::Bin(_, a, b) | Tpl::Convert(a, b) => match i { 0 => a.set_path(rest, with), 1 => b.set_path(rest, with), _ => false },
            Tpl::Sqrt(a) | Tpl::Abs(a) | Tpl::Paren(a) | Tpl::Eval(a) | Tpl::Assign(_, a) => if i == 0 { a.set_path(rest, with) } else { false },
            Tpl::Integral(a, b, c, d) => match i { 0 => a.set_path(rest, with), 1 => b.set_path(rest, with), 2 => c.set_path(rest, with), 3 => d.set_path(rest, with), _ => false },
            Tpl::Sum(a, b, c) | Tpl::Prod(a, b, c) | Tpl::Limit(a, b, c) => match i { 0 => a.set_path(rest, with), 1 => b.set_path(rest, with), 2 => c.set_path(rest, with), _ => false },
            Tpl::Deriv(a, b, _) => match i { 0 => a.set_path(rest, with), 1 => b.set_path(rest, with), _ => false },
            Tpl::Matrix(rows) => {
                let mut k = 0;
                for r in rows { for c in r { if k == i { return c.set_path(rest, with); } k += 1; } }
                false
            }
            _ => false,
        }
    }

    pub fn to_source(&self) -> String {
        match self {
            Tpl::Hole(n) => format!("hole{n}"),
            Tpl::Num(s) | Tpl::Sym(s) | Tpl::Unit(s) => s.clone(),
            Tpl::Frac(a, b) => format!("({})/({})", a.to_source(), b.to_source()),
            Tpl::Pow(a, b) => format!("({})^({})", a.to_source(), b.to_source()),
            Tpl::Sub(a, b) => format!("{}_{}", a.to_source(), b.to_source()),
            Tpl::Sqrt(a) => format!("sqrt({})", a.to_source()),
            Tpl::NthRoot(a, b) => format!("nroot({}, {})", a.to_source(), b.to_source()),
            Tpl::Abs(a) => format!("abs({})", a.to_source()),
            Tpl::Paren(a) => format!("({})", a.to_source()),
            Tpl::Integral(f, lo, hi, _) => format!("integrate({}, {}, {})", f.to_source(), lo.to_source(), hi.to_source()),
            Tpl::Sum(lo, hi, b) => format!("sumrange({}, {}, {})", lo.to_source(), hi.to_source(), b.to_source()),
            Tpl::Prod(lo, hi, b) => format!("prodrange({}, {}, {})", lo.to_source(), hi.to_source(), b.to_source()),
            Tpl::Limit(v, to, b) => format!("limit({}, {}, {})", b.to_source(), v.to_source(), to.to_source()),
            Tpl::Deriv(f, v, n) => format!("nderiv({}, {}, {})", f.to_source(), v.to_source(), n),
            Tpl::Matrix(rows) => {
                let rs: Vec<String> = rows.iter().map(|r| format!("[{}]", r.iter().map(|c| c.to_source()).collect::<Vec<_>>().join(", "))).collect();
                format!("[{}]", rs.join(", "))
            }
            Tpl::Bin(op, a, b) => format!("({}) {} ({})", a.to_source(), op, b.to_source()),
            Tpl::Assign(name, v) => format!("{} := {}", name, v.to_source()),
            Tpl::Eval(a) => format!("{} =", a.to_source()),
            Tpl::Convert(a, u) => format!("convert({}, {})", a.to_source(), u.to_source()),
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Tpl::Hole(_) => "empty slot", Tpl::Num(_) => "number", Tpl::Sym(_) => "symbol", Tpl::Unit(_) => "unit",
            Tpl::Frac(..) => "fraction", Tpl::Pow(..) => "power", Tpl::Sub(..) => "subscript",
            Tpl::Sqrt(_) => "square root", Tpl::NthRoot(..) => "n-th root", Tpl::Abs(_) => "magnitude", Tpl::Paren(_) => "group",
            Tpl::Integral(..) => "integral", Tpl::Sum(..) => "summation", Tpl::Prod(..) => "product", Tpl::Limit(..) => "limit",
            Tpl::Deriv(..) => "derivative", Tpl::Matrix(_) => "matrix", Tpl::Bin(..) => "operation",
            Tpl::Assign(..) => "definition", Tpl::Eval(_) => "evaluation", Tpl::Convert(..) => "conversion",
        }
    }
}

pub fn glyph(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "\u{03B1}", "beta" => "\u{03B2}", "gamma" => "\u{03B3}", "delta" => "\u{03B4}",
        "epsilon" => "\u{03B5}", "zeta" => "\u{03B6}", "eta" => "\u{03B7}", "theta" => "\u{03B8}",
        "iota" => "\u{03B9}", "kappa" => "\u{03BA}", "lambda" => "\u{03BB}", "mu" => "\u{03BC}",
        "nu" => "\u{03BD}", "xi" => "\u{03BE}", "pi" => "\u{03C0}", "rho" => "\u{03C1}",
        "sigma" => "\u{03C3}", "tau" => "\u{03C4}", "upsilon" => "\u{03C5}", "phi" => "\u{03C6}",
        "chi" => "\u{03C7}", "psi" => "\u{03C8}", "omega" => "\u{03C9}",
        "Gamma" => "\u{0393}", "Delta" => "\u{0394}", "Theta" => "\u{0398}", "Lambda" => "\u{039B}",
        "Xi" => "\u{039E}", "Pi" => "\u{03A0}", "Sigma" => "\u{03A3}", "Phi" => "\u{03A6}",
        "Psi" => "\u{03A8}", "Omega" => "\u{03A9}",
        "inf" | "infty" => "\u{221E}", "deg" => "\u{00B0}", "ohm" => "\u{03A9}",
        _ => return None,
    })
}

const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh",
    "sec", "csc", "cot", "sinc", "ln", "log", "log10", "log2", "exp", "sqrt", "nroot",
    "abs", "sign", "floor", "ceil", "round", "trunc", "frac", "sum", "prod", "mean",
    "median", "std", "var", "min", "max", "convert", "integrate", "diff", "nderiv",
    "limit", "root", "newton", "minimize", "maximize", "polyfit", "polyval", "linterp",
];
fn is_function(n: &str) -> bool { FUNCTIONS.contains(&n) }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Make {
    Integral, Sum, Prod, Limit, Deriv, Frac, Pow, Sub, Sqrt, NthRoot, Abs,
    Matrix2, Matrix3, Convert, Define, Evaluate, Plus, Minus, Times, Divide, Paren, Leaf,
}

pub fn build(kind: Make, first: Slot) -> Tpl {
    let h = |i: u32| Box::new(Tpl::Hole(first + i));
    match kind {
        Make::Integral => Tpl::Integral(h(0), h(1), h(2), Box::new(Tpl::Sym("x".into()))),
        Make::Sum => Tpl::Sum(h(0), h(1), h(2)),
        Make::Prod => Tpl::Prod(h(0), h(1), h(2)),
        Make::Limit => Tpl::Limit(h(0), h(1), h(2)),
        Make::Deriv => Tpl::Deriv(h(0), h(1), 1),
        Make::Frac => Tpl::Frac(h(0), h(1)),
        Make::Pow => Tpl::Pow(h(0), h(1)),
        Make::Sub => Tpl::Sub(h(0), h(1)),
        Make::Sqrt => Tpl::Sqrt(h(0)),
        Make::NthRoot => Tpl::NthRoot(h(0), h(1)),
        Make::Abs => Tpl::Abs(h(0)),
        Make::Paren => Tpl::Paren(h(0)),
        Make::Convert => Tpl::Convert(h(0), h(1)),
        Make::Matrix2 => Tpl::Matrix(vec![vec![Tpl::Hole(first), Tpl::Hole(first + 1)], vec![Tpl::Hole(first + 2), Tpl::Hole(first + 3)]]),
        Make::Matrix3 => Tpl::Matrix((0..3).map(|r| (0..3).map(|c| Tpl::Hole(first + r * 3 + c)).collect()).collect()),
        Make::Define => Tpl::Assign("f".into(), h(0)),
        Make::Evaluate => Tpl::Eval(h(0)),
        Make::Plus => Tpl::Bin("+".into(), h(0), h(1)),
        Make::Minus => Tpl::Bin("-".into(), h(0), h(1)),
        Make::Times => Tpl::Bin("*".into(), h(0), h(1)),
        Make::Divide => Tpl::Bin("/".into(), h(0), h(1)),
        Make::Leaf => Tpl::Hole(first),
    }
}

// ---------------------------------------------------------------- box model
#[derive(Clone)]
enum Item {
    Glyph(String, Rect, FontId),
    Rule(Rect),
    Path(Vec<Pos2>, f32),
    Slot(Slot, Rect, String, f32),
    Delim(Rect, bool),
    Leaf(Vec<usize>, Rect, String, f32),
}

impl Item {
    fn bounds(&self) -> Rect {
        match self {
            Item::Glyph(_, r, _) | Item::Rule(r) | Item::Slot(_, r, _, _) | Item::Delim(r, _) | Item::Leaf(_, r, _, _) => *r,
            Item::Path(pts, w) => {
                let f = pts.first().copied().unwrap_or(Pos2::ZERO);
                let mut r = Rect::from_min_max(f, f);
                for p in pts { r.extend_with(*p); }
                r.expand(*w * 0.5)
            }
        }
    }
    fn shift(&mut self, d: Vec2) {
        match self {
            Item::Glyph(_, r, _) | Item::Rule(r) | Item::Slot(_, r, _, _) | Item::Delim(r, _) | Item::Leaf(_, r, _, _) => *r = r.translate(d),
            Item::Path(pts, _) => { for p in pts { *p += d; } }
        }
    }
}

#[derive(Clone)]
struct B { w: f32, h: f32, d: f32, items: Vec<Item> }
impl B {
    fn moved(mut self, dx: f32, dy: f32) -> B {
        for i in &mut self.items { i.shift(Vec2::new(dx, dy)); }
        self.h -= dy; self.d += dy; self
    }
}

fn finish(items: Vec<Item>) -> B {
    if items.is_empty() { return B { w: 0.0, h: 0.0, d: 0.0, items }; }
    let mut r = items[0].bounds();
    for i in items.iter().skip(1) { r = r.union(i.bounds()); }
    let dx = -r.left();
    let mut items = items;
    for i in &mut items { i.shift(Vec2::new(dx, 0.0)); }
    B { w: r.width().max(0.0), h: (-r.top()).max(0.0), d: r.bottom().max(0.0), items }
}

fn seq(parts: Vec<B>, gap: f32) -> B {
    let mut x = 0.0;
    let mut items = Vec::new();
    for p in parts { let w = p.w; let p = p.moved(x, 0.0); items.extend(p.items); x += w + gap; }
    finish(items)
}

#[derive(Clone, Copy)]
struct Ctx { script: f32, spacing: f32, rule: f32, pad: f32 }
impl Ctx {
    fn from(state: &UiState) -> Ctx {
        Ctx { script: state.script_scale.clamp(0.45, 0.95), spacing: state.math_spacing.clamp(0.5, 1.8),
              rule: state.rule_scale.clamp(0.4, 2.4), pad: state.slot_pad.clamp(0.3, 2.2) }
    }
}
struct Slots<'a> { focused: Option<Slot>, buffer: &'a str }

fn measure(ui: &Ui, s: &str, f: &FontId) -> Vec2 {
    ui.fonts(|g| g.layout_no_wrap(s.to_string(), f.clone(), Color32::BLACK).size())
}

fn glyph_box(ui: &Ui, text: &str, s: f32, italic: bool) -> B {
    let f = if italic { theme::math_italic(s) } else { theme::math(s) };
    let sz = measure(ui, text, &f);
    let asc = sz.y * 0.80; let desc = sz.y * 0.20;
    B { w: sz.x, h: asc, d: desc,
        items: vec![Item::Glyph(text.to_string(), Rect::from_min_size(Pos2::new(0.0, -asc), sz), f)] }
}

fn leaf_box(ui: &Ui, path: Vec<usize>, text: &str, s: f32, italic: bool) -> B {
    let f = if italic { theme::math_italic(s) } else { theme::math(s) };
    let sz = measure(ui, text, &f);
    let asc = sz.y * 0.80; let desc = sz.y * 0.20;
    B { w: sz.x, h: asc, d: desc,
        items: vec![Item::Leaf(path, Rect::from_min_size(Pos2::new(0.0, -asc), sz), text.to_string(), s)] }
}

fn slot_box(ui: &Ui, slot: Slot, s: f32, hint: &str, sl: &Slots<'_>, c: &Ctx) -> B {
    let ps = s * 0.84;
    let f = theme::math(ps);
    let active = sl.focused == Some(slot);
    let shown: &str = if active && !sl.buffer.is_empty() { sl.buffer } else { hint };
    let tw = measure(ui, shown, &f).x;
    let w = (tw + s * 0.50 * c.pad).max(s * 0.80).max(13.0);
    let h = s * 1.00; let top = -h * 0.78;
    B { w, h: -top, d: h + top,
        items: vec![Item::Slot(slot, Rect::from_min_size(Pos2::new(0.0, top), Vec2::new(w, h)), shown.to_string(), ps)] }
}

fn bezier(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, n: usize) -> Vec<Pos2> {
    (0..=n).map(|i| {
        let t = i as f32 / n as f32; let u = 1.0 - t;
        let (a, b, c, d) = (u*u*u, 3.0*u*u*t, 3.0*u*t*t, t*t*t);
        Pos2::new(a*p0.x + b*p1.x + c*p2.x + d*p3.x, a*p0.y + b*p1.y + c*p2.y + d*p3.y)
    }).collect()
}

fn integral_path(w: f32, top: f32, bot: f32) -> Vec<Pos2> {
    let h = bot - top; let cx = w * 0.52; let st = top + h * 0.17; let sb = bot - h * 0.17;
    let mut p = bezier(Pos2::new(w*0.96, top+h*0.062), Pos2::new(w*0.88, top-h*0.016), Pos2::new(w*0.60, top+h*0.010), Pos2::new(cx+w*0.05, st), 16);
    let stem = bezier(Pos2::new(cx+w*0.05, st), Pos2::new(cx+w*0.17, top+h*0.42), Pos2::new(cx-w*0.17, bot-h*0.42), Pos2::new(cx-w*0.05, sb), 20);
    p.extend(stem.into_iter().skip(1));
    let hook = bezier(Pos2::new(cx-w*0.05, sb), Pos2::new(cx-w*0.12, bot-h*0.010), Pos2::new(w*0.16, bot+h*0.016), Pos2::new(w*0.04, bot-h*0.062), 16);
    p.extend(hook.into_iter().skip(1));
    p
}

fn paint_delim(p: &egui::Painter, r: Rect, square: bool, sw: f32, col: Color32) {
    let st = Stroke::new(sw, col);
    if square {
        let k = r.width() * 0.62;
        p.line_segment([r.right_top() - Vec2::new(k, 0.0), r.left_top()], st);
        p.line_segment([r.left_top(), r.left_bottom()], st);
        p.line_segment([r.left_bottom(), r.right_bottom() - Vec2::new(k, 0.0)], st);
    } else {
        let n = 18;
        let pts: Vec<Pos2> = (0..=n).map(|i| {
            let t = i as f32 / n as f32;
            let bow = (t * std::f32::consts::PI).sin();
            Pos2::new(r.right() - bow * r.width(), r.top() + t * r.height())
        }).collect();
        p.add(egui::Shape::line(pts, st));
    }
}

fn group(inner: B, s: f32, c: &Ctx, square: bool) -> B {
    let pad = s * 0.26 * c.spacing;
    let sw = (s * 0.05 * c.rule).max(1.0);
    let h = inner.h + inner.d + s * 0.20;
    let top = -inner.h - s * 0.10;
    let w = inner.w + pad * 2.0;
    let mut items = vec![
        Item::Delim(Rect::from_min_size(Pos2::new(0.0, top), Vec2::new(pad*0.72, h)), square),
        Item::Delim(Rect::from_min_size(Pos2::new(w - pad*0.72, top), Vec2::new(pad*0.72, h)), !square),
    ];
    items.extend(inner.moved(pad, 0.0).items);
    finish(items)
}

fn fraction(num: B, den: B, s: f32, c: &Ctx) -> B {
    let axis = s * 0.25;
    let rule = (s * 0.05 * c.rule).max(0.9);
    let gap = s * 0.16 * c.spacing;
    let pad = s * 0.18 * c.spacing;
    let w = num.w.max(den.w) + pad * 2.0;
    let bar_y = -axis - rule * 0.5;
    let (nw, nd, dw, dh) = (num.w, num.d, den.w, den.h);
    let mut items = Vec::new();
    items.extend(num.moved((w - nw)*0.5, bar_y - gap - nd).items);
    items.push(Item::Rule(Rect::from_min_size(Pos2::new(0.0, bar_y), Vec2::new(w, rule))));
    items.extend(den.moved((w - dw)*0.5, bar_y + rule + gap + dh).items);
    finish(items)
}

fn layout(ui: &Ui, tpl: &Tpl, s: f32, hint: &str, sl: &Slots<'_>, c: &Ctx, path: &[usize]) -> B {
    let axis = s * 0.25;
    let sp = |k: f32| s * k * c.spacing;
    let kid = |i: usize| { let mut p = path.to_vec(); p.push(i); p };
    match tpl {
        Tpl::Hole(n) => slot_box(ui, *n, s, hint, sl, c),
        Tpl::Num(v) => leaf_box(ui, path.to_vec(), v, s, false),
        Tpl::Unit(v) => leaf_box(ui, path.to_vec(), v.trim(), s * 0.94, false),
        Tpl::Sym(v) => {
            let (base, sub) = match v.split_once('_') {
                Some((a, b)) if !a.is_empty() && !b.is_empty() => (a, Some(b)),
                _ => (v.as_str(), None),
            };
            let shown = glyph(base).unwrap_or(base);
            let up = is_function(base);
            let b = leaf_box(ui, path.to_vec(), shown, s, !up);
            match sub {
                Some(x) => {
                    let sb = layout(ui, &Tpl::Sym(x.to_string()), s * c.script, "", sl, c, &kid(0));
                    let bw = b.w;
                    let off = (s * 0.20).max(b.d + s * 0.06);
                    let mut items = b.items;
                    items.extend(sb.moved(bw + sp(0.02), off).items);
                    finish(items)
                }
                None => b,
            }
        }
        Tpl::Frac(a, b) => fraction(layout(ui, a, s*0.92, "num", sl, c, &kid(0)), layout(ui, b, s*0.92, "den", sl, c, &kid(1)), s, c),
        Tpl::Pow(a, b) => {
            let base = layout(ui, a, s, "base", sl, c, &kid(0));
            let sup = layout(ui, b, s * c.script, "n", sl, c, &kid(1));
            let bw = base.w;
            let raise = (base.h - s*0.30).max(s*0.42);
            let mut items = base.items;
            items.extend(sup.moved(bw + sp(0.03), -raise).items);
            finish(items)
        }
        Tpl::Sub(a, b) => {
            let base = layout(ui, a, s, "x", sl, c, &kid(0));
            let sub = layout(ui, b, s * c.script, "i", sl, c, &kid(1));
            let bw = base.w;
            let drop = (s*0.22).max(base.d + s*0.08);
            let mut items = base.items;
            items.extend(sub.moved(bw + sp(0.02), drop).items);
            finish(items)
        }
        Tpl::Sqrt(a) => {
            let body = layout(ui, a, s, "value", sl, c, &kid(0));
            let lead = s * 0.58;
            let clr = s * 0.16 * c.spacing;
            let rule = (s * 0.05 * c.rule).max(1.0);
            let top = -(body.h + clr);
            let bot = body.d + s * 0.06;
            let bw = body.w;
            let right = lead + s*0.10 + bw + s*0.12;
            let pts = vec![
                Pos2::new(0.0, top + (bot-top)*0.56),
                Pos2::new(lead*0.20, top + (bot-top)*0.68),
                Pos2::new(lead*0.46, bot),
                Pos2::new(lead, top),
                Pos2::new(right, top),
            ];
            let mut items = vec![Item::Path(pts, rule)];
            items.extend(body.moved(lead + s*0.10, 0.0).items);
            finish(items)
        }
        Tpl::NthRoot(deg, a) => {
            let inner = layout(ui, &Tpl::Sqrt(a.clone()), s, "value", sl, c, &kid(1));
            let d = layout(ui, deg, s * c.script * 0.9, "n", sl, c, &kid(0));
            let off = (d.w * 0.70).max(s * 0.26);
            let mut items = inner.moved(off, 0.0).items;
            items.extend(d.moved(0.0, -s*0.30).items);
            finish(items)
        }
        Tpl::Abs(a) => {
            let body = layout(ui, a, s, "value", sl, c, &kid(0));
            let rule = (s*0.05*c.rule).max(1.0);
            let pad = s*0.14*c.spacing;
            let h = body.h + body.d + s*0.16;
            let top = -body.h - s*0.08;
            let bw = body.w;
            let mut items = Vec::new();
            items.push(Item::Rule(Rect::from_min_size(Pos2::new(pad, top), Vec2::new(rule, h))));
            items.extend(body.moved(pad + rule + pad*0.4, 0.0).items);
            items.push(Item::Rule(Rect::from_min_size(Pos2::new(pad + rule + pad*0.4 + bw + pad*0.4, top), Vec2::new(rule, h))));
            finish(items)
        }
        Tpl::Paren(a) => group(layout(ui, a, s, "value", sl, c, &kid(0)), s, c, false),
        Tpl::Integral(f, lo, hi, var) => {
            let hgt = s * 1.95; let wid = s * 0.52;
            let top = -(axis + hgt*0.5); let bot = top + hgt;
            let sw = (s*0.06*c.rule).max(1.2);
            let mut items = vec![Item::Path(integral_path(wid, top, bot), sw)];
            let sup = layout(ui, hi, s*c.script, "b", sl, c, &kid(2));
            let sub = layout(ui, lo, s*c.script, "a", sl, c, &kid(1));
            let sup_x = wid + sp(0.06); let sup_y = top + hgt*0.30;
            let sub_x = wid*0.66; let sub_y = bot - hgt*0.07;
            let (sup_w, sub_w) = (sup.w, sub.w);
            items.extend(sup.moved(sup_x, sup_y).items);
            items.extend(sub.moved(sub_x, sub_y).items);
            let body_x = (sup_x + sup_w).max(sub_x + sub_w) + sp(0.22);
            let body = layout(ui, f, s, "f(x)", sl, c, &kid(0));
            let bw = body.w;
            items.extend(body.moved(body_x, 0.0).items);
            let dx = body_x + bw + sp(0.18);
            let dg = glyph_box(ui, "d", s*0.94, false);
            let dw = dg.w;
            items.extend(dg.moved(dx, 0.0).items);
            let v = layout(ui, var, s*0.94, "x", sl, c, &kid(3));
            items.extend(v.moved(dx + dw + sp(0.03), 0.0).items);
            finish(items)
        }
        Tpl::Sum(lo, hi, body) | Tpl::Prod(lo, hi, body) => {
            let is_sum = matches!(tpl, Tpl::Sum(..));
            let g = if is_sum { "\u{2211}" } else { "\u{220F}" };
            let sign = glyph_box(ui, g, s*1.55, false);
            let (sw, sh, sd) = (sign.w, sign.h, sign.d);
            let sign = sign.moved(0.0, axis - (sh - sd)*0.5);
            let st = -sign.h; let sb = sign.d;
            let up = layout(ui, hi, s*c.script, "n", sl, c, &kid(1));
            let lo_b = layout(ui, lo, s*c.script, "k=1", sl, c, &kid(0));
            let col = sw.max(up.w).max(lo_b.w);
            let gap = sp(0.13);
            let (uw, ud, lw, lh) = (up.w, up.d, lo_b.w, lo_b.h);
            let mut items = sign.moved((col - sw)*0.5, 0.0).items;
            items.extend(up.moved((col - uw)*0.5, st - gap - ud).items);
            items.extend(lo_b.moved((col - lw)*0.5, sb + gap + lh).items);
            let b = layout(ui, body, s, "f(k)", sl, c, &kid(2));
            items.extend(b.moved(col + sp(0.22), 0.0).items);
            finish(items)
        }
        Tpl::Limit(v, to, body) => {
            let lim = glyph_box(ui, "lim", s, false);
            let under = seq(vec![
                layout(ui, v, s*c.script, "x", sl, c, &kid(0)),
                glyph_box(ui, "\u{2192}", s*c.script, false),
                layout(ui, to, s*c.script, "0", sl, c, &kid(1)),
            ], sp(0.07));
            let col = lim.w.max(under.w);
            let (lw, ld, uw, uh) = (lim.w, lim.d, under.w, under.h);
            let mut items = lim.moved((col - lw)*0.5, 0.0).items;
            items.extend(under.moved((col - uw)*0.5, ld + sp(0.09) + uh).items);
            let b = layout(ui, body, s, "f(x)", sl, c, &kid(2));
            items.extend(b.moved(col + sp(0.20), 0.0).items);
            finish(items)
        }
        Tpl::Deriv(f, v, n) => {
            let dn = if *n <= 1 { "d".to_string() } else { format!("d{}", n) };
            let num = seq(vec![glyph_box(ui, &dn, s*0.88, false), layout(ui, f, s*0.88, "f(x)", sl, c, &kid(0))], sp(0.05));
            let den = seq(vec![glyph_box(ui, "d", s*0.88, false), layout(ui, v, s*0.88, "x", sl, c, &kid(1))], sp(0.05));
            fraction(num, den, s, c)
        }
        Tpl::Matrix(rows) => {
            let mut idx = 0;
            let mut cells: Vec<Vec<(B, Vec<usize>)>> = Vec::new();
            for r in rows {
                let mut row = Vec::new();
                for x in r { row.push((layout(ui, x, s*0.92, "0", sl, c, &kid(idx)), kid(idx))); idx += 1; }
                cells.push(row);
            }
            let cw = cells.iter().flatten().map(|(b, _)| b.w).fold(0.0_f32, f32::max) + sp(0.40);
            let ch = cells.iter().flatten().map(|(b, _)| b.h + b.d).fold(0.0_f32, f32::max) + sp(0.30);
            let nr = rows.len().max(1);
            let nc = rows.iter().map(|r| r.len()).max().unwrap_or(1);
            let pad = s * 0.30;
            let w = pad*2.0 + nc as f32 * cw;
            let h = pad*0.8 + nr as f32 * ch;
            let sw = (s*0.05*c.rule).max(1.0);
            let mut items = vec![
                Item::Delim(Rect::from_min_size(Pos2::new(0.0, -h*0.55), Vec2::new(pad*0.7, h)), true),
                Item::Delim(Rect::from_min_size(Pos2::new(w - pad*0.7, -h*0.55), Vec2::new(pad*0.7, h)), false),
            ];
            for (ri, row) in cells.iter().enumerate() {
                for (ci, (b, _)) in row.iter().enumerate() {
                    let (bw, bh, bd) = (b.w, b.h, b.d);
                    let cx = pad + ci as f32 * cw + (cw - bw)*0.5;
                    let cy = -h*0.55 + pad*0.4 + ri as f32 * ch + (ch - (bh + bd))*0.5 + bh;
                    items.extend(b.clone().moved(cx.max(pad), cy).items);
                }
            }
            finish(items)
        }
        Tpl::Bin(op, a, b) => {
            let g = match op.as_str() { "*" => "\u{22C5}", "/" => "\u{00F7}", "-" => "\u{2212}", x => x };
            seq(vec![
                layout(ui, a, s, hint, sl, c, &kid(0)),
                glyph_box(ui, g, s, false),
                layout(ui, b, s, "value", sl, c, &kid(1)),
            ], sp(0.22))
        }
        Tpl::Assign(name, v) => seq(vec![
            glyph_box(ui, &glyph(name).unwrap_or(name).to_string(), s, true),
            glyph_box(ui, ":=", s, false),
            layout(ui, v, s, "value", sl, c, &kid(0)),
        ], sp(0.26)),
        Tpl::Eval(a) => seq(vec![layout(ui, a, s, "value", sl, c, &kid(0)), glyph_box(ui, "=", s, false)], sp(0.26)),
        Tpl::Convert(a, u) => seq(vec![
            layout(ui, a, s, "value", sl, c, &kid(0)),
            glyph_box(ui, "\u{2192}", s, false),
            layout(ui, u, s*0.94, "unit", sl, c, &kid(1)),
        ], sp(0.22)),
    }
}

// ---------------------------------------------------------------- painting
pub struct Painted {
    pub size: Vec2,
    pub slots: Vec<(Slot, Rect)>,
    pub hints: Vec<(Slot, String)>,
    pub leaves: Vec<(Vec<usize>, Rect, String)>,
}

pub fn draw(ui: &mut Ui, state: &UiState, id: Uuid, t: &Theme, size: f32, interactive: bool) -> Option<Painted> {
    let tpl = state.templates.get(&id)?.clone();
    let focused = state.focused_slot.filter(|(b, _)| *b == id).map(|(_, s)| s);
    let sl = Slots { focused, buffer: &state.slot_buffer };
    let c = Ctx::from(state);
    let b = layout(ui, &tpl, size, "value", &sl, &c, &[]);
    let pad = 4.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(b.w + pad*2.0, b.h + b.d + pad*2.0), Sense::hover());
    let origin = rect.min + Vec2::new(pad, pad + b.h);
    let painter = ui.painter_at(rect);
    let mut slots = Vec::new();
    let mut hints = Vec::new();
    let mut leaves = Vec::new();
    let sw = (size*0.05*c.rule).max(1.0);
    let focused_leaf = state.focused_leaf.as_ref().filter(|(b, _)| *b == id).map(|(_, p)| p.as_slice());
    
    for it in &b.items {
        match it {
            Item::Glyph(s, r, f) => { painter.text(origin + r.min.to_vec2(), egui::Align2::LEFT_TOP, s, f.clone(), t.ink); }
            Item::Rule(r) => { painter.rect_filled(r.translate(origin.to_vec2()), 0.4, t.ink); }
            Item::Path(pts, w) => {
                let mv: Vec<Pos2> = pts.iter().map(|p| *p + origin.to_vec2()).collect();
                painter.add(egui::Shape::line(mv, Stroke::new(*w, t.ink)));
            }
            Item::Delim(r, left) => { paint_delim(&painter, r.translate(origin.to_vec2()), *left, sw, t.ink_soft); }
            Item::Slot(slot, r, label, ps) => {
                let r = r.translate(origin.to_vec2());
                let active = interactive && focused == Some(*slot);
                let anim = ui.ctx().animate_bool_with_time(egui::Id::new(("slot", id, *slot)), active, 0.12);
                if anim > 0.001 && anim < 0.999 { ui.ctx().request_repaint(); }
                painter.rect_filled(r, 3.0, t.accent_soft.linear_multiply(0.26 + 0.55*anim));
                painter.rect_stroke(r, 3.0, Stroke::new(if active { 1.6_f32 } else { 1.0_f32 }, if active { t.accent } else { t.accent_line }));
                if !active { painter.text(r.center(), egui::Align2::CENTER_CENTER, label, theme::math(*ps), t.ink_faint); }
                slots.push((*slot, r));
                hints.push((*slot, label.clone()));
            }
            Item::Leaf(path, r, text, ls) => {
                let screen = r.translate(origin.to_vec2());
                let active = interactive && focused_leaf.as_deref() == Some(path.as_slice());
                if active {
                    painter.rect_stroke(screen.expand(2.0), 3.0, Stroke::new(1.4_f32, t.accent));
                } else {
                    painter.text(screen.min, egui::Align2::LEFT_TOP, text, theme::math(*ls), t.ink);
                }
                leaves.push((path.clone(), screen, text.clone()));
            }
        }
    }
    Some(Painted { size: rect.size(), slots, hints, leaves })
}

pub fn paint(ui: &mut Ui, state: &UiState, id: Uuid, t: &Theme, size: f32) -> Option<Painted> {
    draw(ui, state, id, t, size, true)
}

pub fn slot_editor(ui: &mut Ui, ctx: &egui::Context, state: &mut UiState, session: &mut aaiyyan_core::Session,
    id: Uuid, t: &Theme, size: f32, painted: &Painted) {
    let Some((bid, slot)) = state.focused_slot else { return };
    if bid != id { return; }
    let Some((_, r)) = painted.slots.iter().find(|(s, _)| *s == slot).copied() else { return };
    let hint = painted.hints.iter().find(|(s, _)| *s == slot).map(|(_, h)| h.clone()).unwrap_or_else(|| "value".into());
    let ps = size * 0.84;
    let font = theme::math(ps);
    let inner = (r.width() - 10.0).max(22.0);
    let rect = Rect::from_min_size(r.min + Vec2::new(4.0, 3.0), Vec2::new(inner + 2.0, (r.height() - 6.0).max(ps*1.2)));
    let mut buf = state.slot_buffer.clone();
    let resp = ui.allocate_ui_at_rect(rect, |ui| {
        ui.add(egui::TextEdit::singleline(&mut buf).id(egui::Id::new(("slotedit", id, slot)))
            .font(font.clone()).desired_width(inner).margin(1.0).frame(false)
            .hint_text(egui::RichText::new(hint).font(font).color(t.ink_faint)))
    }).inner;
    if state.slot_focus == Some((id, slot)) { resp.request_focus(); state.slot_focus = None; }
    if resp.changed() { state.slot_buffer = buf.clone(); }
    let advance = ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Tab));
    if advance || resp.lost_focus() {
        commit_slot(state, session, id, slot, &buf);
        let n = next_slot(state, id);
        state.focused_slot = n.map(|s| (id, s));
        state.slot_focus = state.focused_slot;
        state.slot_buffer.clear();
    }
}

fn commit_slot(state: &mut UiState, session: &mut aaiyyan_core::Session, id: Uuid, slot: Slot, buf: &str) {
    let v = buf.trim();
    if v.is_empty() { return; }
    if let Some(mut tpl) = state.templates.get(&id).cloned() {
        let mut n = tpl.max_slot() + 1;
        let leaf = if v.ends_with('=') {
            Tpl::Eval(Box::new(parse_expr(v[..v.len() - 1].trim(), &mut n).unwrap_or_else(|| Tpl::Hole(n))))
        } else {
            parse_expr(v, &mut n).unwrap_or_else(|| parse_leaf(v))
        };
        if tpl.fill(slot, leaf) { state.templates.insert(id, tpl); sync(session, state, id); }
    }
}

/// Inline editor on a symbol/number leaf. Typing `name := value` promotes the
/// whole object to a definition; anything else replaces the leaf in place.
pub fn leaf_editor(ui: &mut Ui, ctx: &egui::Context, state: &mut UiState, session: &mut aaiyyan_core::Session,
    id: Uuid, t: &Theme, size: f32, painted: &Painted) {
    let Some((bid, path)) = state.focused_leaf.clone() else { return };
    if bid != id { return; }
    let Some((_, r, text)) = painted.leaves.iter().find(|(p, _, _)| *p == path).cloned() else { return };
    if state.leaf_focus == Some((id, path.clone())) {
        state.leaf_buffer = text.clone();
        state.leaf_focus = None;
    }
    let font = theme::math(size * 0.9);
    let bw = ui.fonts(|g| g.layout_no_wrap(state.leaf_buffer.clone(), font.clone(), t.ink).size().x);
    let w = (bw + 20.0).max(r.width() + 12.0).max(40.0);
    let rect = Rect::from_min_size(r.min - Vec2::new(3.0, 3.0), Vec2::new(w, r.height() + 6.0));
    let mut buf = state.leaf_buffer.clone();
    let resp = ui.allocate_ui_at_rect(rect, |ui| {
        ui.add(egui::TextEdit::singleline(&mut buf).id(egui::Id::new(("leafedit", id, path.clone())))
            .font(font.clone()).desired_width(w - 8.0).margin(2.0))
    }).inner;
    if resp.changed() { state.leaf_buffer = buf.clone(); }
    let advance = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    if advance || resp.lost_focus() {
        commit_leaf(state, session, id, &path, &buf);
        state.focused_leaf = None;
        state.leaf_buffer.clear();
        let n = next_slot(state, id);
        state.focused_slot = n.map(|s| (id, s));
        state.slot_focus = state.focused_slot;
    }
}

fn parse_leaf(v: &str) -> Tpl {
    let numeric = v.chars().next().map(|c| c.is_ascii_digit() || c == '-' || c == '.').unwrap_or(false);
    if numeric { Tpl::Num(v.into()) } else { Tpl::Sym(v.into()) }
}

pub fn commit_leaf(state: &mut UiState, session: &mut aaiyyan_core::Session, id: Uuid, path: &[usize], buf: &str) {
    let v = buf.trim();
    if v.is_empty() { return; }
    let Some(old) = state.templates.get(&id).cloned() else { return };
    let mut n = old.max_slot() + 1;
    let new = if let Some(eq) = v.find(":=") {
        let name = v[..eq].trim().to_string();
        let val = v[eq + 2..].trim();
        Tpl::Assign(name, Box::new(parse_expr(val, &mut n).unwrap_or_else(|| Tpl::Hole(n))))
    } else if v.ends_with('=') {
        let body = v[..v.len() - 1].trim();
        Tpl::Eval(Box::new(parse_expr(body, &mut n).unwrap_or_else(|| Tpl::Hole(n))))
    } else {
        let parsed = parse_expr(v, &mut n).unwrap_or_else(|| parse_leaf(v));
        let mut t = old.clone();
        t.set_path(path, parsed);
        t
    };
    if new != old {
        state.templates.insert(id, new);
        sync(session, state, id);
    }
}

// ---------------------------------------------------------------- round-trip
pub fn from_source(src: &str) -> Option<Tpl> {
    let s = src.trim();
    if s.is_empty() { return None; }
    let mut next: Slot = 1;
    parse_top(s, &mut next)
}

fn parse_top(s: &str, n: &mut Slot) -> Option<Tpl> {
    if let Some(eq) = find_top(s, ":=") {
        return Some(Tpl::Assign(s[..eq].trim().into(), Box::new(parse_expr(s[eq + 2..].trim(), n)?)));
    }
    if s.ends_with('=') && !s.ends_with("==") && !s.ends_with(":=") {
        return Some(Tpl::Eval(Box::new(parse_expr(s[..s.len() - 1].trim(), n)?)));
    }
    parse_expr(s, n)
}

fn find_top(s: &str, needle: &str) -> Option<usize> {
    let mut d = 0i32;
    let b = s.as_bytes();
    for i in 0..s.len() {
        match b[i] { b'(' | b'[' => d += 1, b')' | b']' => d -= 1, _ => {} }
        if d == 0 && s[i..].starts_with(needle) { return Some(i); }
    }
    None
}

fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut d = 0i32;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' | '[' => { d += 1; cur.push(ch); }
            ')' | ']' => { d -= 1; cur.push(ch); }
            ',' if d == 0 => { out.push(cur.trim().into()); cur.clear(); }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() { out.push(cur.trim().into()); }
    out
}

fn balanced(s: &str) -> bool {
    let mut d = 0i32;
    for ch in s.chars() {
        match ch { '(' | '[' => d += 1, ')' | ']' => d -= 1, _ => {} }
        if d < 0 { return false; }
    }
    d == 0
}

fn find_top_op(s: &str, op: &str) -> Option<usize> {
    let mut d = 0i32;
    let b = s.as_bytes();
    let mut last = None;
    for i in 0..s.len() {
        match b[i] { b'(' | b'[' => d += 1, b')' | b']' => d -= 1, _ => {} }
        if d == 0 && i > 0 && s[i..].starts_with(op) {
            let p = b[i - 1];
            if p != b'(' && p != b'[' && p != b'*' && p != b'/' && p != b'^' && p != b'e' && p != b'E' { last = Some(i); }
        }
    }
    last
}

fn parse_expr(s: &str, n: &mut Slot) -> Option<Tpl> {
    let s = s.trim();
    if s.is_empty() { return None; }
    if s.starts_with('(') && s.ends_with(')') && balanced(&s[1..s.len() - 1]) {
        return parse_expr(&s[1..s.len() - 1], n);
    }
    for op in ["+", "-"] {
        if let Some(i) = find_top_op(s, op) {
            return Some(Tpl::Bin(op.into(), Box::new(parse_expr(&s[..i], n)?), Box::new(parse_expr(&s[i + 1..], n)?)));
        }
    }
    for op in ["*", "/"] {
        if let Some(i) = find_top_op(s, op) {
            return Some(Tpl::Bin(op.into(), Box::new(parse_expr(&s[..i], n)?), Box::new(parse_expr(&s[i + 1..], n)?)));
        }
    }
    if let Some(i) = find_top_op(s, "^") {
        return Some(Tpl::Pow(Box::new(parse_expr(&s[..i], n)?), Box::new(parse_expr(&s[i + 1..], n)?)));
    }
    if s.starts_with("[[") && s.ends_with("]]") {
        let rows: Vec<Vec<Tpl>> = split_args(&s[1..s.len() - 1]).iter().filter_map(|r| {
            let r = r.trim();
            if !(r.starts_with('[') && r.ends_with(']')) { return None; }
            Some(split_args(&r[1..r.len() - 1]).iter().filter_map(|x| parse_expr(x, n)).collect())
        }).collect();
        if !rows.is_empty() { return Some(Tpl::Matrix(rows)); }
    }
    if let Some(open) = s.find('(') {
        if s.ends_with(')') {
            let name = s[..open].trim();
            let a = split_args(&s[open + 1..s.len() - 1]);
            let p = |i: usize, n: &mut Slot| a.get(i).and_then(|x| parse_expr(x, n));
            match name {
                "integrate" if a.len() >= 3 => return Some(Tpl::Integral(Box::new(p(0, n)?), Box::new(p(1, n)?), Box::new(p(2, n)?), Box::new(Tpl::Sym("x".into())))),
                "sumrange" if a.len() == 3 => return Some(Tpl::Sum(Box::new(p(0, n)?), Box::new(p(1, n)?), Box::new(p(2, n)?))),
                "prodrange" if a.len() == 3 => return Some(Tpl::Prod(Box::new(p(0, n)?), Box::new(p(1, n)?), Box::new(p(2, n)?))),
                "limit" if a.len() == 3 => return Some(Tpl::Limit(Box::new(p(1, n)?), Box::new(p(2, n)?), Box::new(p(0, n)?))),
                "nderiv" if a.len() >= 2 => {
                    let k = a.get(2).and_then(|v| v.parse::<u32>().ok()).unwrap_or(1);
                    return Some(Tpl::Deriv(Box::new(p(0, n)?), Box::new(p(1, n)?), k));
                }
                "sqrt" if a.len() == 1 => return Some(Tpl::Sqrt(Box::new(p(0, n)?))),
                "nroot" if a.len() == 2 => return Some(Tpl::NthRoot(Box::new(p(1, n)?), Box::new(p(0, n)?))),
                "abs" if a.len() == 1 => return Some(Tpl::Abs(Box::new(p(0, n)?))),
                "convert" if a.len() == 2 => return Some(Tpl::Convert(Box::new(p(0, n)?), Box::new(p(1, n)?))),
                _ => {}
            }
        }
    }
    if let Some(rest) = s.strip_prefix("hole") {
        if let Ok(k) = rest.parse::<Slot>() { *n = (*n).max(k + 1); return Some(Tpl::Hole(k)); }
    }
    if s.chars().any(|c| c.is_ascii_digit()) && s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e') {
        return Some(Tpl::Num(s.into()));
    }
    Some(Tpl::Sym(s.into()))
}

// ---------------------------------------------------------------- session glue
pub fn ensure(state: &mut UiState, id: Uuid, source: &str) {
    if state.templates.contains_key(&id) { return; }
    if let Some(t) = from_source(source) { state.templates.insert(id, t); }
}

pub fn sync(session: &mut aaiyyan_core::Session, state: &UiState, id: Uuid) {
    if let Some(t) = state.templates.get(&id) { session.update_source(id, t.to_source()); }
}

pub fn next_slot(state: &UiState, id: Uuid) -> Option<Slot> {
    let t = state.templates.get(&id)?;
    let mut v = Vec::new();
    t.slots(&mut v);
    v.sort_unstable();
    let cur = state.focused_slot.filter(|(b, _)| *b == id).map(|(_, s)| s);
    match cur {
        Some(c) => v.iter().find(|s| **s > c).copied().or_else(|| v.first().copied()),
        None => v.first().copied(),
    }
}

pub fn insert_make(state: &mut UiState, session: &mut aaiyyan_core::Session, kind: Make, at: (f32, f32)) {
    let target = state.focused_slot.filter(|(id, _)| session.worksheet().block(*id).is_some());
    if let Some((id, slot)) = target {
        if let Some(t) = state.templates.get(&id).cloned() {
            let first = t.max_slot() + 1;
            let sub = build(kind, first);
            let mut nt = t;
            if nt.fill(slot, sub) {
                state.templates.insert(id, nt);
                sync(session, state, id);
                let n = next_slot(state, id);
                state.focused_slot = n.map(|s| (id, s));
                state.slot_focus = state.focused_slot;
                state.slot_buffer.clear();
                return;
            }
        }
    }
    state.focused_slot = None;
    state.slot_focus = None;
    let tpl = build(kind, 1);
    let id = session.insert_block_at(aaiyyan_worksheet::BlockKind::Equation { source: tpl.to_source() }, at.0, at.1);
    state.templates.insert(id, tpl);
    state.selected = Some(id);
    state.editing = None;
    state.cursor = Some(Vec2::new(at.0, at.1 + 96.0));
    let n = next_slot(state, id);
    state.focused_slot = n.map(|s| (id, s));
    state.slot_focus = state.focused_slot;
    state.slot_buffer.clear();
}

pub fn insert_leaf(state: &mut UiState, session: &mut aaiyyan_core::Session, text: &str, unit: bool, at: (f32, f32)) {
    let raw = text.trim();
    let numeric = raw.chars().next().map(|c| c.is_ascii_digit() || c == '-' || c == '.').unwrap_or(false);
    let leaf = if unit { Tpl::Unit(raw.into()) } else if numeric { Tpl::Num(raw.into()) } else { Tpl::Sym(raw.into()) };
    if let Some((id, path)) = state.focused_leaf.clone() {
        if session.worksheet().block(id).is_some() {
            if let Some(mut t) = state.templates.get(&id).cloned() {
                if t.set_path(&path, leaf.clone()) {
                    state.templates.insert(id, t);
                    sync(session, state, id);
                    state.focused_leaf = None;
                    return;
                }
            }
        }
    }
    let target = state.focused_slot.filter(|(id, _)| session.worksheet().block(*id).is_some());
    if let Some((id, slot)) = target {
        if let Some(t) = state.templates.get(&id).cloned() {
            let mut nt = t;
            if nt.fill(slot, leaf.clone()) {
                state.templates.insert(id, nt);
                sync(session, state, id);
                let n = next_slot(state, id);
                state.focused_slot = n.map(|s| (id, s));
                state.slot_focus = state.focused_slot;
                state.slot_buffer.clear();
                return;
            }
        }
    }
    let id = session.insert_block_at(aaiyyan_worksheet::BlockKind::Equation { source: leaf.to_source() }, at.0, at.1);
    state.templates.insert(id, leaf);
    state.selected = Some(id);
    state.editing = None;
    state.cursor = Some(Vec2::new(at.0, at.1 + 60.0));
    state.focused_slot = None;
    state.focused_leaf = None;
}

pub fn insert_definition(state: &mut UiState, session: &mut aaiyyan_core::Session, name: &str, at: (f32, f32)) {
    let tpl = Tpl::Assign(name.to_string(), Box::new(Tpl::Hole(1)));
    let id = session.insert_block_at(aaiyyan_worksheet::BlockKind::Equation { source: tpl.to_source() }, at.0, at.1);
    state.templates.insert(id, tpl);
    state.def_names.insert(id, name.to_string());
    state.selected = Some(id);
    state.editing = None;
    state.focused_leaf = None;
    state.cursor = Some(Vec2::new(at.0, at.1 + 72.0));
    state.focused_slot = Some((id, 1));
    state.slot_focus = Some((id, 1));
    state.slot_buffer.clear();
}

pub fn prune(state: &mut UiState, session: &aaiyyan_core::Session) {
    let live: std::collections::HashSet<Uuid> = session.worksheet().blocks.iter().map(|b| b.id).collect();
    state.templates.retain(|id, _| live.contains(id));
    state.def_names.retain(|id, _| live.contains(id));
    state.slot_hits.retain(|(id, _, _)| live.contains(id));
    state.leaf_hits.retain(|(id, _, _)| live.contains(id));
    if let Some((id, _)) = state.focused_slot {
        if !live.contains(&id) { state.focused_slot = None; state.slot_focus = None; state.slot_buffer.clear(); }
    }
    if let Some((id, _)) = state.focused_leaf.clone() {
        if !live.contains(&id) { state.focused_leaf = None; state.leaf_focus = None; state.leaf_buffer.clear(); }
    }
    if let Some(id) = state.selected { if !live.contains(&id) { state.selected = None; } }
    if let Some(id) = state.editing { if !live.contains(&id) { state.editing = None; } }
    if let Some(id) = state.focus_request { if !live.contains(&id) { state.focus_request = None; } }
}

/// Inline control strip: a persistent definition rail plus structure and style.
pub fn toolbar(ui: &mut Ui, state: &mut UiState, session: &mut aaiyyan_core::Session, actions: &mut Vec<UiAction>, t: &Theme) {
    let Some(id) = state.selected else { return };
    if !state.templates.contains_key(&id) { return; }
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 3.0;

        // Definition rail — always visible, names the object inline.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            ui.label(egui::RichText::new("define").size(9.5).strong().color(t.accent));
            let current = match state.templates.get(&id) {
                Some(Tpl::Assign(n, _)) => n.clone(),
                _ => state.def_names.get(&id).cloned().unwrap_or_else(|| "f".into()),
            };
            let mut name = current.clone();
            let resp = ui.add(egui::TextEdit::singleline(&mut name)
                .id(egui::Id::new(("defname", id)))
                .desired_width(64.0).font(theme::math_italic(14.0)).margin(3.0)
                .hint_text("name"));
            if resp.changed() {
                state.def_names.insert(id, name.clone());
                if let Some(Tpl::Assign(_, v)) = state.templates.get(&id).cloned() {
                    state.templates.insert(id, Tpl::Assign(name.clone(), v));
                    sync(session, state, id);
                }
            }
            ui.label(egui::RichText::new(":=").font(theme::math(14.0)).color(t.ink_soft));
            let is_assign = matches!(state.templates.get(&id), Some(Tpl::Assign(..)));
            if ui.small_button(if is_assign { "unwrap" } else { "wrap :=" }).clicked() {
                if let Some(tp) = state.templates.get(&id).cloned() {
                    let next = if let Tpl::Assign(_, v) = tp { *v } else { Tpl::Assign(name.clone(), Box::new(tp)) };
                    state.templates.insert(id, next);
                    sync(session, state, id);
                }
            }
            ui.add_space(6.0);
            let kind = state.templates.get(&id).map(|x| x.kind_name()).unwrap_or("");
            ui.label(egui::RichText::new(kind.to_uppercase()).size(9.5).strong().color(t.ink_faint));
            let mut v = Vec::new();
            if let Some(tp) = state.templates.get(&id) { tp.slots(&mut v); }
            let pos = state.focused_slot.filter(|(b, _)| *b == id).and_then(|(_, s)| v.iter().position(|x| *x == s));
            ui.label(egui::RichText::new(match pos { Some(p) => format!("slot {}/{}", p + 1, v.len()), None => "complete".into() }).size(10.0).color(t.ink_faint));
            if ui.small_button("\u{2190}").clicked() {
                let cur = state.focused_slot.filter(|(b, _)| *b == id).map(|(_, s)| s);
                let prev = cur.and_then(|c| v.iter().rev().find(|x| **x < c).copied()).or_else(|| v.last().copied());
                state.focused_slot = prev.map(|s| (id, s)); state.slot_focus = state.focused_slot; state.slot_buffer.clear();
            }
            if ui.small_button("\u{2192}").on_hover_text("Next slot (Tab)").clicked() {
                let n = next_slot(state, id); state.focused_slot = n.map(|s| (id, s)); state.slot_focus = state.focused_slot; state.slot_buffer.clear();
            }
            if ui.small_button("Clear").clicked() {
                if let Some((bid, slot)) = state.focused_slot {
                    if let Some(mut tp) = state.templates.get(&bid).cloned() {
                        tp.fill(slot, Tpl::Hole(slot)); state.templates.insert(bid, tp); sync(session, state, bid);
                        state.slot_buffer.clear(); state.slot_focus = Some((bid, slot));
                    }
                }
            }
            if ui.small_button("\u{2715}").on_hover_text("Delete object").clicked() { actions.push(UiAction::Delete(id)); }
        });

        // Structure rail
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            for (label, kind, tip) in [("a/b", Make::Frac, "Fraction"), ("x\u{207F}", Make::Pow, "Power"), ("\u{221A}", Make::Sqrt, "Root"), ("( )", Make::Paren, "Group"), ("\u{222B}", Make::Integral, "Integral"), ("\u{2211}", Make::Sum, "Sum")] {
                if ui.small_button(label).on_hover_text(tip).clicked() {
                    if let Some((bid, slot)) = state.focused_slot { actions.push(UiAction::TemplateMake(kind, bid, slot)); }
                }
            }
            ui.add_space(4.0);
            if ui.small_button("=").on_hover_text("Evaluate").clicked() {
                if let Some(tp) = state.templates.get(&id).cloned() { if !matches!(tp, Tpl::Eval(_)) { state.templates.insert(id, Tpl::Eval(Box::new(tp))); sync(session, state, id); } }
            }
            ui.add_space(4.0);
            ui.label(egui::RichText::new("size").size(9.5).color(t.ink_faint));
            ui.add(egui::Slider::new(&mut state.math_size, 12.0..=40.0).show_value(false).text(""));
            if ui.small_button(if state.style_open { "style \u{25B4}" } else { "style \u{25BE}" }).clicked() { state.style_open = !state.style_open; }
        });

        if state.style_open {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (label, val, range, tip) in [
                    ("scripts", &mut state.script_scale, 0.45..=0.95, "Limit / exponent scale"),
                    ("spacing", &mut state.math_spacing, 0.5..=1.8, "Atom spacing"),
                    ("rules", &mut state.rule_scale, 0.4..=2.4, "Stroke weight"),
                    ("slots", &mut state.slot_pad, 0.3..=2.2, "Placeholder padding"),
                ] {
                    ui.label(egui::RichText::new(label).size(9.5).color(t.ink_faint));
                    ui.add(egui::Slider::new(val, range).show_value(false).text("")).on_hover_text(tip);
                }
                if ui.small_button("reset").clicked() { state.script_scale = 0.70; state.math_spacing = 1.0; state.rule_scale = 1.0; state.slot_pad = 1.0; state.math_size = 20.0; }
                if ui.small_button("example").clicked() {
                    let tpl = Tpl::Integral(Box::new(Tpl::Pow(Box::new(Tpl::Sym("x".into())), Box::new(Tpl::Num("2".into())))), Box::new(Tpl::Num("0".into())), Box::new(Tpl::Num("1".into())), Box::new(Tpl::Sym("x".into())));
                    state.templates.insert(id, tpl); sync(session, state, id); state.focused_slot = None;
                }
            });
        }
    });
}
