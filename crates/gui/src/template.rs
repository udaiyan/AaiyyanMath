//! Structured mathematics templates.
//!
//! Palette clicks place real math objects with fillable slots. Layout boxes derive
//! their extents from the primitives they contain, so large operators are never
//! clipped. Identifier names render as the glyphs they name — `pi` draws π,
//! `delta_mm` draws δ with a subscript — and the tree serialises to ordinary
//! worksheet math for the parser, evaluator and Python generator.
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
        if let Tpl::Hole(n) = self {
            if *n == target { *self = with; return true; }
            return false;
        }
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
}

// ---------------------------------------------------------------- naming
/// Identifier names that stand for a glyph rather than a spelled-out word.
pub fn glyph(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "\u{03B1}", "beta" => "\u{03B2}", "gamma" => "\u{03B3}", "delta" => "\u{03B4}",
        "epsilon" => "\u{03B5}", "zeta" => "\u{03B6}", "eta" => "\u{03B7}", "theta" => "\u{03B8}",
        "iota" => "\u{03B9}", "kappa" => "\u{03BA}", "lambda" => "\u{03BB}", "mu" => "\u{03BC}",
        "nu" => "\u{03BD}", "xi" => "\u{03BE}", "omicron" => "\u{03BF}", "pi" => "\u{03C0}",
        "rho" => "\u{03C1}", "sigma" => "\u{03C3}", "tau" => "\u{03C4}", "upsilon" => "\u{03C5}",
        "phi" => "\u{03C6}", "chi" => "\u{03C7}", "psi" => "\u{03C8}", "omega" => "\u{03C9}",
        "Gamma" => "\u{0393}", "Delta" => "\u{0394}", "Theta" => "\u{0398}", "Lambda" => "\u{039B}",
        "Xi" => "\u{039E}", "Pi" => "\u{03A0}", "Sigma" => "\u{03A3}", "Upsilon" => "\u{03A5}",
        "Phi" => "\u{03A6}", "Psi" => "\u{03A8}", "Omega" => "\u{03A9}",
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

fn is_function(name: &str) -> bool { FUNCTIONS.contains(&name) }

// ---------------------------------------------------------------- constructors
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Make {
    Integral, Sum, Prod, Limit, Deriv, Frac, Pow, Sub, Sqrt, NthRoot, Abs,
    Matrix2, Matrix3, Convert, Define, Evaluate, Plus, Minus, Times, Divide, Leaf,
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

// ---------------------------------------------------------------- layout model
#[derive(Clone)]
enum Prim {
    Text(String, Rect, FontId, bool),
    Rule(Rect),
    Hole(Slot, Rect),
    Bracket(Rect, bool),
}

impl Prim {
    fn bounds(&self) -> Rect {
        match self {
            Prim::Text(_, r, _, _) | Prim::Rule(r) | Prim::Hole(_, r) | Prim::Bracket(r, _) => *r,
        }
    }
    fn shift(&mut self, d: Vec2) {
        match self {
            Prim::Text(_, r, _, _) | Prim::Rule(r) | Prim::Hole(_, r) | Prim::Bracket(r, _) => *r = r.translate(d),
        }
    }
}

#[derive(Clone)]
struct Node { w: f32, up: f32, down: f32, prims: Vec<Prim> }

impl Node {
    fn top(&self) -> f32 { -self.up }
    fn bottom(&self) -> f32 { self.down }
    fn shift(&mut self, dx: f32, dy: f32) {
        for p in &mut self.prims { p.shift(Vec2::new(dx, dy)); }
        self.up -= dy;
        self.down += dy;
    }
}

fn finish(mut prims: Vec<Prim>) -> Node {
    if prims.is_empty() { return Node { w: 0.0, up: 0.0, down: 0.0, prims }; }
    let mut b = prims[0].bounds();
    for p in prims.iter().skip(1) { b = b.union(p.bounds()); }
    let dx = -b.left();
    for p in &mut prims { p.shift(Vec2::new(dx, 0.0)); }
    Node { w: b.width().max(0.0), up: (-b.top()).max(0.0), down: b.bottom().max(0.0), prims }
}

fn measure(ui: &Ui, s: &str, font: &FontId) -> Vec2 {
    ui.fonts(|f| f.layout_no_wrap(s.to_string(), font.clone(), Color32::BLACK).size())
}

fn text_node(ui: &Ui, s: &str, font: FontId, italic: bool) -> Node {
    let sz = measure(ui, s, &font);
    let r = Rect::from_min_size(Pos2::new(0.0, -sz.y * 0.76), sz);
    finish(vec![Prim::Text(s.to_string(), r, font, italic)])
}

fn hole_node(slot: Slot, size: f32) -> Node {
    let w = (size * 0.80).max(13.0);
    let h = size * 0.92;
    finish(vec![Prim::Hole(slot, Rect::from_min_size(Pos2::new(0.0, -h * 0.78), Vec2::new(w, h)))])
}

fn row(parts: Vec<Node>, gap: f32) -> Node {
    let mut x = 0.0;
    let mut prims = Vec::new();
    for mut p in parts {
        p.shift(x, 0.0);
        x += p.w + gap;
        prims.extend(p.prims);
    }
    finish(prims)
}

fn frac(num: Node, den: Node, size: f32) -> Node {
    let pad = size * 0.22;
    let gap = size * 0.17;
    let w = num.w.max(den.w) + pad * 2.0;
    let bar_y = -size * 0.26;
    let bar_h = (size * 0.055).max(1.0);
    let mut prims = Vec::new();
    let mut n = num;
    n.shift((w - n.w) * 0.5, bar_y - gap - n.down);
    prims.extend(n.prims);
    prims.push(Prim::Rule(Rect::from_min_size(Pos2::new(0.0, bar_y), Vec2::new(w, bar_h))));
    let mut d = den;
    d.shift((w - d.w) * 0.5, bar_y + bar_h + gap + d.up);
    prims.extend(d.prims);
    finish(prims)
}

fn brackets(inner: Node, size: f32, square: bool) -> Node {
    let pad = size * 0.30;
    let h = inner.up + inner.down + size * 0.20;
    let top = -inner.up - size * 0.10;
    let w = inner.w + pad * 2.0;
    let mut prims = Vec::new();
    let mut b = inner;
    b.shift(pad, 0.0);
    prims.extend(b.prims);
    prims.push(Prim::Bracket(Rect::from_min_size(Pos2::new(0.0, top), Vec2::new(pad * 0.7, h)), square));
    prims.push(Prim::Bracket(Rect::from_min_size(Pos2::new(w - pad * 0.7, top), Vec2::new(pad * 0.7, h)), !square));
    finish(prims)
}

fn subscripted(base: Node, sub: Node, size: f32) -> Node {
    let bw = base.w;
    let mut s = sub;
    s.shift(bw + size * 0.04, size * 0.22 + s.up);
    let mut prims = base.prims;
    prims.extend(s.prims);
    finish(prims)
}

fn layout(ui: &Ui, tpl: &Tpl, size: f32) -> Node {
    let m = |s: f32| theme::math(s);
    let mi = |s: f32| theme::math_italic(s);
    match tpl {
        Tpl::Hole(n) => hole_node(*n, size),
        Tpl::Num(s) => text_node(ui, s, m(size), false),
        Tpl::Unit(s) => text_node(ui, s.trim(), m(size * 0.94), false),
        Tpl::Sym(s) => {
            let (base, sub) = match s.split_once('_') {
                Some((a, b)) if !a.is_empty() && !b.is_empty() => (a, Some(b)),
                _ => (s.as_str(), None),
            };
            let shown = glyph(base).unwrap_or(base);
            let upright = is_function(base);
            let node = text_node(ui, shown, if upright { m(size) } else { mi(size) }, !upright);
            match sub {
                Some(sx) => subscripted(node, layout(ui, &Tpl::Sym(sx.to_string()), size * 0.62), size),
                None => node,
            }
        }
        Tpl::Frac(a, b) => frac(layout(ui, a, size * 0.9), layout(ui, b, size * 0.9), size),
        Tpl::Pow(a, b) => {
            let base = layout(ui, a, size);
            let bw = base.w;
            let mut sup = layout(ui, b, size * 0.68);
            sup.shift(bw + size * 0.06, -size * 0.42 - sup.down);
            let mut prims = base.prims;
            prims.extend(sup.prims);
            finish(prims)
        }
        Tpl::Sub(a, b) => subscripted(layout(ui, a, size), layout(ui, b, size * 0.66), size),
        Tpl::Sqrt(a) => {
            let rad = text_node(ui, "\u{221A}", m(size * 1.25), false);
            let rw = rad.w;
            let mut body = layout(ui, a, size);
            body.shift(rw * 0.90, size * 0.14);
            let rule_y = body.top() - size * 0.14;
            let rule_w = body.w + size * 0.22;
            let mut prims = Vec::new();
            let mut r = rad;
            r.shift(0.0, size * 0.10);
            prims.extend(r.prims);
            prims.push(Prim::Rule(Rect::from_min_size(Pos2::new(rw * 0.80, rule_y), Vec2::new(rule_w, (size * 0.05).max(1.0)))));
            prims.extend(body.prims);
            finish(prims)
        }
        Tpl::NthRoot(deg, a) => {
            let inner = layout(ui, &Tpl::Sqrt(a.clone()), size);
            let mut d = layout(ui, deg, size * 0.58);
            let off = (d.w * 0.72).max(size * 0.30);
            d.shift(0.0, -size * 0.30);
            let mut prims = Vec::new();
            let mut i = inner;
            i.shift(off, 0.0);
            prims.extend(i.prims);
            prims.extend(d.prims);
            finish(prims)
        }
        Tpl::Abs(a) => {
            let body = layout(ui, a, size);
            let pad = size * 0.18;
            let bar = (size * 0.05).max(1.0);
            let h = body.up + body.down + size * 0.16;
            let top = -body.up - size * 0.08;
            let mut prims = Vec::new();
            let mut b = body;
            b.shift(pad + bar, 0.0);
            let bw = b.w;
            prims.extend(b.prims);
            prims.push(Prim::Rule(Rect::from_min_size(Pos2::new(pad, top), Vec2::new(bar, h))));
            prims.push(Prim::Rule(Rect::from_min_size(Pos2::new(pad + bar + bw + pad, top), Vec2::new(bar, h))));
            finish(prims)
        }
        Tpl::Paren(a) => brackets(layout(ui, a, size), size, true),
        Tpl::Integral(f, lo, hi, var) => {
            // Sign, then a limit column clear of its advance width, then integrand, then d·var.
            let mut sign = text_node(ui, "\u{222B}", m(size * 2.05), false);
            sign.shift(0.0, size * 0.34);
            let (sw, st, sb) = (sign.w, sign.top(), sign.bottom());
            let lim_x = sw + size * 0.06;
            let mut upper = layout(ui, hi, size * 0.62);
            upper.shift(lim_x, st + size * 0.06 + upper.up);
            let mut lower = layout(ui, lo, size * 0.62);
            lower.shift(lim_x, sb - size * 0.06 - lower.down);
            let lim_w = upper.w.max(lower.w);
            let body_x = lim_x + lim_w + size * 0.24;
            let mut body = layout(ui, f, size);
            body.shift(body_x, 0.0);
            let dx = body_x + body.w + size * 0.18;
            let mut d = text_node(ui, "d", m(size * 0.94), false);
            d.shift(dx, 0.0);
            let mut v = layout(ui, var, size * 0.94);
            v.shift(dx + d.w + size * 0.03, 0.0);
            let mut prims = sign.prims;
            prims.extend(upper.prims);
            prims.extend(lower.prims);
            prims.extend(body.prims);
            prims.extend(d.prims);
            prims.extend(v.prims);
            finish(prims)
        }
        Tpl::Sum(lo, hi, body) | Tpl::Prod(lo, hi, body) => {
            let ch = if matches!(tpl, Tpl::Sum(..)) { "\u{2211}" } else { "\u{220F}" };
            let mut sign = text_node(ui, ch, m(size * 1.70), false);
            sign.shift(0.0, size * 0.26);
            let (sw, st, sb) = (sign.w, sign.top(), sign.bottom());
            let mut upper = layout(ui, hi, size * 0.60);
            upper.shift((sw - upper.w) * 0.5, st - size * 0.12 - upper.down);
            let mut lower = layout(ui, lo, size * 0.60);
            lower.shift((sw - lower.w) * 0.5, sb + size * 0.12 + lower.up);
            let col_w = sw.max(upper.w).max(lower.w);
            let mut b = layout(ui, body, size);
            b.shift(col_w + size * 0.24, 0.0);
            let mut prims = sign.prims;
            prims.extend(upper.prims);
            prims.extend(lower.prims);
            prims.extend(b.prims);
            finish(prims)
        }
        Tpl::Limit(v, to, body) => {
            let lim = text_node(ui, "lim", m(size), false);
            let under = row(vec![
                layout(ui, v, size * 0.60),
                text_node(ui, "\u{2192}", m(size * 0.60), false),
                layout(ui, to, size * 0.60),
            ], size * 0.08);
            let col_w = lim.w.max(under.w);
            let mut l = lim;
            l.shift((col_w - l.w) * 0.5, 0.0);
            let lb = l.bottom();
            let mut u = under;
            u.shift((col_w - u.w) * 0.5, lb + size * 0.10 + u.up);
            let mut b = layout(ui, body, size);
            b.shift(col_w + size * 0.22, 0.0);
            let mut prims = l.prims;
            prims.extend(u.prims);
            prims.extend(b.prims);
            finish(prims)
        }
        Tpl::Deriv(f, v, n) => {
            let dn = if *n <= 1 { "d".to_string() } else { format!("d{}", n) };
            let num = row(vec![text_node(ui, &dn, m(size * 0.86), false), layout(ui, f, size * 0.86)], size * 0.06);
            let den = row(vec![text_node(ui, "d", m(size * 0.86), false), layout(ui, v, size * 0.86)], size * 0.06);
            frac(num, den, size)
        }
        Tpl::Matrix(rows) => {
            let nr = rows.len().max(1);
            let nc = rows.iter().map(|r| r.len()).max().unwrap_or(1);
            let cw = size * 1.7;
            let ch = size * 1.3;
            let pad = size * 0.36;
            let mut prims = Vec::new();
            for (ri, r) in rows.iter().enumerate() {
                for (ci, c) in r.iter().enumerate() {
                    let mut n = layout(ui, c, size * 0.9);
                    let cx = pad + ci as f32 * cw + (cw - n.w) * 0.5;
                    let cy = pad + ri as f32 * ch + (ch - (n.up + n.down)) * 0.5 + n.up;
                    n.shift(cx.max(pad), cy);
                    prims.extend(n.prims);
                }
            }
            let w = pad * 2.0 + nc as f32 * cw;
            let h = pad * 1.2 + nr as f32 * ch;
            prims.push(Prim::Bracket(Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(pad * 0.7, h)), true));
            prims.push(Prim::Bracket(Rect::from_min_size(Pos2::new(w - pad * 0.7, 0.0), Vec2::new(pad * 0.7, h)), false));
            finish(prims)
        }
        Tpl::Bin(op, a, b) => {
            let g = match op.as_str() { "*" => "\u{00B7}", "/" => "\u{00F7}", "-" => "\u{2212}", s => s };
            row(vec![layout(ui, a, size), text_node(ui, g, m(size), false), layout(ui, b, size)], size * 0.24)
        }
        Tpl::Assign(name, v) => row(vec![
            layout(ui, &Tpl::Sym(name.clone()), size),
            text_node(ui, ":=", m(size), false),
            layout(ui, v, size),
        ], size * 0.26),
        Tpl::Eval(a) => row(vec![layout(ui, a, size), text_node(ui, "=", m(size), false)], size * 0.26),
        Tpl::Convert(a, u) => row(vec![
            layout(ui, a, size),
            text_node(ui, "\u{2192}", m(size), false),
            layout(ui, u, size * 0.94),
        ], size * 0.24),
    }
}

// ---------------------------------------------------------------- painting
pub struct Painted { pub size: Vec2, pub slots: Vec<(Slot, Rect)> }

pub fn paint(ui: &mut Ui, state: &UiState, id: Uuid, t: &Theme, size: f32) -> Option<Painted> {
    let tpl = state.templates.get(&id)?.clone();
    let focus = state.focused_slot.filter(|(b, _)| *b == id).map(|(_, s)| s);
    let node = layout(ui, &tpl, size);
    let pad = 4.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(node.w + pad * 2.0, node.up + node.down + pad * 2.0), Sense::hover());
    let origin = rect.min + Vec2::new(pad, pad + node.up);
    let painter = ui.painter_at(rect);
    let mut slots = Vec::new();

    for p in &node.prims {
        match p {
            Prim::Text(s, r, font, _italic) => {
                painter.text(origin + r.min.to_vec2(), egui::Align2::LEFT_TOP, s, font.clone(), t.ink);
            }
            Prim::Rule(r) => { painter.rect_filled(r.translate(origin.to_vec2()), 0.5, t.ink); }
            Prim::Bracket(r, left) => {
                let r = r.translate(origin.to_vec2());
                let st = Stroke::new((size * 0.055).max(1.1_f32), t.ink_soft);
                let k = r.width() * 0.6;
                if *left {
                    painter.line_segment([r.right_top() - Vec2::new(k, 0.0), r.left_top()], st);
                    painter.line_segment([r.left_top(), r.left_bottom()], st);
                    painter.line_segment([r.left_bottom(), r.right_bottom() - Vec2::new(k, 0.0)], st);
                } else {
                    painter.line_segment([r.left_top() + Vec2::new(k, 0.0), r.right_top()], st);
                    painter.line_segment([r.right_top(), r.right_bottom()], st);
                    painter.line_segment([r.right_bottom(), r.left_bottom() + Vec2::new(k, 0.0)], st);
                }
            }
            Prim::Hole(slot, r) => {
                let r = r.translate(origin.to_vec2());
                let active = focus == Some(*slot);
                let anim = ui.ctx().animate_bool_with_time(egui::Id::new(("slot", id, *slot)), active, 0.12);
                if anim > 0.001 && anim < 0.999 { ui.ctx().request_repaint(); }
                painter.rect_filled(r, 2.0, t.accent_soft.linear_multiply(0.30 + 0.60 * anim));
                painter.rect_stroke(r, 2.0, Stroke::new(if active { 1.6_f32 } else { 1.0_f32 }, if active { t.accent } else { t.accent_line }));
                if !active {
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, "\u{25AB}", theme::math(size * 0.5), t.accent_line);
                }
                slots.push((*slot, r));
            }
        }
    }
    Some(Painted { size: rect.size(), slots })
}

pub fn slot_editor(
    ui: &mut Ui, ctx: &egui::Context, state: &mut UiState, session: &mut aaiyyan_core::Session,
    id: Uuid, t: &Theme, size: f32, slots: &[(Slot, Rect)],
) {
    let Some((bid, slot)) = state.focused_slot else { return };
    if bid != id { return; }
    let Some((_, r)) = slots.iter().find(|(s, _)| *s == slot).copied() else { return };

    let font = theme::math(size * 0.9);
    let text_w = ui.fonts(|f| f.layout_no_wrap(state.slot_buffer.clone(), font.clone(), t.ink).size().x);
    let w = (text_w + 18.0).max(r.width() + 8.0);
    let rect = Rect::from_min_size(r.min - Vec2::new(3.0, 3.0), Vec2::new(w + 6.0, r.height() + 6.0));

    let mut buf = state.slot_buffer.clone();
    let resp = ui.allocate_ui_at_rect(rect, |ui| {
        ui.add(egui::TextEdit::singleline(&mut buf)
            .id(egui::Id::new(("slotedit", id, slot)))
            .font(font)
            .desired_width(w)
            .margin(3.0)
            .hint_text("value"))
    }).inner;

    if state.slot_focus == Some((id, slot)) {
        resp.request_focus();
        state.slot_focus = None;
    }
    if resp.changed() { state.slot_buffer = buf.clone(); }

    let advance = ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Tab));
    if advance || resp.lost_focus() {
        commit(state, session, id, slot, &buf);
        let next = next_slot(state, id);
        state.focused_slot = next.map(|s| (id, s));
        state.slot_focus = state.focused_slot;
        state.slot_buffer.clear();
    }
}

fn commit(state: &mut UiState, session: &mut aaiyyan_core::Session, id: Uuid, slot: Slot, buf: &str) {
    let s = buf.trim();
    if s.is_empty() { return; }
    if let Some(mut tpl) = state.templates.get(&id).cloned() {
        let numeric = s.chars().next().map(|c| c.is_ascii_digit() || c == '-' || c == '.').unwrap_or(false);
        let leaf = if numeric { Tpl::Num(s.to_string()) } else { Tpl::Sym(s.to_string()) };
        if tpl.fill(slot, leaf) {
            state.templates.insert(id, tpl);
            sync(session, state, id);
        }
    }
}

// ---------------------------------------------------------------- round-trip
pub fn from_source(src: &str) -> Option<Tpl> {
    let s = src.trim();
    if s.is_empty() { return None; }
    let mut next: Slot = 1;
    parse_top(s, &mut next)
}

fn parse_top(s: &str, next: &mut Slot) -> Option<Tpl> {
    if let Some(eq) = find_top(s, ":=") {
        return Some(Tpl::Assign(s[..eq].trim().to_string(), Box::new(parse_expr(s[eq + 2..].trim(), next)?)));
    }
    if s.ends_with('=') && !s.ends_with("==") && !s.ends_with(":=") {
        return Some(Tpl::Eval(Box::new(parse_expr(s[..s.len() - 1].trim(), next)?)));
    }
    parse_expr(s, next)
}

fn find_top(s: &str, needle: &str) -> Option<usize> {
    let mut depth = 0i32;
    let b = s.as_bytes();
    for i in 0..s.len() {
        match b[i] { b'(' | b'[' => depth += 1, b')' | b']' => depth -= 1, _ => {} }
        if depth == 0 && s[i..].starts_with(needle) { return Some(i); }
    }
    None
}

fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '(' | '[' => { depth += 1; cur.push(c); }
            ')' | ']' => { depth -= 1; cur.push(c); }
            ',' if depth == 0 => { out.push(cur.trim().to_string()); cur.clear(); }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() { out.push(cur.trim().to_string()); }
    out
}

fn balanced(s: &str) -> bool {
    let mut d = 0i32;
    for c in s.chars() {
        match c { '(' | '[' => d += 1, ')' | ']' => d -= 1, _ => {} }
        if d < 0 { return false; }
    }
    d == 0
}

fn find_top_op(s: &str, op: &str) -> Option<usize> {
    let mut depth = 0i32;
    let b = s.as_bytes();
    let mut last = None;
    for i in 0..s.len() {
        match b[i] { b'(' | b'[' => depth += 1, b')' | b']' => depth -= 1, _ => {} }
        if depth == 0 && i > 0 && s[i..].starts_with(op) {
            let prev = b[i - 1];
            if prev != b'(' && prev != b'[' && prev != b'*' && prev != b'/' && prev != b'^' && prev != b'e' && prev != b'E' {
                last = Some(i);
            }
        }
    }
    last
}

fn parse_expr(s: &str, next: &mut Slot) -> Option<Tpl> {
    let s = s.trim();
    if s.is_empty() { return None; }
    if s.starts_with('(') && s.ends_with(')') && balanced(&s[1..s.len() - 1]) {
        return parse_expr(&s[1..s.len() - 1], next);
    }
    for op in ["+", "-"] {
        if let Some(i) = find_top_op(s, op) {
            return Some(Tpl::Bin(op.into(), Box::new(parse_expr(&s[..i], next)?), Box::new(parse_expr(&s[i + 1..], next)?)));
        }
    }
    for op in ["*", "/"] {
        if let Some(i) = find_top_op(s, op) {
            return Some(Tpl::Bin(op.into(), Box::new(parse_expr(&s[..i], next)?), Box::new(parse_expr(&s[i + 1..], next)?)));
        }
    }
    if let Some(i) = find_top_op(s, "^") {
        return Some(Tpl::Pow(Box::new(parse_expr(&s[..i], next)?), Box::new(parse_expr(&s[i + 1..], next)?)));
    }
    if s.starts_with("[[") && s.ends_with("]]") {
        let rows: Vec<Vec<Tpl>> = split_args(&s[1..s.len() - 1]).iter().filter_map(|r| {
            let r = r.trim();
            if !(r.starts_with('[') && r.ends_with(']')) { return None; }
            Some(split_args(&r[1..r.len() - 1]).iter().filter_map(|c| parse_expr(c, next)).collect())
        }).collect();
        if !rows.is_empty() { return Some(Tpl::Matrix(rows)); }
    }
    if let Some(open) = s.find('(') {
        if s.ends_with(')') {
            let name = s[..open].trim();
            let args = split_args(&s[open + 1..s.len() - 1]);
            let p = |i: usize, n: &mut Slot| args.get(i).and_then(|a| parse_expr(a, n));
            match name {
                "integrate" if args.len() >= 3 => {
                    return Some(Tpl::Integral(Box::new(p(0, next)?), Box::new(p(1, next)?), Box::new(p(2, next)?), Box::new(Tpl::Sym("x".into()))));
                }
                "sumrange" if args.len() == 3 => return Some(Tpl::Sum(Box::new(p(0, next)?), Box::new(p(1, next)?), Box::new(p(2, next)?))),
                "prodrange" if args.len() == 3 => return Some(Tpl::Prod(Box::new(p(0, next)?), Box::new(p(1, next)?), Box::new(p(2, next)?))),
                "limit" if args.len() == 3 => return Some(Tpl::Limit(Box::new(p(1, next)?), Box::new(p(2, next)?), Box::new(p(0, next)?))),
                "nderiv" if args.len() >= 2 => {
                    let n = args.get(2).and_then(|v| v.parse::<u32>().ok()).unwrap_or(1);
                    return Some(Tpl::Deriv(Box::new(p(0, next)?), Box::new(p(1, next)?), n));
                }
                "sqrt" if args.len() == 1 => return Some(Tpl::Sqrt(Box::new(p(0, next)?))),
                "nroot" if args.len() == 2 => return Some(Tpl::NthRoot(Box::new(p(1, next)?), Box::new(p(0, next)?))),
                "abs" if args.len() == 1 => return Some(Tpl::Abs(Box::new(p(0, next)?))),
                "convert" if args.len() == 2 => return Some(Tpl::Convert(Box::new(p(0, next)?), Box::new(p(1, next)?))),
                _ => {}
            }
        }
    }
    if let Some(stripped) = s.strip_prefix("hole") {
        if let Ok(n) = stripped.parse::<Slot>() { *next = (*next).max(n + 1); return Some(Tpl::Hole(n)); }
    }
    if s.chars().any(|c| c.is_ascii_digit()) && s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e') {
        return Some(Tpl::Num(s.to_string()));
    }
    Some(Tpl::Sym(s.to_string()))
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
    if let Some((id, slot)) = state.focused_slot {
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
    let tpl = build(kind, 1);
    let id = session.insert_block_at(aaiyyan_worksheet::BlockKind::Equation { source: tpl.to_source() }, at.0, at.1);
    state.templates.insert(id, tpl);
    state.selected = Some(id);
    state.editing = None;
    state.cursor = None;
    let n = next_slot(state, id);
    state.focused_slot = n.map(|s| (id, s));
    state.slot_focus = state.focused_slot;
    state.slot_buffer.clear();
}

pub fn insert_leaf(state: &mut UiState, session: &mut aaiyyan_core::Session, text: &str, unit: bool, at: (f32, f32)) {
    let raw = text.trim();
    let numeric = raw.chars().next().map(|c| c.is_ascii_digit() || c == '-' || c == '.').unwrap_or(false);
    let leaf = if unit { Tpl::Unit(raw.to_string()) } else if numeric { Tpl::Num(raw.to_string()) } else { Tpl::Sym(raw.to_string()) };
    if let Some((id, slot)) = state.focused_slot {
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
    state.cursor = None;
    state.focused_slot = None;
}

/// Live controls for the selected math object, including its width.
pub fn toolbar(ui: &mut Ui, state: &mut UiState, session: &mut aaiyyan_core::Session, actions: &mut Vec<UiAction>, t: &Theme) {
    let Some(id) = state.selected else { return };
    if !state.templates.contains_key(&id) { return; }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(egui::RichText::new("MATH OBJECT").size(9.5).strong().color(t.ink_faint));
        ui.add_space(2.0);
        ui.label(egui::RichText::new("size").size(10.5).color(t.ink_soft));
        ui.add(egui::Slider::new(&mut state.math_size, 12.0..=40.0).show_value(false).text(""));
        ui.label(egui::RichText::new(format!("{:.0}", state.math_size)).size(10.5).color(t.ink));
        ui.add_space(4.0);
        ui.label(egui::RichText::new("width").size(10.5).color(t.ink_soft));
        let mut w = state.widths.get(&id).copied().unwrap_or(0.0);
        if ui.add(egui::DragValue::new(&mut w).range(0.0..=2400.0).speed(2.0).suffix(" px"))
            .on_hover_text("0 = auto-fit to the equation").changed()
        {
            if w <= 1.0 { state.widths.remove(&id); } else { state.widths.insert(id, w); }
        }
        if ui.small_button("Fit").on_hover_text("Auto-fit width so the equation stays on one line").clicked() {
            state.widths.remove(&id);
        }
        ui.add_space(6.0);
        if ui.small_button("Next slot").on_hover_text("Tab").clicked() {
            let n = next_slot(state, id);
            state.focused_slot = n.map(|s| (id, s));
            state.slot_focus = state.focused_slot;
            state.slot_buffer.clear();
        }
        if ui.small_button("Clear slot").clicked() {
            if let Some((bid, slot)) = state.focused_slot {
                if let Some(mut tp) = state.templates.get(&bid).cloned() {
                    tp.fill(slot, Tpl::Hole(slot));
                    state.templates.insert(bid, tp);
                    sync(session, state, bid);
                    state.slot_buffer.clear();
                    state.slot_focus = Some((bid, slot));
                }
            }
        }
        for (label, kind, tip) in [
            ("x\u{207F}", Make::Pow, "Wrap the slot in a power"),
            ("\u{221A}", Make::Sqrt, "Wrap the slot in a root"),
            ("a/b", Make::Frac, "Wrap the slot in a fraction"),
            ("\u{222B}", Make::Integral, "Insert an integral into the slot"),
        ] {
            if ui.small_button(label).on_hover_text(tip).clicked() {
                if let Some((bid, slot)) = state.focused_slot { actions.push(UiAction::TemplateMake(kind, bid, slot)); }
            }
        }
        if ui.small_button("Evaluate =").clicked() {
            if let Some(tp) = state.templates.get(&id).cloned() {
                if !matches!(tp, Tpl::Eval(_)) {
                    state.templates.insert(id, Tpl::Eval(Box::new(tp)));
                    sync(session, state, id);
                }
            }
        }
        if ui.small_button("Define :=").clicked() {
            if let Some(tp) = state.templates.get(&id).cloned() {
                if !matches!(tp, Tpl::Assign(..)) {
                    state.templates.insert(id, Tpl::Assign("f".into(), Box::new(tp)));
                    sync(session, state, id);
                }
            }
        }
        ui.add_space(4.0);
        ui.label(egui::RichText::new("matrix").size(10.5).color(t.ink_soft));
        let mut dims = match state.templates.get(&id) {
            Some(Tpl::Matrix(r)) => (r.len(), r.first().map(|c| c.len()).unwrap_or(0)),
            _ => (2, 2),
        };
        let mut changed = false;
        changed |= ui.add(egui::DragValue::new(&mut dims.0).range(1..=8).speed(0.05)).changed();
        changed |= ui.add(egui::DragValue::new(&mut dims.1).range(1..=8).speed(0.05)).changed();
        if changed {
            let mut next = state.templates.get(&id).map(|t| t.max_slot()).unwrap_or(0) + 1;
            let rows: Vec<Vec<Tpl>> = (0..dims.0).map(|_| (0..dims.1).map(|_| { let h = Tpl::Hole(next); next += 1; h }).collect()).collect();
            state.templates.insert(id, Tpl::Matrix(rows));
            sync(session, state, id);
        }
        ui.add_space(4.0);
        if ui.small_button("Reset").clicked() {
            state.templates.insert(id, Tpl::Hole(1));
            sync(session, state, id);
            state.focused_slot = Some((id, 1));
            state.slot_focus = Some((id, 1));
            state.slot_buffer.clear();
        }
        if ui.small_button("Copy").on_hover_text("Copy the worksheet source").clicked() {
            if let Some(tp) = state.templates.get(&id) { ui.ctx().copy_text(tp.to_source()); }
        }
    });
}
