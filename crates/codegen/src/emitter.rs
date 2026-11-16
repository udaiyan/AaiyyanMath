use crate::{expr_type, Options, Ty};
use aaiyyan_ast::{BinaryOp, Expr, UnaryOp};
use aaiyyan_units::is_unit_name;
use std::collections::{BTreeMap, BTreeSet};

const PYTHON_KEYWORDS: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
    "None", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
];

pub struct Emitter {
    pub opts: Options,
    pub types: BTreeMap<String, Ty>,
    pub libraries: BTreeSet<String>,
    pub free_symbols: BTreeSet<String>,
    defined: BTreeSet<String>,
}

impl Emitter {
    pub fn new(opts: Options, types: BTreeMap<String, Ty>) -> Emitter {
        let defined: BTreeSet<String> = types.keys().cloned().collect();
        Emitter { opts, types, libraries: BTreeSet::new(), free_symbols: BTreeSet::new(), defined }
    }

    pub fn name(&self, raw: &str) -> String {
        if PYTHON_KEYWORDS.contains(&raw) { format!("{raw}_") } else { raw.to_string() }
    }

    fn ty(&self, e: &Expr) -> Ty { expr_type(e, &self.types) }

    fn unit_symbol(&mut self, name: &str) -> String {
        self.libraries.insert("pint".into());
        let mapped = match name {
            "\u{03A9}" => "ohm",
            other => other,
        };
        format!("ureg.{}", mapped.replace('\u{00B5}', "u").replace('\u{03BC}', "u"))
    }

    pub fn expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Number(v) => format_number_py(*v),
            Expr::Imaginary => "1j".into(),
            Expr::Bool(b) => if *b { "True".into() } else { "False".into() },
            Expr::Str(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            Expr::Symbol(s) => {
                match s.as_str() {
                    "pi" | "\u{03C0}" => { self.libraries.insert("math".into()); return "math.pi".into(); }
                    "e" => { self.libraries.insert("math".into()); return "math.e".into(); }
                    "inf" => { self.libraries.insert("math".into()); return "math.inf".into(); }
                    "i" => return "1j".into(),
                    _ => {}
                }
                if self.opts.use_units && is_unit_name(s) && !self.defined.contains(s) {
                    return self.unit_symbol(s);
                }
                let n = self.name(s);
                if !self.defined.contains(s) {
                    self.free_symbols.insert(n.clone());
                }
                n
            }
            Expr::Unary { op, expr } => {
                let inner = self.wrap(expr, 16);
                match op {
                    UnaryOp::Neg => format!("-{inner}"),
                    UnaryOp::Pos => format!("+{inner}"),
                    UnaryOp::Not => format!("not {inner}"),
                    UnaryOp::Factorial => {
                        self.libraries.insert("math".into());
                        format!("math.factorial({})", self.expr(expr))
                    }
                    UnaryOp::Transpose => {
                        self.libraries.insert("numpy".into());
                        format!("np.transpose({})", self.expr(expr))
                    }
                }
            }
            Expr::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs),
            Expr::Call { name, args } => self.call(name, args),
            Expr::Matrix(rows) => {
                self.libraries.insert("numpy".into());
                let rs: Vec<String> = rows.iter()
                    .map(|r| format!("[{}]", r.iter().map(|c| self.expr(c)).collect::<Vec<_>>().join(", ")))
                    .collect();
                format!("np.array([{}], dtype=float)", rs.join(", "))
            }
            Expr::Vector(items) => {
                self.libraries.insert("numpy".into());
                let inner: Vec<String> = items.iter().map(|c| self.expr(c)).collect();
                format!("np.array([{}], dtype=float)", inner.join(", "))
            }
            Expr::Range { start, step, end } => {
                let a = self.expr(start);
                let b = self.expr(end);
                let s = step.as_ref().map(|x| self.expr(x)).unwrap_or_else(|| "1.0".into());
                format!("aaiyyan.math_range({a}, {b}, {s})")
            }
            Expr::Index { target, indices } => {
                let t = self.wrap(target, 20);
                let idx: Vec<String> = indices.iter().map(|i| self.expr(i)).collect();
                format!("{t}[{}]", idx.join(", "))
            }
            Expr::Conditional { cond, then, otherwise } => {
                format!("({} if {} else {})", self.expr(then), self.expr(cond), self.expr(otherwise))
            }
        }
    }

    fn binary(&mut self, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> String {
        let p = op.precedence();
        let lt = self.ty(lhs);
        let rt = self.ty(rhs);
        let l = self.wrap(lhs, p);
        let r = self.wrap(rhs, p + 1);
        match op {
            BinaryOp::Mul => {
                if (lt == Ty::Matrix && rt == Ty::Matrix)
                    || (lt == Ty::Matrix && rt == Ty::Vector)
                    || (lt == Ty::Vector && rt == Ty::Matrix)
                {
                    return format!("{l} @ {r}");
                }
                if lt == Ty::Vector && rt == Ty::Vector {
                    self.libraries.insert("numpy".into());
                    return format!("np.dot({l}, {r})");
                }
                format!("{l} * {r}")
            }
            BinaryOp::Pow | BinaryOp::ElemPow => format!("{l}**{r}"),
            BinaryOp::Div | BinaryOp::ElemDiv => format!("{l} / {r}"),
            BinaryOp::Add => format!("{l} + {r}"),
            BinaryOp::Sub => format!("{l} - {r}"),
            BinaryOp::Mod => format!("{l} % {r}"),
            BinaryOp::ElemMul => format!("{l} * {r}"),
            BinaryOp::And => format!("{l} and {r}"),
            BinaryOp::Or => format!("{l} or {r}"),
            BinaryOp::Eq => format!("{l} == {r}"),
            BinaryOp::Ne => format!("{l} != {r}"),
            BinaryOp::Lt => format!("{l} < {r}"),
            BinaryOp::Le => format!("{l} <= {r}"),
            BinaryOp::Gt => format!("{l} > {r}"),
            BinaryOp::Ge => format!("{l} >= {r}"),
        }
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> String {
        let a: Vec<String> = args.iter().map(|x| self.expr(x)).collect();
        let array = args.iter().any(|x| self.ty(x).is_array());
        let join = a.join(", ");
        let first = a.first().cloned().unwrap_or_default();
        match name {
            "sqrt" | "exp" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh"
            | "tanh" | "floor" | "ceil" | "log10" | "log2" | "atan2" | "hypot" => {
                if array {
                    self.libraries.insert("numpy".into());
                    format!("np.{name}({join})")
                } else {
                    self.libraries.insert("math".into());
                    format!("math.{name}({join})")
                }
            }
            "abs" => format!("abs({join})"),
            "ln" => {
                if array {
                    self.libraries.insert("numpy".into());
                    format!("np.log({join})")
                } else {
                    self.libraries.insert("math".into());
                    format!("math.log({join})")
                }
            }
            "log" => {
                self.libraries.insert("math".into());
                if a.len() == 2 { format!("math.log({}, {})", a[0], a[1]) } else { format!("math.log({join})") }
            }
            "convert" => format!("({first}).to({})", a.get(1).cloned().unwrap_or_default()),
            "si" => format!("({first}).to_base_units().magnitude"),
            "strip_units" => format!("({first}).magnitude"),
            "unit_of" => format!("str(({first}).units)"),
            "det" | "inv" | "eigenvals" | "eigenvectors" | "rank" | "solve" | "norm" => {
                self.libraries.insert("numpy".into());
                let f = match name {
                    "det" => "np.linalg.det",
                    "inv" => "np.linalg.inv",
                    "eigenvals" => "np.linalg.eigvals",
                    "eigenvectors" => "np.linalg.eig",
                    "rank" => "np.linalg.matrix_rank",
                    "solve" => "np.linalg.solve",
                    _ => "np.linalg.norm",
                };
                format!("{f}({join})")
            }
            "transpose" | "dot" | "cross" | "diag" | "linspace" | "mean" | "std" | "var" | "median"
            | "sum" | "prod" | "sort" | "polyfit" | "polyval" => {
                self.libraries.insert("numpy".into());
                format!("np.{name}({join})")
            }
            "interp1" => { self.libraries.insert("numpy".into()); format!("np.interp({join})") }
            "zeros" | "ones" => { self.libraries.insert("numpy".into()); format!("np.{name}({join})") }
            "identity" | "eye" => { self.libraries.insert("numpy".into()); format!("np.eye({join})") }
            "range" | "arange" => format!("aaiyyan.math_range({join})"),
            "len" => format!("len({join})"),
            "rows" => { self.libraries.insert("numpy".into()); format!("np.shape({first})[0]") }
            "cols" => { self.libraries.insert("numpy".into()); format!("np.shape({first})[1]") }
            "min" | "max" => format!("{name}({join})"),
            "if" => format!(
                "({} if {} else {})",
                a.get(1).cloned().unwrap_or_default(), first, a.get(2).cloned().unwrap_or_default()
            ),
            "diff" | "simplify" | "expand" | "factor" | "series" | "limit" => {
                self.libraries.insert("sympy".into());
                format!("sp.{name}({join})")
            }
            "solve_eq" => { self.libraries.insert("sympy".into()); format!("sp.solve({join})") }
            "dsolve" => { self.libraries.insert("sympy".into()); format!("sp.dsolve({join})") }
            "integrate" => {
                if args.len() >= 3 {
                    self.libraries.insert("scipy".into());
                    format!("scipy.integrate.quad({}, {}, {})[0]", a[0], a[1], a[2])
                } else {
                    self.libraries.insert("sympy".into());
                    format!("sp.integrate({join})")
                }
            }
            "nderiv" => format!("aaiyyan.numeric_derivative({join})"),
            "root" => { self.libraries.insert("scipy".into()); format!("scipy.optimize.brentq({join})") }
            "minimize" => {
                self.libraries.insert("scipy".into());
                format!(
                    "scipy.optimize.minimize_scalar({}, bounds=({}, {}), method=\"bounded\").x",
                    first, a[1], a[2]
                )
            }
            "odesolve" => format!("aaiyyan.ode_solve({join})"),
            "bvp_solve" => format!("aaiyyan.bvp_solve({join})"),
            "curve_fit" => format!("aaiyyan.fit({join})"),
            "read_csv" => { self.libraries.insert("pandas".into()); format!("pd.read_csv({join})") }
            "read_excel" => { self.libraries.insert("pandas".into()); format!("pd.read_excel({join})") }
            "column" => format!("({first})[{}].to_numpy()", a.get(1).cloned().unwrap_or_default()),
            "table_matrix" => format!("({first}).to_numpy()"),
            "slider" | "input_number" | "checkbox" | "dropdown" => first,
            "print" => format!("print({join})"),
            other => {
                if crate::NATIVE_HELPERS.contains(&other) {
                    format!("aaiyyan.{other}({join})")
                } else {
                    let n = self.name(other);
                    format!("{n}({join})")
                }
            }
        }
    }

    fn wrap(&mut self, expr: &Expr, parent_prec: u8) -> String {
        let s = self.expr(expr);
        if expr.precedence() < parent_prec { format!("({s})") } else { s }
    }
}

fn format_number_py(v: f64) -> String {
    if v.is_finite() && v.fract().abs() < 1e-12 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}
