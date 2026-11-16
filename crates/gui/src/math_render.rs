//! Mathcad-style typesetting: serif faces, italic variables, upright functions,
//! teal units, real fraction bars, radicals, brackets and big operators.
use crate::theme::{self, Theme};
use aaiyyan_ast::{BinaryOp, Expr, Statement, UnaryOp};
use aaiyyan_evaluator::Value;
use egui::{Align2, Color32, FontId, Painter, Pos2, Sense, Ui, Vec2};

#[derive(Clone, Copy, PartialEq)]
pub enum Style { Var, Num, Func, Unit, Op }

pub enum Node {
    Text(String, Style),
    Space(f32),
    Row(Vec<Node>),
    Frac(Box<Node>, Box<Node>),
    Pow(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Sqrt(Box<Node>),
    Matrix(Vec<Vec<Node>>),
    BigOp { symbol: String, lower: Option<Box<Node>>, upper: Option<Box<Node>>, body: Box<Node> },
}

impl Node {
    fn row(items: Vec<Node>) -> Node { Node::Row(items) }
    fn t(s: impl Into<String>, style: Style) -> Node { Node::Text(s.into(), style) }
}

#[derive(Clone, Copy, Default)]
struct Metrics { size: Vec2, ascent: f32 }

impl Metrics {
    fn descent(&self) -> f32 { self.size.y - self.ascent }
}

fn font_for(style: Style, size: f32) -> FontId {
    match style {
        Style::Var => theme::math_italic(size),
        _ => theme::math(size),
    }
}

fn color_for(style: Style, t: &Theme) -> Color32 {
    match style {
        Style::Unit => t.unit,
        Style::Op => t.ink_soft,
        _ => t.ink,
    }
}

fn greek(s: &str) -> Option<&'static str> {
    Some(match s {
        "alpha" => "\u{03B1}", "beta" => "\u{03B2}", "gamma" => "\u{03B3}", "delta" => "\u{03B4}",
        "epsilon" => "\u{03B5}", "theta" => "\u{03B8}", "lambda" => "\u{03BB}", "mu" => "\u{03BC}",
        "nu" => "\u{03BD}", "xi" => "\u{03BE}", "pi" => "\u{03C0}", "rho" => "\u{03C1}",
        "sigma" => "\u{03C3}", "tau" => "\u{03C4}", "phi" => "\u{03C6}", "chi" => "\u{03C7}",
        "psi" => "\u{03C8}", "omega" => "\u{03C9}", "Delta" => "\u{0394}", "Omega" => "\u{03A9}",
        "Sigma" => "\u{03A3}", "Phi" => "\u{03A6}", "Gamma" => "\u{0393}", "Theta" => "\u{0398}",
        _ => return None,
    })
}

fn symbol_node(name: &str) -> Node {
    let style = if aaiyyan_units::is_unit_name(name) { Style::Unit } else { Style::Var };
    let base_text = |s: &str| greek(s).unwrap_or(s).to_string();
    if let Some((base, sub)) = name.split_once('_') {
        return Node::Sub(
            Box::new(Node::t(base_text(base), style)),
            Box::new(Node::t(base_text(sub), Style::Num)),
        );
    }
    Node::t(base_text(name), style)
}

fn op_text(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "\u{2212}",
        BinaryOp::Mul => "\u{00B7}",
        BinaryOp::ElemMul => "\u{2299}",
        BinaryOp::ElemDiv => "\u{2298}",
        BinaryOp::Mod => "mod",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::Eq => "=",
        BinaryOp::Ne => "\u{2260}",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "\u{2264}",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => "\u{2265}",
        _ => "?",
    }
}

pub fn expr_node(expr: &Expr) -> Node {
    match expr {
        Expr::Number(v) => Node::t(aaiyyan_ast::latex::format_number(*v), Style::Num),
        Expr::Imaginary => Node::t("i", Style::Var),
        Expr::Bool(b) => Node::t(if *b { "true" } else { "false" }, Style::Func),
        Expr::Str(s) => Node::t(format!("\u{201C}{s}\u{201D}"), Style::Num),
        Expr::Symbol(s) => symbol_node(s),
        Expr::Unary { op, expr } => match op {
            UnaryOp::Neg => Node::row(vec![Node::t("\u{2212}", Style::Op), wrap(expr, 16)]),
            UnaryOp::Pos => wrap(expr, 16),
            UnaryOp::Not => Node::row(vec![Node::t("not ", Style::Func), wrap(expr, 6)]),
            UnaryOp::Factorial => Node::row(vec![wrap(expr, 20), Node::t("!", Style::Op)]),
            UnaryOp::Transpose => Node::Pow(Box::new(wrap(expr, 20)), Box::new(Node::t("T", Style::Func))),
        },
        Expr::Binary { op, lhs, rhs } => match op {
            BinaryOp::Div => Node::Frac(Box::new(expr_node(lhs)), Box::new(expr_node(rhs))),
            BinaryOp::Pow | BinaryOp::ElemPow => Node::Pow(Box::new(wrap(lhs, 19)), Box::new(expr_node(rhs))),
            BinaryOp::Mul => {
                let left = wrap(lhs, 12);
                let right = wrap(rhs, 13);
                let tight = matches!(rhs.as_ref(), Expr::Symbol(s) if aaiyyan_units::is_unit_name(s));
                Node::row(vec![left, Node::Space(if tight { 0.22 } else { 0.14 }), right])
            }
            _ => Node::row(vec![
                wrap(lhs, op.precedence()),
                Node::Space(0.22),
                Node::t(op_text(*op), Style::Op),
                Node::Space(0.22),
                wrap(rhs, op.precedence() + 1),
            ]),
        },
        Expr::Call { name, args } => call_node(name, args),
        Expr::Matrix(rows) => {
            let grid: Vec<Vec<Node>> = rows.iter()
                .map(|r| r.iter().map(expr_node).collect())
                .collect();
            Node::Matrix(grid)
        }
        Expr::Vector(items) => {
            let cells: Vec<Node> = items.iter().map(expr_node).collect();
            Node::Matrix(vec![cells])
        }
        Expr::Range { start, step, end } => {
            let mut parts = vec![expr_node(start), Node::t(",", Style::Op), Node::Space(0.18)];
            if let Some(s) = step {
                parts.push(expr_node(s));
                parts.push(Node::t(",", Style::Op));
                parts.push(Node::Space(0.18));
            }
            parts.push(Node::t("\u{2026}", Style::Op));
            parts.push(Node::t(",", Style::Op));
            parts.push(Node::Space(0.18));
            parts.push(expr_node(end));
            Node::row(parts)
        }
        Expr::Index { target, indices } => {
            let mut parts = Vec::new();
            for (i, a) in indices.iter().enumerate() {
                if i > 0 { parts.push(Node::t(",", Style::Op)); }
                parts.push(expr_node(a));
            }
            Node::Sub(Box::new(expr_node(target)), Box::new(Node::row(parts)))
        }
        Expr::Conditional { cond, then, otherwise } => Node::row(vec![
            Node::t("if ", Style::Func), expr_node(cond),
            Node::t("  then ", Style::Func), expr_node(then),
            Node::t("  else ", Style::Func), expr_node(otherwise),
        ]),
    }
}

fn call_node(name: &str, args: &[Expr]) -> Node {
    match name {
        "sqrt" if !args.is_empty() => Node::Sqrt(Box::new(expr_node(&args[0]))),
        "abs" if !args.is_empty() => Node::row(vec![
            Node::t("|", Style::Op), expr_node(&args[0]), Node::t("|", Style::Op),
        ]),
        "diff" if args.len() >= 2 => Node::row(vec![
            Node::Frac(
                Box::new(Node::t("d", Style::Func)),
                Box::new(Node::row(vec![Node::t("d", Style::Func), expr_node(&args[1])])),
            ),
            Node::Space(0.12),
            Node::t("(", Style::Op), expr_node(&args[0]), Node::t(")", Style::Op),
        ]),
        "integrate" => {
            let (lower, upper) = if args.len() >= 3 {
                (Some(Box::new(expr_node(&args[1]))), Some(Box::new(expr_node(&args[2]))))
            } else { (None, None) };
            Node::BigOp {
                symbol: "\u{222B}".into(), lower, upper,
                body: Box::new(args.first().map(expr_node).unwrap_or(Node::t("", Style::Num))),
            }
        }
        "sum" | "prod" => Node::BigOp {
            symbol: if name == "sum" { "\u{2211}".into() } else { "\u{220F}".into() },
            lower: args.get(1).map(|a| Box::new(expr_node(a))),
            upper: args.get(2).map(|a| Box::new(expr_node(a))),
            body: Box::new(args.first().map(expr_node).unwrap_or(Node::t("", Style::Num))),
        },
        _ => {
            let mut parts = vec![Node::t(name, Style::Func), Node::t("(", Style::Op)];
            for (i, a) in args.iter().enumerate() {
                if i > 0 { parts.push(Node::t(",", Style::Op)); parts.push(Node::Space(0.16)); }
                parts.push(expr_node(a));
            }
            parts.push(Node::t(")", Style::Op));
            Node::row(parts)
        }
    }
}

fn wrap(expr: &Expr, parent: u8) -> Node {
    let node = expr_node(expr);
    if expr.precedence() < parent {
        Node::row(vec![Node::t("(", Style::Op), node, Node::t(")", Style::Op)])
    } else {
        node
    }
}

/// Result typesetting. Functions and bare units have no numeric echo.
pub fn value_node(value: &Value) -> Option<Node> {
    match value {
        Value::Function(_) | Value::Unit(_) => None,
        Value::Scalar(v) => Some(Node::t(aaiyyan_ast::latex::format_number(*v), Style::Num)),
        Value::Bool(b) => Some(Node::t(b.to_string(), Style::Func)),
        Value::Str(s) => Some(Node::t(format!("\u{201C}{s}\u{201D}"), Style::Num)),
        Value::Complex { re, im } => Some(Node::row(vec![
            Node::t(aaiyyan_ast::latex::format_number(*re), Style::Num),
            Node::Space(0.18),
            Node::t(if *im < 0.0 { "\u{2212}" } else { "+" }, Style::Op),
            Node::Space(0.18),
            Node::t(aaiyyan_ast::latex::format_number(im.abs()), Style::Num),
            Node::t("i", Style::Var),
        ])),
        Value::Quantity(q) => {
            let mut parts = vec![Node::t(aaiyyan_ast::latex::format_number(q.value()), Style::Num)];
            if !q.unit.name.is_empty() {
                parts.push(Node::Space(0.28));
                parts.push(unit_node(&q.unit.name));
            }
            Some(Node::row(parts))
        }
        Value::Vector(v) => {
            let cells: Vec<Node> = v.iter().take(24)
                .map(|x| Node::t(aaiyyan_ast::latex::format_number(*x), Style::Num))
                .collect();
            Some(Node::Matrix(vec![cells]))
        }
        Value::Matrix(m) => {
            let grid: Vec<Vec<Node>> = (0..m.rows.min(24))
                .map(|r| (0..m.cols.min(12))
                    .map(|c| Node::t(aaiyyan_ast::latex::format_number(m.get(r, c)), Style::Num))
                    .collect())
                .collect();
            Some(Node::Matrix(grid))
        }
        Value::Symbolic { expr, .. } => Some(Node::t(expr.clone(), Style::Var)),
        Value::Table(t) => Some(Node::t(format!("table {} \u{00D7} {}", t.rows.len(), t.columns.len()), Style::Func)),
    }
}

fn unit_node(name: &str) -> Node {
    let mut parts: Vec<Node> = Vec::new();
    let mut first = true;
    for chunk in name.split('/') {
        if !first { parts.push(Node::t("/", Style::Unit)); }
        first = false;
        let mut inner_first = true;
        for term in chunk.trim_matches(|c| c == '(' || c == ')').split('*') {
            if !inner_first { parts.push(Node::t("\u{00B7}", Style::Unit)); }
            inner_first = false;
            if let Some((base, exp)) = term.split_once('^') {
                parts.push(Node::Pow(Box::new(Node::t(base, Style::Unit)), Box::new(Node::t(exp, Style::Unit))));
            } else {
                parts.push(Node::t(term, Style::Unit));
            }
        }
    }
    Node::row(parts)
}

fn lhs_node(stmt: &Statement) -> Option<Node> {
    match stmt {
        Statement::Assign { name, .. } => Some(symbol_node(name)),
        Statement::FuncDef { name, params, .. } => {
            let mut parts = vec![symbol_node(name), Node::t("(", Style::Op)];
            for (i, p) in params.iter().enumerate() {
                if i > 0 { parts.push(Node::t(",", Style::Op)); parts.push(Node::Space(0.14)); }
                parts.push(symbol_node(p));
            }
            parts.push(Node::t(")", Style::Op));
            Some(Node::row(parts))
        }
        Statement::Expr(_) => None,
    }
}

/// `name := expression = result unit` on one baseline, exactly like Mathcad.
pub fn definition_node(stmt: &Statement, value: Option<&Value>, echo: bool) -> Node {
    let mut parts = Vec::new();
    if let Some(lhs) = lhs_node(stmt) {
        parts.push(lhs);
        parts.push(Node::Space(0.30));
        parts.push(Node::t(":=", Style::Op));
        parts.push(Node::Space(0.30));
    }
    parts.push(expr_node(stmt.value_expr()));
    if echo {
        if let Some(v) = value.and_then(value_node) {
            parts.push(Node::Space(0.42));
            parts.push(Node::t("=", Style::Op));
            parts.push(Node::Space(0.34));
            parts.push(v);
        }
    }
    Node::row(parts)
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

fn text_metrics(ui: &Ui, text: &str, style: Style, size: f32) -> Metrics {
    let galley = ui.fonts(|f| f.layout_no_wrap(text.to_string(), font_for(style, size), Color32::BLACK));
    let s = galley.size();
    Metrics { size: s, ascent: s.y * 0.76 }
}

fn measure(ui: &Ui, node: &Node, size: f32) -> Metrics {
    match node {
        Node::Text(t, style) => text_metrics(ui, t, *style, size),
        Node::Space(f) => Metrics { size: Vec2::new(size * f, size * 0.5), ascent: size * 0.4 },
        Node::Row(items) => {
            let (mut w, mut asc, mut desc) = (0.0f32, 0.0f32, 0.0f32);
            for item in items {
                let m = measure(ui, item, size);
                w += m.size.x;
                asc = asc.max(m.ascent);
                desc = desc.max(m.descent());
            }
            Metrics { size: Vec2::new(w, asc + desc), ascent: asc }
        }
        Node::Frac(n, d) => {
            let a = measure(ui, n, size * 0.94);
            let b = measure(ui, d, size * 0.94);
            let ascent = a.size.y + size * 0.20;
            let descent = b.size.y + size * 0.16;
            Metrics { size: Vec2::new(a.size.x.max(b.size.x) + size * 0.36, ascent + descent), ascent }
        }
        Node::Pow(b, e) => {
            let bm = measure(ui, b, size);
            let em = measure(ui, e, size * 0.68);
            Metrics {
                size: Vec2::new(bm.size.x + em.size.x + 1.0, bm.size.y + em.size.y * 0.42),
                ascent: bm.ascent + em.size.y * 0.42,
            }
        }
        Node::Sub(b, s) => {
            let bm = measure(ui, b, size);
            let sm = measure(ui, s, size * 0.68);
            Metrics {
                size: Vec2::new(bm.size.x + sm.size.x + 1.0, bm.size.y + sm.size.y * 0.30),
                ascent: bm.ascent,
            }
        }
        Node::Sqrt(inner) => {
            let m = measure(ui, inner, size);
            Metrics {
                size: Vec2::new(m.size.x + size * 0.78, m.size.y + size * 0.22),
                ascent: m.ascent + size * 0.22,
            }
        }
        Node::Matrix(rows) => {
            let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
            let mut col_w = vec![0.0f32; cols];
            let mut row_h = vec![0.0f32; rows.len()];
            for (i, row) in rows.iter().enumerate() {
                for (j, cell) in row.iter().enumerate() {
                    let m = measure(ui, cell, size * 0.94);
                    col_w[j] = col_w[j].max(m.size.x);
                    row_h[i] = row_h[i].max(m.size.y);
                }
            }
            let w = col_w.iter().sum::<f32>() + cols as f32 * size * 0.42 + size * 0.72;
            let h = row_h.iter().sum::<f32>() + rows.len() as f32 * size * 0.22;
            Metrics { size: Vec2::new(w, h), ascent: h * 0.5 + size * 0.26 }
        }
        Node::BigOp { symbol, lower, upper, body } => {
            let sym = text_metrics(ui, symbol, Style::Op, size * 1.55);
            let lo = lower.as_ref().map(|n| measure(ui, n, size * 0.62));
            let up = upper.as_ref().map(|n| measure(ui, n, size * 0.62));
            let bm = measure(ui, body, size);
            let col = sym.size.x.max(lo.map(|m| m.size.x).unwrap_or(0.0)).max(up.map(|m| m.size.x).unwrap_or(0.0));
            let h = sym.size.y + lo.map(|m| m.size.y).unwrap_or(0.0) + up.map(|m| m.size.y).unwrap_or(0.0);
            Metrics { size: Vec2::new(col + bm.size.x + size * 0.24, h.max(bm.size.y)), ascent: h * 0.56 + size * 0.2 }
        }
    }
}

fn paint(ui: &Ui, painter: &Painter, node: &Node, top_left: Pos2, m: Metrics, size: f32, t: &Theme) {
    let baseline = top_left.y + m.ascent;
    match node {
        Node::Text(text, style) => {
            painter.text(top_left, Align2::LEFT_TOP, text.clone(), font_for(*style, size), color_for(*style, t));
        }
        Node::Space(_) => {}
        Node::Row(items) => {
            let mut x = top_left.x;
            for item in items {
                let im = measure(ui, item, size);
                paint(ui, painter, item, Pos2::new(x, baseline - im.ascent), im, size, t);
                x += im.size.x;
            }
        }
        Node::Frac(n, d) => {
            let nm = measure(ui, n, size * 0.94);
            let dm = measure(ui, d, size * 0.94);
            let cx = top_left.x + m.size.x * 0.5;
            paint(ui, painter, n, Pos2::new(cx - nm.size.x * 0.5, top_left.y), nm, size * 0.94, t);
            let bar_y = baseline - size * 0.02;
            painter.line_segment(
                [Pos2::new(top_left.x + size * 0.10, bar_y), Pos2::new(top_left.x + m.size.x - size * 0.10, bar_y)],
                egui::Stroke::new((size * 0.055).max(1.0_f32), t.ink),
            );
            paint(ui, painter, d, Pos2::new(cx - dm.size.x * 0.5, bar_y + size * 0.14), dm, size * 0.94, t);
        }
        Node::Pow(b, e) => {
            let bm = measure(ui, b, size);
            let em = measure(ui, e, size * 0.68);
            paint(ui, painter, b, Pos2::new(top_left.x, baseline - bm.ascent), bm, size, t);
            paint(ui, painter, e, Pos2::new(top_left.x + bm.size.x + 1.0, top_left.y), em, size * 0.68, t);
        }
        Node::Sub(b, s) => {
            let bm = measure(ui, b, size);
            let sm = measure(ui, s, size * 0.68);
            paint(ui, painter, b, top_left, bm, size, t);
            paint(ui, painter, s, Pos2::new(top_left.x + bm.size.x + 1.0, top_left.y + bm.size.y * 0.56), sm, size * 0.68, t);
        }
        Node::Sqrt(inner) => {
            let im = measure(ui, inner, size);
            let x0 = top_left.x;
            let bottom = baseline + im.descent();
            let st = egui::Stroke::new((size * 0.055).max(1.0_f32), t.ink);
            painter.line_segment([Pos2::new(x0, bottom - size * 0.34), Pos2::new(x0 + size * 0.16, bottom - size * 0.16)], st);
            painter.line_segment([Pos2::new(x0 + size * 0.16, bottom - size * 0.16), Pos2::new(x0 + size * 0.38, top_left.y + size * 0.10)], st);
            painter.line_segment([Pos2::new(x0 + size * 0.38, top_left.y + size * 0.10), Pos2::new(x0 + m.size.x, top_left.y + size * 0.10)], st);
            painter.line_segment([Pos2::new(x0 + m.size.x, top_left.y + size * 0.10), Pos2::new(x0 + m.size.x, top_left.y + size * 0.26)], st);
            paint(ui, painter, inner, Pos2::new(x0 + size * 0.52, top_left.y + size * 0.20), im, size, t);
        }
        Node::Matrix(rows) => {
            let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
            let mut col_w = vec![0.0f32; cols];
            let mut row_h = vec![0.0f32; rows.len()];
            for (i, row) in rows.iter().enumerate() {
                for (j, cell) in row.iter().enumerate() {
                    let cm = measure(ui, cell, size * 0.94);
                    col_w[j] = col_w[j].max(cm.size.x);
                    row_h[i] = row_h[i].max(cm.size.y);
                }
            }
            let st = egui::Stroke::new((size * 0.05).max(1.0_f32), t.ink);
            let h = m.size.y;
            let left = top_left.x + size * 0.16;
            let right = top_left.x + m.size.x - size * 0.16;
            let tick = size * 0.20;
            painter.line_segment([Pos2::new(left, top_left.y), Pos2::new(left, top_left.y + h)], st);
            painter.line_segment([Pos2::new(left, top_left.y), Pos2::new(left + tick, top_left.y)], st);
            painter.line_segment([Pos2::new(left, top_left.y + h), Pos2::new(left + tick, top_left.y + h)], st);
            painter.line_segment([Pos2::new(right, top_left.y), Pos2::new(right, top_left.y + h)], st);
            painter.line_segment([Pos2::new(right - tick, top_left.y), Pos2::new(right, top_left.y)], st);
            painter.line_segment([Pos2::new(right - tick, top_left.y + h), Pos2::new(right, top_left.y + h)], st);
            let mut y = top_left.y + size * 0.11;
            for (i, row) in rows.iter().enumerate() {
                let mut x = left + tick + size * 0.10;
                for (j, cell) in row.iter().enumerate() {
                    let cm = measure(ui, cell, size * 0.94);
                    paint(ui, painter, cell, Pos2::new(x + (col_w[j] - cm.size.x) * 0.5, y), cm, size * 0.94, t);
                    x += col_w[j] + size * 0.42;
                }
                y += row_h[i] + size * 0.22;
            }
        }
        Node::BigOp { symbol, lower, upper, body } => {
            let sym = text_metrics(ui, symbol, Style::Op, size * 1.55);
            let lo = lower.as_ref().map(|n| measure(ui, n, size * 0.62));
            let up = upper.as_ref().map(|n| measure(ui, n, size * 0.62));
            let col = sym.size.x.max(lo.map(|m| m.size.x).unwrap_or(0.0)).max(up.map(|m| m.size.x).unwrap_or(0.0));
            let mut y = top_left.y;
            if let (Some(node), Some(mm)) = (upper, up) {
                paint(ui, painter, node, Pos2::new(top_left.x + (col - mm.size.x) * 0.5, y), mm, size * 0.62, t);
                y += mm.size.y;
            }
            painter.text(Pos2::new(top_left.x + (col - sym.size.x) * 0.5, y), Align2::LEFT_TOP, symbol.clone(), theme::math(size * 1.55), t.ink);
            y += sym.size.y;
            if let (Some(node), Some(mm)) = (lower, lo) {
                paint(ui, painter, node, Pos2::new(top_left.x + (col - mm.size.x) * 0.5, y), mm, size * 0.62, t);
            }
            let bm = measure(ui, body, size);
            paint(ui, painter, body, Pos2::new(top_left.x + col + size * 0.24, baseline - bm.ascent), bm, size, t);
        }
    }
}

pub fn show(ui: &mut Ui, node: &Node, size: f32) {
    let t = theme::current(ui.ctx());
    let m = measure(ui, node, size);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(m.size.x.max(4.0), m.size.y.max(size * 1.1)), Sense::hover());
    paint(ui, &ui.painter_at(rect), node, rect.left_top(), m, size, &t);
}

pub fn show_expr(ui: &mut Ui, expr: &Expr, size: f32) {
    show(ui, &expr_node(expr), size);
}

pub fn show_definition(ui: &mut Ui, stmt: &Statement, value: Option<&Value>, echo: bool, size: f32) {
    show(ui, &definition_node(stmt, value, echo), size);
}

pub fn show_value(ui: &mut Ui, value: &Value, size: f32) {
    if let Some(node) = value_node(value) {
        show(ui, &node, size);
    }
}
