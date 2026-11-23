//! LaTeX ⇄ AST. MathLive is the editor; this keeps the canvas and the Python
//! pane describing exactly the same mathematics.
use crate::math::{fmt_num, Node, Stmt, FUNCS};

#[derive(Clone, Debug, PartialEq)]
enum T { Cmd(String), Num(f64), Id(String), Sym(char), LBrace, RBrace, LBrack, RBrack }

fn lex(s: &str) -> Result<Vec<T>, String> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() { i += 1; continue; }
        match ch {
            '\\' => {
                i += 1;
                if i >= c.len() { return Err("trailing \\".into()); }
                let mut n = String::new();
                if c[i].is_alphabetic() { while i < c.len() && c[i].is_alphabetic() { n.push(c[i]); i += 1; } }
                else { n.push(c[i]); i += 1; }
                if matches!(n.as_str(), "," | ";" | ":" | "!" | " " | "quad" | "qquad" | "left" | "right"
                    | "displaystyle" | "textstyle" | "limits" | "nolimits" | "big" | "Big" | "bigg" | "Bigg") { continue; }
                out.push(T::Cmd(n));
            }
            '{' => { out.push(T::LBrace); i += 1; }
            '}' => { out.push(T::RBrace); i += 1; }
            '[' => { out.push(T::LBrack); i += 1; }
            ']' => { out.push(T::RBrack); i += 1; }
            '0'..='9' | '.' => {
                let mut t = String::new();
                while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') { t.push(c[i]); i += 1; }
                out.push(T::Num(t.parse().map_err(|_| "bad number".to_string())?));
            }
            _ if ch.is_alphabetic() => {
                let mut t = String::new();
                while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') { t.push(c[i]); i += 1; }
                out.push(T::Id(t));
            }
            other => { out.push(T::Sym(other)); i += 1; }
        }
    }
    Ok(out)
}

fn greek(n: &str) -> Option<&'static str> {
    Some(match n {
        "alpha"=>"alpha","beta"=>"beta","gamma"=>"gamma","delta"=>"delta","epsilon"=>"epsilon",
        "theta"=>"theta","lambda"=>"lambda","mu"=>"mu","nu"=>"nu","xi"=>"xi","pi"=>"pi",
        "rho"=>"rho","sigma"=>"sigma","tau"=>"tau","phi"=>"phi","chi"=>"chi","psi"=>"psi",
        "omega"=>"omega","Gamma"=>"Gamma","Delta"=>"Delta","Theta"=>"Theta","Sigma"=>"Sigma",
        "Phi"=>"Phi","Omega"=>"Omega","infty"=>"inf", _ => return None,
    })
}

fn fname(n: &str) -> Option<&'static str> {
    Some(match n {
        "sin"=>"sin","cos"=>"cos","tan"=>"tan","arcsin"=>"asin","arccos"=>"acos","arctan"=>"atan",
        "sinh"=>"sinh","cosh"=>"cosh","tanh"=>"tanh","ln"=>"ln","log"=>"log10","exp"=>"exp",
        "sec"=>"sec","csc"=>"csc","cot"=>"cot", _ => return None,
    })
}

struct L { t: Vec<T>, i: usize }

impl L {
    fn peek(&self) -> Option<&T> { self.t.get(self.i) }
    fn group(&mut self) -> Result<Vec<T>, String> {
        if self.peek() == Some(&T::LBrace) {
            self.i += 1; let start = self.i; let mut d = 1;
            while self.i < self.t.len() {
                match self.t[self.i] { T::LBrace => d += 1, T::RBrace => { d -= 1; if d == 0 { let g = self.t[start..self.i].to_vec(); self.i += 1; return Ok(g); } } _ => {} }
                self.i += 1;
            }
            return Err("unbalanced {".into());
        }
        match self.t.get(self.i).cloned() { Some(t) => { self.i += 1; Ok(vec![t]) } None => Err("expected argument".into()) }
    }
    fn sub(&mut self, g: Vec<T>) -> Result<Node, String> {
        let mut p = L { t: g, i: 0 }; let e = p.expr(0)?; Ok(e)
    }
    fn arg(&mut self) -> Result<Node, String> { let g = self.group()?; self.sub(g) }
    fn text(&mut self) -> Result<String, String> {
        let g = self.group()?; let mut s = String::new();
        for t in g { match t { T::Id(x) => s.push_str(&x), T::Num(v) => s.push_str(&fmt_num(v)),
                              T::Cmd(x) => s.push_str(greek(&x).unwrap_or(&x)), T::Sym(c) => s.push(c), _ => {} } }
        Ok(s)
    }
    fn limits(&mut self) -> Result<(Option<Node>, Option<Node>), String> {
        let (mut lo, mut hi) = (None, None);
        loop {
            match self.peek() {
                Some(T::Sym('_')) => { self.i += 1; lo = Some(self.arg()?); }
                Some(T::Sym('^')) => { self.i += 1; hi = Some(self.arg()?); }
                _ => break,
            }
        }
        Ok((lo, hi))
    }
    /// Find the trailing differential of an integral.
    /// Returns (body_end, after_differential, variable).
    /// Scans from the END so a body variable like `density` is never mistaken
    /// for `d` + `ensity`, and handles both `dx` and `d x` / `d\rho`.
    fn differential(&self, from: usize) -> Option<(usize, usize, String)> {
        let mut d = 0i32;
        let mut end = from;
        while end < self.t.len() {
            match &self.t[end] {
                T::LBrace | T::LBrack => d += 1,
                T::RBrace | T::RBrack => { if d == 0 { break; } d -= 1; }
                T::Sym(')') => { if d == 0 { break; } }
                T::Cmd(c) if c == "end" => break,
                _ => {}
            }
            end += 1;
        }
        if end <= from { return None; }
        let last = &self.t[end - 1];
        // `dx` written as one identifier.
        if let T::Id(s) = last {
            if s.len() > 1 && s.starts_with('d') {
                return Some((end - 1, end, s[1..].to_string()));
            }
        }
        if end < from + 2 { return None; }
        // `d x` or `d\rho` written as two tokens.
        if let T::Id(dd) = &self.t[end - 2] {
            if dd == "d" {
                let v = match last {
                    T::Id(v) => v.clone(),
                    T::Cmd(c) => greek(c).unwrap_or(c).to_string(),
                    _ => return None,
                };
                return Some((end - 2, end, v));
            }
        }
        None
    }
    fn starts_atom(&self) -> bool {
        matches!(self.peek(), Some(T::Num(_)) | Some(T::Id(_)) | Some(T::LBrace) | Some(T::Sym('(')) | Some(T::Cmd(_)))
    }
    fn binop(&self) -> Option<(&'static str, u8)> {
        match self.peek() {
            Some(T::Sym('=')) => Some(("=", 0)),
            Some(T::Sym('+')) => Some(("+", 1)),
            Some(T::Sym('-')) => Some(("-", 1)),
            Some(T::Sym('*')) => Some(("*", 2)),
            Some(T::Sym('/')) => Some(("/", 2)),
            Some(T::Cmd(c)) => match c.as_str() {
                "cdot" | "times" => Some(("*", 2)), "div" => Some(("/", 2)), _ => None },
            _ => None,
        }
    }
    fn expr(&mut self, min: u8) -> Result<Node, String> {
        let mut lhs = self.postfix()?;
        loop {
            if let Some((op, p)) = self.binop() {
                if p < min { break; }
                self.i += 1;
                let rhs = self.expr(p + 1)?;
                lhs = Node::Bin(op, Box::new(lhs), Box::new(rhs));
                continue;
            }
            if min <= 2 && self.starts_atom() {
                let rhs = self.expr(3)?;
                lhs = Node::Bin("*", Box::new(lhs), Box::new(rhs));
                continue;
            }
            break;
        }
        Ok(lhs)
    }
    fn postfix(&mut self) -> Result<Node, String> {
        let mut b = self.atom()?;
        loop {
            match self.peek() {
                Some(T::Sym('^')) => { self.i += 1; let s = self.arg()?; b = Node::Bin("^", Box::new(b), Box::new(s)); }
                Some(T::Sym('_')) => {
                    self.i += 1; let s = self.arg()?;
                    let nm = match (&b, &s) {
                        (Node::Var(x), Node::Var(y)) => Some(format!("{x}_{y}")),
                        (Node::Var(x), Node::Num(v)) => Some(format!("{x}_{}", *v as i64)),
                        _ => None,
                    };
                    b = match nm { Some(n) => Node::Var(n), None => Node::Call("index".into(), vec![b, s]) };
                }
                _ => break,
            }
        }
        Ok(b)
    }
    fn atom(&mut self) -> Result<Node, String> {
        let t = self.t.get(self.i).cloned().ok_or("unexpected end")?;
        self.i += 1;
        match t {
            T::Num(v) => Ok(Node::Num(v)),
            T::Id(s) => Ok(Node::Var(s)),
            T::LBrace => { self.i -= 1; let g = self.group()?; self.sub(g) }
            T::Sym('(') => { let e = self.expr(0)?; if self.peek() != Some(&T::Sym(')')) { return Err("expected )".into()); } self.i += 1; Ok(e) }
            T::Sym('-') => Ok(Node::Neg(Box::new(self.expr(3)?))),
            T::Sym('+') => self.expr(3),
            T::Sym('|') => { let e = self.expr(0)?; if self.peek() == Some(&T::Sym('|')) { self.i += 1; } Ok(Node::Call("abs".into(), vec![e])) }
            T::Cmd(c) => self.cmd(c),
            other => Err(format!("unexpected {other:?}")),
        }
    }
    fn cmd(&mut self, c: String) -> Result<Node, String> {
        if let Some(g) = greek(&c) { return Ok(Node::Var(g.into())); }
        if let Some(f) = fname(&c) { let a = self.expr(3)?; return Ok(Node::Call(f.into(), vec![a])); }
        match c.as_str() {
            "frac" | "dfrac" | "tfrac" => {
                let ng = self.group()?; let n = self.sub(ng)?;
                let dg = self.group()?; let d = self.sub(dg)?;
                Ok(Node::Bin("/", Box::new(n), Box::new(d)))
            }
            "sqrt" => {
                if self.peek() == Some(&T::LBrack) {
                    self.i += 1; let s = self.i;
                    while self.i < self.t.len() && self.t[self.i] != T::RBrack { self.i += 1; }
                    let deg = self.sub(self.t[s..self.i].to_vec())?; self.i += 1;
                    let bg = self.group()?; let body = self.sub(bg)?;
                    Ok(Node::Call("nroot".into(), vec![body, deg]))
                } else { let g = self.group()?; let b = self.sub(g)?; Ok(Node::Call("sqrt".into(), vec![b])) }
            }
            "int" => {
                let (lo, hi) = self.limits()?;
                let start = self.i;
                let (body, var) = match self.differential(start) {
                    Some((bend, after, v)) => {
                        let g = self.t[start..bend].to_vec();
                        self.i = after;
                        (self.sub(g)?, v)
                    }
                    None => (self.expr(1)?, "x".to_string()),
                };
                Ok(Node::Int { body: Box::new(body), var,
                    lo: Box::new(lo.unwrap_or(Node::Num(0.0))), hi: Box::new(hi.unwrap_or(Node::Num(1.0))) })
            }
            "sum" | "prod" => {
                let (lo, hi) = self.limits()?;
                let (start, var) = match lo {
                    Some(Node::Bin("=", a, b)) =>
                        (*b, match *a { Node::Var(v) => v, _ => "k".to_string() }),
                    Some(other) => (other, "k".to_string()),
                    None => (Node::Num(1.0), "k".to_string()),
                };
                let body = self.expr(1)?;
                Ok(Node::Sum { body: Box::new(body), var, lo: Box::new(start),
                    hi: Box::new(hi.unwrap_or(Node::Var("n".into()))), prod: c == "prod" })
            }
            "mathrm" | "text" | "operatorname" | "mathbf" => {
                let n = self.text()?;
                if FUNCS.contains(&n.as_str()) { let a = self.expr(3)?; Ok(Node::Call(n, vec![a])) }
                else { Ok(Node::Var(n)) }
            }
            "abs" => { let g = self.group()?; let b = self.sub(g)?; Ok(Node::Call("abs".into(), vec![b])) }
            "placeholder" => Err("incomplete".into()),
            other => Ok(Node::Var(other.into())),
        }
    }
}

/// LaTeX from a math field -> statement. `incomplete` means an empty placeholder.
pub fn parse_latex(src: &str) -> Result<Option<Stmt>, String> {
    let s = src.trim();
    if s.is_empty() { return Ok(None); }
    if s.contains("\\placeholder") { return Err("incomplete".into()); }
    let body = s.trim_end().trim_end_matches('=').trim();
    if body.is_empty() { return Ok(None); }

    let (name, rest) = split_def(body);
    let t = lex(rest)?;
    if t.is_empty() { return Ok(None); }
    let mut p = L { t, i: 0 };
    let expr = p.expr(0)?;
    Ok(Some(Stmt { name, expr }))
}

fn split_def(s: &str) -> (Option<String>, &str) {
    let mut d = 0i32;
    for (i, ch) in s.char_indices() {
        match ch { '{' | '(' | '[' => d += 1, '}' | ')' | ']' => d -= 1, _ => {} }
        if d == 0 {
            if s[i..].starts_with(":=") { return (Some(s[..i].trim().to_string()), &s[i + 2..]); }
            if s[i..].starts_with("\\coloneqq") { return (Some(s[..i].trim().to_string()), &s[i + 9..]); }
        }
    }
    (None, s)
}

// ───────────────────────── LaTeX renderer ─────────────────────────
fn lx(n: &Node, parent: u8) -> String {
    match n {
        Node::Num(v) => fmt_num(*v),
        Node::Var(s) => {
            if let Some((a, b)) = s.split_once('_') { return format!("{}_{{{}}}", greek_inv(a), b); }
            greek_inv(s).to_string()
        }
        Node::Neg(a) => format!("-{}", lx(a, 3)),
        Node::Bin("/", a, b) => format!("\\frac{{{}}}{{{}}}", lx(a, 0), lx(b, 0)),
        Node::Bin("^", a, b) => format!("{}^{{{}}}", lx(a, 5), lx(b, 0)),
        Node::Bin(op, a, b) => {
            let p = if *op == "+" || *op == "-" { 1 } else { 2 };
            let sym = match *op { "*" => "\\cdot ", "-" => "-", "+" => "+", _ => op };
            let s = format!("{} {} {}", lx(a, p), sym, lx(b, p + 1));
            if p < parent { format!("\\left({s}\\right)") } else { s }
        }
        Node::Call(f, args) => {
            if f == "sqrt" && args.len() == 1 { return format!("\\sqrt{{{}}}", lx(&args[0], 0)); }
            if f == "nroot" && args.len() == 2 { return format!("\\sqrt[{}]{{{}}}", lx(&args[1], 0), lx(&args[0], 0)); }
            if f == "abs" && args.len() == 1 { return format!("\\left|{}\\right|", lx(&args[0], 0)); }
            let a: Vec<String> = args.iter().map(|x| lx(x, 0)).collect();
            let head = if FUNCS.contains(&f.as_str()) { format!("\\{f}") } else { format!("\\mathrm{{{f}}}") };
            format!("{head}\\left({}\\right)", a.join(", "))
        }
        Node::Int { body, var, lo, hi } =>
            format!("\\int_{{{}}}^{{{}}}{}\\,d{}", lx(lo, 0), lx(hi, 0), lx(body, 1), var),
        Node::Sum { body, var, lo, hi, prod } =>
            format!("\\{}_{{{}={}}}^{{{}}}{}", if *prod { "prod" } else { "sum" },
                    var, lx(lo, 0), lx(hi, 0), lx(body, 1)),
    }
}

fn greek_inv(n: &str) -> &str {
    match n {
        "alpha"=>"\u{03B1}","beta"=>"\u{03B2}","gamma"=>"\u{03B3}","delta"=>"\u{03B4}",
        "theta"=>"\u{03B8}","lambda"=>"\u{03BB}","mu"=>"\u{03BC}","pi"=>"\u{03C0}",
        "sigma"=>"\u{03C3}","phi"=>"\u{03C6}","omega"=>"\u{03C9}","inf"=>"\u{221E}",
        _ => n,
    }
}

pub fn to_latex(s: &Stmt) -> String {
    match &s.name {
        Some(n) => format!("{}:= {}", greek_inv(n), lx(&s.expr, 0)),
        None => lx(&s.expr, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{eval, Env};
    #[test] fn fraction_roundtrip() {
        let s = parse_latex("\\frac{1}{2}").unwrap().unwrap();
        assert_eq!(eval(&s.expr, &mut Env::new()).unwrap(), 0.5);
    }
    #[test] fn integral_roundtrip() {
        let s = parse_latex("\\int_{0}^{1}x^{2}\\,dx").unwrap().unwrap();
        assert!((eval(&s.expr, &mut Env::new()).unwrap() - 1.0 / 3.0).abs() < 1e-9);
    }
    #[test] fn definition() {
        let s = parse_latex("x:= 3").unwrap().unwrap();
        assert_eq!(s.name.as_deref(), Some("x"));
    }
    #[test] fn sum_roundtrip() {
        let s = parse_latex("\\sum_{k=1}^{10}k^{2}").unwrap().unwrap();
        assert_eq!(eval(&s.expr, &mut Env::new()).unwrap(), 385.0);
    }
    #[test] fn latex_survives_python() {
        let s = crate::math::parse_python("integrate(lambda x: x**2, 0, 1)").unwrap().unwrap();
        let back = parse_latex(&to_latex(&s)).unwrap().unwrap();
        assert_eq!(s.expr, back.expr);
    }
    #[test] fn placeholder_incomplete() {
        assert!(parse_latex("\\frac{\\placeholder{}}{2}").is_err());
    }
}
