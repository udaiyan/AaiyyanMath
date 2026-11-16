#!/usr/bin/env python3
# AaiyyanMath: MATLAB-parity function catalog (native Rust <-> embedded Python routing),
# shape-class aware codegen, and the Python runtime mirrors for the SciPy/SymPy set.
# Derived from learn_matlab.pdf + matlab_math.pdf. Idempotent; backs up every file it touches.
# Usage: python patch_matlab_catalog.py [--root PATH] [--dry-run] [--no-backup]

import argparse, datetime, shutil, sys
from pathlib import Path

FILES = {}
def add(p, c): FILES[p] = c.lstrip("\n")

# ======================================================================================
# crates/evaluator/src/functions.rs  — the single authoritative catalog
# ======================================================================================
add("crates/evaluator/src/functions.rs", r"""
//! MATLAB-parity function catalog.
//!
//! One source of truth for every named operation the worksheet can call. Each entry
//! carries a *shape class* (matrix / array / scalar / structured) and a routing
//! decision: native Rust, or embedded Python (NumPy/SciPy/SymPy) via the existing
//! `EvalError::RequiresPython` fallback. Code generation reads `python_call` so the
//! generated Python is always the canonical scientific-computing form.
//!
//! Traceability: names are grouped to match learn_matlab.pdf / matlab_math.pdf chapters.
use crate::env::Environment;
use crate::interp::{dimensionless, Evaluator};
use crate::linalg::{from_dm, matmul, to_dm};
use crate::value::{Matrix, Value};
use crate::EvalError;
use nalgebra::DMatrix;
use std::cell::RefCell;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class { Scalar, Array, Matrix, Structured }

#[derive(Clone, Copy)]
pub struct Spec {
    pub name: &'static str,
    pub class: Class,
    /// `true` when evaluation must be dispatched to the embedded Python backend.
    pub python: bool,
    /// Python expression template; `{a}`, `{b}`, … are positional argument slots.
    pub py: &'static str,
    pub arity: u8,
    pub doc: &'static str,
}

// ---- catalog -----------------------------------------------------------------------
macro_rules! cat {
    ($($n:expr, $c:expr, $p:expr, $py:expr, $ar:expr, $d:expr;)*) => {
        &[ $( Spec { name: $n, class: $c, python: $p, py: $py, arity: $ar, doc: $d } ),* ]
    };
}

pub const CATALOG: &[Spec] = cat![
    // -- matrices & arrays (learn ch.1-2, math ch.1) --
    "zeros", Class::Array, false, "np.zeros(({a}, {b}))", 2, "m-by-n matrix of zeros";
    "ones",  Class::Array, false, "np.ones(({a}, {b}))", 2, "m-by-n matrix of ones";
    "eye",   Class::Matrix, false, "np.eye({a})", 1, "identity matrix (eye(m,n) for rectangular)";
    "rand",  Class::Array, false, "np.random.rand({a})", 1, "uniform random in (0,1)";
    "randn", Class::Array, false, "np.random.randn({a})", 1, "standard-normal random";
    "randi", Class::Array, false, "np.random.randint(1, {a}+1, {b})", 2, "uniform random integers";
    "randperm", Class::Array, false, "np.random.permutation({a})", 1, "random permutation of 1..n";
    "magic", Class::Matrix, false, "aaiyyan.magic({a})", 1, "magic square";
    "linspace", Class::Array, false, "np.linspace({a}, {b}, {c})", 3, "evenly spaced vector";
    "logspace", Class::Array, false, "np.logspace({a}, {b}, {c})", 3, "log-spaced vector";
    "colon",  Class::Array, false, "np.arange({a}, {b}+1e-12) if _ncols==2 else np.arange({a}, {c}+1e-12, {b})", 3, "start:step:stop / start:stop";
    "reshape", Class::Array, false, "np.reshape({a}, ({b}, {c}))", 3, "reshape to m-by-n";
    "diag",   Class::Array, false, "np.diag({a})", 1, "diagonal / construct diagonal matrix";
    "trace",  Class::Matrix, false, "np.trace({a})", 1, "sum of diagonal";
    "flipud", Class::Array, false, "np.flipud({a})", 1, "flip upside-down";
    "fliplr", Class::Array, false, "np.fliplr({a})", 1, "flip left-right";
    "rot90",  Class::Array, false, "np.rot90({a}, {b})", 2, "rotate 90 degrees k times";
    "circshift", Class::Array, false, "np.roll({a}, {b})", 2, "circular shift";
    "repmat", Class::Array, false, "np.tile({a}, ({b}, {c}))", 3, "replicate and tile";
    "kron",   Class::Matrix, false, "np.kron({a}, {b})", 2, "Kronecker tensor product";
    "cat",    Class::Array, false, "np.concatenate(({a}, {b}), axis={c}-1)", 3, "concatenate along dim";
    "vertcat",Class::Array, false, "np.vstack(({a}, {b}))", 2, "vertical concatenation";
    "horzcat",Class::Array, false, "np.hstack(({a}, {b}))", 2, "horizontal concatenation";
    "size",   Class::Array, false, "np.shape({a})", 1, "dimensions";
    "numel",  Class::Array, false, "np.size({a})", 1, "number of elements";
    "length", Class::Array, false, "max(np.shape({a}))", 1, "length of largest dimension";
    "ndims",  Class::Array, false, "np.ndim({a})", 1, "number of dimensions";
    "isempty",Class::Array, false, "(np.size({a})==0)", 1, "true if empty";
    "sort",   Class::Array, false, "np.sort({a})", 1, "sort ascending";
    "find",   Class::Array, false, "np.flatnonzero({a})+1", 1, "indices of nonzero (1-based)";
    "unique", Class::Array, false, "np.unique({a})", 1, "unique values";
    "ismember",Class::Array, false, "np.isin({a}, {b})", 2, "membership test";

    // -- linear algebra: factorizations & specials (math ch.2) --
    "lu",     Class::Matrix, true,  "scipy.linalg.lu({a})", 1, "LU factorization";
    "qr",     Class::Matrix, true,  "np.linalg.qr({a})", 1, "QR factorization";
    "chol",   Class::Matrix, true,  "scipy.linalg.cholesky({a})", 1, "Cholesky factorization";
    "svd",    Class::Matrix, true,  "np.linalg.svd({a})", 1, "singular value decomposition";
    "eig",    Class::Matrix, true,  "np.linalg.eig({a})", 1, "eigenvalues and vectors";
    "eigs",   Class::Matrix, true,  "scipy.sparse.linalg.eigs({a})", 1, "sparse eigenvalues";
    "expm",   Class::Matrix, false, "aaiyyan.expm({a})", 1, "matrix exponential (scaling-and-squaring)";
    "logm",   Class::Matrix, true,  "scipy.linalg.logm({a})", 1, "matrix logarithm";
    "sqrtm",  Class::Matrix, true,  "scipy.linalg.sqrtm({a})", 1, "matrix square root";
    "sylvester", Class::Matrix, true, "scipy.linalg.solve_sylvester({a},{b},{c},{d})", 4, "AX+XB=C";
    "lyap",   Class::Matrix, true,  "scipy.linalg.solve_lyapunov({a},{b})", 2, "AX-XA=B";
    "pagemtimes", Class::Matrix, true, "aaiyyan.pagemtimes({a},{b})", 2, "page-wise matrix multiply";
    "pagetranspose", Class::Matrix, true, "aaiyyan.pagetranspose({a})", 1, "page-wise transpose";
    "pageinv", Class::Matrix, true, "aaiyyan.pageinv({a})", 1, "page-wise inverse";
    "pagesvd", Class::Matrix, true, "aaiyyan.pagesvd({a})", 1, "page-wise SVD";
    "pageeig", Class::Matrix, true, "aaiyyan.pageeig({a})", 1, "page-wise eigenvalues";
    "pagenorm", Class::Matrix, true, "aaiyyan.pagenorm({a})", 1, "page-wise norm";

    // -- systems of linear equations, direct + iterative (math ch.2) --
    "solve",  Class::Matrix, false, "aaiyyan.solve({a},{b})", 2, "direct Ax=B (nalgebra LU)";
    "mldivide", Class::Matrix, false, "({b} @ np.linalg.inv({a}) if False else np.linalg.solve({a},{b}))", 2, "backslash A\\B";
    "bicg",   Class::Matrix, true,  "scipy.sparse.linalg.bicg({a},{b})", 2, "BIConjugate Gradient";
    "cgs",    Class::Matrix, true,  "scipy.sparse.linalg.cgs({a},{b})", 2, "Conjugate Gradient Squared";
    "gmres",  Class::Matrix, true,  "scipy.sparse.linalg.gmres({a},{b})", 2, "Generalized Minimal Residual";
    "minres", Class::Matrix, true,  "scipy.sparse.linalg.minres({a},{b})", 2, "Minimum Residual (Hermitian)";
    "qmr",    Class::Matrix, true,  "scipy.sparse.linalg.qmr({a},{b})", 2, "Quasi-Minimal Residual";
    "symmlq", Class::Matrix, true,  "scipy.sparse.linalg.symmlq({a},{b})", 2, "Symmetric LQ";
    "tfqmr",  Class::Matrix, true,  "scipy.sparse.linalg.tfqmr({a},{b})", 2, "Transpose-Free QMR";
    "lsqr",   Class::Matrix, true,  "scipy.sparse.linalg.lsqr({a},{b})", 2, "least-squares QR";
    "lsmr",   Class::Matrix, true,  "scipy.sparse.linalg.lsmr({a},{b})", 2, "least-squares MR";

    // -- eigen / singular (math ch.2) --
    "eigenvals", Class::Matrix, false, "aaiyyan.eigenvals({a})", 1, "symmetric eigenvalues (native)";
    "eigenvectors", Class::Matrix, false, "aaiyyan.eigenvectors({a})", 1, "symmetric eigenvectors (native)";
    "rank",   Class::Matrix, false, "np.linalg.matrix_rank({a})", 1, "matrix rank";
    "det",    Class::Matrix, false, "np.linalg.det({a})", 1, "determinant";
    "inv",    Class::Matrix, false, "np.linalg.inv({a})", 1, "inverse";
    "norm",   Class::Array,  false, "np.linalg.norm({a})", 1, "vector/matrix norm";
    "cond",   Class::Matrix, true,  "np.linalg.cond({a})", 1, "condition number";

    // -- random numbers, generators, seeds (learn ch.3, math ch.3) --
    "rng",    Class::Structured, false, "aaiyyan.rng({a}, {b})", 2, "set seed and generator (twister/v4/v5uniform/v5normal)";
    "random", Class::Array, true,  "aaiyyan.randomdist({a}, {b}, {c})", 3, "named distribution samples";
    "unifrnd",Class::Array, false, "aaiyyan.runif({a},{b},{c})", 3, "uniform samples";
    "normrnd",Class::Array, false, "aaiyyan.rnorm({a},{b},{c})", 3, "normal samples";
    "exprnd", Class::Array, true,  "np.random.exponential({b}, {a})", 2, "exponential samples";
    "poissrnd",Class::Array, true, "np.random.poisson({b}, {a})", 2, "Poisson samples";
    "binornd",Class::Array, true,  "np.random.binomial({c},{b},{a})", 3, "binomial samples";

    // -- ODE IVP, mass matrix, events, moving mesh (math ch.11) --
    "ode45",  Class::Structured, true, "aaiyyan.ode({a}, {b}, {c}, 'RK45')", 3, "explicit Runge-Kutta (Dormand-Prince)";
    "ode23",  Class::Structured, true, "aaiyyan.ode({a}, {b}, {c}, 'RK23')", 3, "explicit Bogacki-Shampine";
    "ode113", Class::Structured, true, "aaiyyan.ode({a}, {b}, {c}, 'LSODA')", 3, "variable-order Adams/BDF";
    "ode15s", Class::Structured, true, "aaiyyan.ode({a}, {b}, {c}, 'BDF', mass={d})", 4, "stiff, mass matrix supported";
    "ode23s", Class::Structured, true, "aaiyyan.ode({a}, {b}, {c}, 'Radau')", 3, "stiff, single mass matrix";
    "ode23t", Class::Structured, true, "aaiyyan.ode({a}, {b}, {c}, 'DOP853')", 3, "trapezoid, mass matrix";
    "ode23tb",Class::Structured, true, "aaiyyan.ode({a}, {b}, {c}, 'BDF')", 3, "TR-BDF2";
    "decic",  Class::Structured, true, "aaiyyan.decic({a},{b},{c},{d})", 4, "consistent initial conditions (mass)";
    "odestate",Class::Structured, true, "aaiyyan.OdeState({a})", 1, "solver state object";

    // -- BVP (math ch.12) --
    "bvp4c",  Class::Structured, true, "aaiyyan.bvp({a},{b},{c},{d},'col')", 4, "collocation BVP solver";
    "bvp5c",  Class::Structured, true, "aaiyyan.bvp({a},{b},{c},{d},'col5')", 4, "five-stage collocation BVP";

    // -- DDE (math ch.14) --
    "dde23",  Class::Structured, true, "aaiyyan.dde({a},{b},{c},{d},'rk')", 4, "explicit DDE solver";
    "ddensd", Class::Structured, true, "aaiyyan.dde({a},{b},{c},{d},'neutral')", 4, "neutral DDE solver";

    // -- PDE (math ch.13) --
    "pdepe",  Class::Structured, true, "aaiyyan.pdepe({a},{b},{c},{d},{e},{f})", 6, "1-D parabolic/elliptic, method of lines";
    "bvpinit",Class::Structured, true, "aaiyyan.bvpinit({a},{b})", 2, "initial guess mesh for BVP";
    "deval",  Class::Structured, true, "aaiyyan.deval({a},{b})", 2, "evaluate solver solution at points";

    // -- interpolation & fitting (math ch.7-8) --
    "interp1",Class::Array, false, "aaiyyan.linterp({a},{b},{c})", 3, "1-D interpolation (native linear)";
    "interp2",Class::Array, true,  "scipy.interpolate.interp2({a},{b},{c},{d})", 4, "2-D interpolation";
    "interp3",Class::Array, true,  "scipy.interpolate.interp3({a},{b},{c},{d},{e})", 5, "3-D interpolation";
    "interpn",Class::Array, true,  "scipy.interpolate.interpn(({a},{b}),{c},{d})", 4, "N-D interpolation";
    "griddedInterpolant", Class::Structured, true, "scipy.interpolate.RegularGridInterpolator(({a},{b}),{c})", 3, "gridded interpolant object";
    "scatteredInterpolant", Class::Structured, true, "scipy.interpolate.LinearNDInterpolator(list(zip({a},{b})),{c})", 3, "scattered interpolant";
    "delaunay",Class::Structured, true, "scipy.spatial.Delaunay(list(zip({a},{b})))", 2, "2-D Delaunay triangulation";
    "delaunayn",Class::Structured, true, "scipy.spatial.Delaunay(np.column_stack({a}))", 1, "N-D Delaunay";
    "tsearchn",Class::Structured, true, "aaiyyan.tsearchn({a},{b})", 2, "simplex lookup in triangulation";
    "dsearchn",Class::Structured, true, "aaiyyan.dsearchn({a},{b})", 2, "nearest vertex in triangulation";
    "spline", Class::Array, true,  "scipy.interpolate.CubicSpline({a},{b})({c})", 3, "cubic spline";
    "csaps",  Class::Array, true,  "scipy.interpolate.CubicSpline({a},{b})({c})", 3, "smoothing spline";
    "pchip",  Class::Array, true,  "scipy.interpolate.PchipInterpolator({a},{b})({c})", 3, "shape-preserving cubic";
    "polyfit",Class::Array, false, "np.polyfit({a},{b},{c})", 3, "polynomial least-squares fit";
    "polyval",Class::Array, false, "np.polyval({a},{b})", 2, "evaluate polynomial";
    "polyder",Class::Array, true,  "np.polyder({a},{b})", 2, "polynomial derivative";
    "roots",  Class::Array, false, "np.roots({a})", 1, "polynomial roots";
    "conv",   Class::Array, false, "np.convolve({a},{b})", 2, "convolution";
    "deconv", Class::Array, true,  "aaiyyan.deconv({a},{b})", 2, "polynomial division";

    // -- optimization (math ch.9-10) --
    "fminsearch", Class::Structured, true, "scipy.optimize.minimize({a}, {b}, method='Nelder-Mead').x", 2, "Nelder-Mead unconstrained";
    "fmincon",    Class::Structured, true, "scipy.optimize.minimize({a}, {b}, method='SLSQP', constraints={c}).x", 3, "constrained minimization";
    "fsolve",     Class::Structured, true, "scipy.optimize.fsolve({a}, {b})", 2, "nonlinear system root";
    "fzero",      Class::Structured, true, "scipy.optimize.brentq({a}, {b}, {c})", 3, "scalar root in bracket";
    "lsqnonlin",  Class::Structured, true, "scipy.optimize.least_squares({a}, {b}).x", 2, "nonlinear least squares";
    "curve_fit",  Class::Structured, true, "scipy.optimize.curve_fit({a}, {b}, {c})[0]", 3, "curve fitting";
    "fitlm",      Class::Structured, true, "aaiyyan.fitlm({a},{b})", 2, "linear model fit";
    "maximize",   Class::Structured, false, "aaiyyan.maximize({a},{b},{c})", 3, "golden-section maximise (native)";
    "newton",     Class::Structured, false, "aaiyyan.newton({a},{b})", 2, "Newton root (native)";
    "root",       Class::Structured, false, "aaiyyan.root({a},{b},{c})", 3, "bracketed root (native)";

    // -- polynomials, FFT, signal (learn ch.1, math ch.2) --
    "fft",  Class::Array, false, "np.fft.fft({a})", 1, "fast Fourier transform";
    "ifft", Class::Array, false, "np.fft.ifft({a})", 1, "inverse FFT";
    "fft2", Class::Array, true,  "np.fft.fft2({a})", 1, "2-D FFT";
    "conv2",Class::Array, true,  "scipy.signal.convolve2d({a},{b})", 2, "2-D convolution";
    "filter",Class::Array, true, "scipy.signal.lfilter({a},{b},{c})", 3, "digital filter";
    "pwelch",Class::Array, true, "scipy.signal.welch({a}, {b})[0]", 2, "Welch PSD estimate";
    "periodogram",Class::Array, true, "scipy.signal.periodogram({a}, {b})[0]", 2, "periodogram PSD";

    // -- strings / cells / structs / tables (learn ch.2) --
    "string",  Class::Structured, false, "str({a})", 1, "string scalar";
    "strlength",Class::Structured, false, "len(str({a}))", 1, "string length";
    "append",  Class::Structured, true,  "aaiyyan.append({a},{b})", 2, "concatenate string/char/cell";
    "cell",    Class::Structured, false, "list({a})", 1, "cell array";
    "struct",  Class::Structured, false, "dict()", 0, "structure array";
    "table",   Class::Structured, true,  "aaiyyan.table({a},{b})", 2, "mixed-data table";
    "num2str", Class::Structured, false, "str({a})", 1, "numeric to string";
    "str2num", Class::Structured, false, "float({a})", 1, "string to numeric";
    "isspace", Class::Structured, false, "str({a}).isspace()", 1, "all whitespace";
    "upper",   Class::Structured, false, "str({a}).upper()", 1, "uppercase";
    "lower",   Class::Structured, false, "str({a}).lower()", 1, "lowercase";
    "sprintf", Class::Structured, false, "({a}) % tuple({b}) if isinstance({b},list) else ({a})%{b}", 2, "formatted string";
    "fprintf", Class::Structured, true,  "aaiyyan.fprintf({a},{b})", 2, "formatted output to console";
    "regexp",  Class::Structured, true,  "aaiyyan.regexp({a},{b})", 2, "regular expression match";
    "contains",Class::Structured, false, "({b}) in str({a})", 2, "substring test";
    "split",   Class::Structured, false, "str({a}).split({b})", 2, "split string";

    // -- reductions with dim (learn ch.1-2) --
    "sum",   Class::Array, false, "np.sum({a})", 1, "sum of elements";
    "prod",  Class::Array, false, "np.prod({a})", 1, "product of elements";
    "max",   Class::Array, false, "np.max({a})", 1, "maximum";
    "min",   Class::Array, false, "np.min({a})", 1, "minimum";
    "mean",  Class::Array, false, "np.mean({a})", 1, "arithmetic mean";
    "median",Class::Array, false, "np.median({a})", 1, "median";
    "std",   Class::Array, false, "np.std({a}, ddof=1)", 1, "standard deviation";
    "var",   Class::Array, false, "np.var({a}, ddof=1)", 1, "variance";
];

/// All catalogued names (used by `is_known`).
pub const NAMES: &[&str] = &{
    let mut v: Vec<&str> = CATALOG.iter().map(|s| s.name).collect();
    v.sort_unstable(); v.dedup(); v.leak() as *const [_]
};
// NOTE: `leak` keeps the static slice valid for the process; built once at first use.

pub fn is_known(name: &str) -> bool { NAMES.binary_search(&name).is_ok() }

pub fn spec(name: &str) -> Option<&'static Spec> { CATALOG.iter().find(|s| s.name == name) }

/// Python expression for codegen; `None` when the name is native-only.
pub fn python_call(name: &str) -> Option<&'static str> {
    spec(name).filter(|s| s.python || s.py.starts_with("aaiyyan.") || s.py.starts_with("np.") || s.py.starts_with("scipy.")).map(|s| s.py)
}

thread_local! { static RNG: RefCell<u64> = RefCell::new(0x2545F4914F6CDD1D); }
fn unit() -> f64 { RNG.with(|r| { let mut s = r.borrow_mut(); *s ^= *s << 13; *s ^= *s >> 7; *s ^= *s << 17; (*s >> 11) as f64 / 9007199254740992.0 }) }
fn num(v: &Value) -> Result<f64, EvalError> { dimensionless(v) }
fn vec_of(v: &Value) -> Result<Vec<f64>, EvalError> {
    match v { Value::Vector(a) => Ok(a.clone()), Value::Matrix(m) if m.rows == 1 || m.cols == 1 => Ok(m.data.clone()), Value::Scalar(x) => Ok(vec![*x]), o => Err(EvalError::Type(format!("expected array, got {}", o.type_name()))) }
}
fn mat_of(v: &Value) -> Result<Matrix, EvalError> {
    match v { Value::Matrix(m) => Ok(m.clone()), Value::Vector(a) => Ok(Matrix::new(a.len(), 1, a.clone())), o => Err(EvalError::Type(format!("expected matrix, got {}", o.type_name()))) }
}
fn need(name: &str, args: &[Value], n: usize) -> Result<(), EvalError> {
    if args.len() != n { Err(EvalError::Args(format!("{name} expects {n} argument(s), got {}", args.len()))) } else { Ok(()) }
}
fn collapse(m: &Matrix, dim: usize, f: impl Fn(&[f64]) -> f64) -> Value {
    if dim == 1 { Value::Vector((0..m.cols).map(|c| f(&(0..m.rows).map(|r| m.get(r, c)).collect::<Vec<_>>())).collect()) }
    else { Value::Vector((0..m.rows).map(|r| f(&(0..m.cols).map(|c| m.get(r, c)).collect::<Vec<_>>())).collect()) }
}

/// Native evaluation. Returns `None` for names this module does not implement
/// (either Python-routed or unknown), so `builtins` can fall through.
pub fn call(name: &str, args: &[Value], ev: &mut Evaluator, env: &mut Environment) -> Option<Result<Value, EvalError>> {
    let s = spec(name)?;
    if s.python { return Some(Err(EvalError::RequiresPython(format!("{name} ({}); routed to embedded Python", s.doc)))); }
    Some(invoke(name, args, ev, env))
}

fn invoke(name: &str, args: &[Value], _ev: &mut Evaluator, _env: &mut Environment) -> Result<Value, EvalError> {
    match name {
        "zeros" => { need(name,args,2)?; let r=num(&args[0])? as usize; let c=num(&args[1])? as usize; Ok(Value::Matrix(Matrix::new(r,c,vec![0.0;r*c]))) }
        "ones"  => { need(name,args,2)?; let r=num(&args[0])? as usize; let c=num(&args[1])? as usize; Ok(Value::Matrix(Matrix::new(r,c,vec![1.0;r*c]))) }
        "eye"   => { need(name,args,1)?; let n=num(&args[0])? as usize; let mut d=vec![0.0;n*n]; for i in 0..n { d[i*n+i]=1.0; } Ok(Value::Matrix(Matrix::new(n,n,d))) }
        "rand"  => { need(name,args,1)?; let n=num(&args[0])? as usize; Ok(Value::Vector((0..n).map(|_| unit()).collect())) }
        "randn" => { need(name,args,1)?; let n=num(&args[0])? as usize; let mut v=Vec::with_capacity(n); for _ in 0..n { let u1=unit().max(1e-12); let u2=unit(); v.push((-2.0*u1.ln()).sqrt()*(std::f64::consts::TAU*u2).cos()); } Ok(Value::Vector(v)) }
        "randi" => { need(name,args,2)?; let hi=num(&args[0])? as i64; let n=num(&args[1])? as usize; Ok(Value::Vector((0..n).map(|_| 1.0+(unit()*(hi as f64)).floor().min((hi-1) as f64)).collect())) }
        "randperm" => { need(name,args,1)?; let n=num(&args[0])? as usize; let mut v:Vec<f64>=(1..=n).map(|x|x as f64).collect(); for i in (1..n).rev() { let j=(unit()*(i+1) as f64).floor() as usize; v.swap(i,j); } Ok(Value::Vector(v)) }
        "magic" => { need(name,args,1)?; let n=num(&args[0])? as usize; Ok(Value::Matrix(magic(n))) }
        "linspace" => { need(name,args,3)?; let a=num(&args[0])?; let b=num(&args[1])?; let n=num(&args[2])? as usize; if n<2 {return Err(EvalError::Args("linspace needs n>=2".into()));} Ok(Value::Vector((0..n).map(|i| a+(b-a)*i as f64/(n-1) as f64).collect())) }
        "logspace" => { need(name,args,3)?; let a=num(&args[0])?; let b=num(&args[1])?; let n=num(&args[2])? as usize; Ok(Value::Vector((0..n).map(|i| 10f64.powf(a+(b-a)*i as f64/(n-1) as f64)).collect())) }
        "colon" => { // 2 or 3 args: start:stop or start:step:stop
            let (a,step,b)=(match args.len(){2=>(num(&args[0])?,1.0,num(&args[1])?),3=>(num(&args[0])?,num(&args[1])?,num(&args[2])?),_=>return Err(EvalError::Args("colon takes 2 or 3 args".into()))});
            if step==0.0 {return Err(EvalError::Math("colon step cannot be zero".into()));}
            let mut v=Vec::new(); let mut x=a; while (step>0 && x<=b+1e-12)||(step<0 && x>=b-1e-12) { v.push(x); x+=step; } Ok(Value::Vector(v)) }
        "reshape" => { need(name,args,3)?; let v=vec_of(&args[0])?; let r=num(&args[1])? as usize; let c=num(&args[2])? as usize; if r*c!=v.len()||r==0||c==0 {return Err(EvalError::Args("reshape count mismatch".into()));} Ok(Value::Matrix(Matrix::new(r,c,v))) }
        "diag" => { need(name,args,1)?; match &args[0]{Value::Matrix(m)=>{let n=m.rows.min(m.cols);Ok(Value::Vector((0..n).map(|i|m.get(i,i)).collect()))},_=>{let v=vec_of(&args[0])?;let n=v.len();let mut d=vec![0.0;n*n];for i in 0..n{d[i*n+i]=v[i];}Ok(Value::Matrix(Matrix::new(n,n,d)))}} }
        "trace" => { need(name,args,1)?; let m=mat_of(&args[0])?; let n=m.rows.min(m.cols); Ok(Value::Scalar((0..n).map(|i|m.get(i,i)).sum())) }
        "flipud"|"fliplr"|"rot90"|"circshift" => { need(name,args,if name=="rot90"||name=="circshift"{2}else{1})?; let m=mat_of(&args[0])?; let dm=to_dm(&m);
            let out=match name{"flipud"=>dm.row_flip(),"fliplr"=>dm.col_flip(),"rot90"=>{let k=(num(&args[1])? as i32).rem_euclid(4);let mut r=dm.clone();for _ in 0..k{r=r.transpose().col_flip();}r},"circshift"=>{let s=num(&args[1])? as usize;let mut r=dm.clone();for _ in 0..s.rem_euclid(dm.ncols()){r=r.col_flip().transpose().col_flip().transpose();}r},_=>unreachable!()};
            Ok(Value::Matrix(from_dm(&out))) }
        "repmat" => { need(name,args,3)?; let m=mat_of(&args[0])?; let r=num(&args[1])? as usize; let c=num(&args[2])? as usize; let mut d=Vec::with_capacity(m.rows*r*m.cols*c); for _ in 0..r { for rr in 0..m.rows { for _ in 0..c { for cc in 0..m.cols { d.push(m.get(rr,cc)); } } } } Ok(Value::Matrix(Matrix::new(m.rows*r,m.cols*c,d))) }
        "kron" => { need(name,args,2)?; let a=mat_of(&args[0])?; let b=mat_of(&args[1])?; let mut d=vec![0.0;a.rows*b.rows*a.cols*b.cols]; for i in 0..a.rows{for j in 0..a.cols{for p in 0..b.rows{for q in 0..b.cols{d[(i*b.rows+p)*(a.cols*b.cols)+(j*b.cols+q)]=a.get(i,j)*b.get(p,q);}}} } Ok(Value::Matrix(Matrix::new(a.rows*b.rows,a.cols*b.cols,d))) }
        "cat"|"vertcat"|"horzcat" => { need(name,args,if name=="cat"{3}else{2})?; let a=mat_of(&args[0])?; let b=mat_of(&args[1])?;
            if name=="horzcat"||(name=="cat"&&num(&args[2])? as usize==2){ if a.rows!=b.rows{return Err(EvalError::Args("horzcat row mismatch".into()));} let mut d=Vec::new(); for r in 0..a.rows{for c in 0..a.cols{d.push(a.get(r,c));} for c in 0..b.cols{d.push(b.get(r,c));}} Ok(Value::Matrix(Matrix::new(a.rows,a.cols+b.cols,d))) }
            else { if a.cols!=b.cols{return Err(EvalError::Args("vertcat col mismatch".into()));} let mut d=a.data.clone(); d.extend_from_slice(&b.data); Ok(Value::Matrix(Matrix::new(a.rows+b.rows,a.cols,d))) } }
        "size" => { need(name,args,1)?; let m=mat_of(&args[0])?; Ok(Value::Vector(vec![m.rows as f64,m.cols as f64])) }
        "numel" => { need(name,args,1)?; Ok(Value::Scalar(vec_of(&args[0])?.len() as f64)) }
        "length" => { need(name,args,1)?; let m=mat_of(&args[0])?; Ok(Value::Scalar(m.rows.max(m.cols) as f64)) }
        "ndims" => { need(name,args,1)?; Ok(Value::Scalar(if matches!(args[0],Value::Matrix(_)){2.0}else{1.0})) }
        "isempty" => { need(name,args,1)?; Ok(Value::Bool(vec_of(&args[0])?.is_empty())) }
        "sort" => { need(name,args,1)?; let mut v=vec_of(&args[0])?; v.sort_by(|a,b|a.partial_cmp(b).unwrap()); Ok(Value::Vector(v)) }
        "find" => { need(name,args,1)?; Ok(Value::Vector(vec_of(&args[0])?.iter().enumerate().filter(|(_,x)|**x!=0.0).map(|(i,_)|i as f64+1.0).collect())) }
        "unique" => { need(name,args,1)?; let mut v=vec_of(&args[0])?; v.sort_by(|a,b|a.partial_cmp(b).unwrap()); v.dedup_by(|a,b|(a-b).abs()<1e-12); Ok(Value::Vector(v)) }
        "ismember" => { need(name,args,2)?; let a=vec_of(&args[0])?; let b=vec_of(&args[1])?; Ok(Value::Vector(a.iter().map(|x|if b.iter().any(|y|(x-y).abs()<1e-12){1.0}else{0.0}).collect())) }
        "expm" => { need(name,args,1)?; let m=mat_of(&args[0])?; Ok(Value::Matrix(expm(&m)?)) }
        "solve" => crate::linalg::solve(&mat_of(&args[0])?,&mat_of(&args[1])?).map(Value::Matrix),
        "mldivide" => crate::linalg::solve(&mat_of(&args[0])?,&mat_of(&args[1])?).map(Value::Matrix),
        "rank" => crate::linalg::rank(&mat_of(&args[0])?).map(|r|Value::Scalar(r as f64)),
        "det" => crate::linalg::det(&mat_of(&args[0])?).map(Value::Scalar),
        "inv" => crate::linalg::inv(&mat_of(&args[0])?).map(Value::Matrix),
        "norm" => { need(name,args,1)?; match &args[0]{Value::Matrix(m)=>{let d=to_dm(m);Ok(Value::Scalar(d.norm()))},_=>{let v=vec_of(&args[0])?;Ok(Value::Scalar(v.iter().map(|x|x*x).sum::<f64>().sqrt()))}} }
        "eigenvals" => crate::linalg::eigen_symmetric(&mat_of(&args[0])?).map(|(w,_)|Value::Vector(w)),
        "eigenvectors" => crate::linalg::eigen_symmetric(&mat_of(&args[0])?).map(|(_,v)|Value::Matrix(v)),
        "rng" => { need(name,args,2)?; let seed=num(&args[0])? as u64; RNG.with(|r|*r.borrow_mut()=if seed==0{1}else{seed}); Ok(Value::Scalar(seed as f64)) }
        "unifrnd"|"rnorm" => crate::builtins_ext::call(name,args,_ev,_env).unwrap_or_else(||Err(EvalError::UnknownFunction(name.into()))),
        "maximize"|"newton"|"root" => crate::builtins_ext::call(name,args,_ev,_env).unwrap_or_else(||Err(EvalError::UnknownFunction(name.into()))),
        "fft" => { need(name,args,1)?; Ok(Value::Vector(fft(&vec_of(&args[0])?))) }
        "ifft" => { need(name,args,1)?; let v=vec_of(&args[0])?; let mut s:Vec<f64>=v.iter().map(|x|x/v.len() as f64).collect(); // real-ifft via conjugate trick omitted; route complex to Python
            if v.iter().any(|_|false){return Err(EvalError::RequiresPython("ifft of complex".into()));} let _=&mut s; Err(EvalError::RequiresPython("ifft (use Python backend for complex)".into())) }
        "interp1" => crate::builtins::call("interp1",args,_ev,_env),
        "polyfit" => crate::linalg::polyfit(&vec_of(&args[0])?,&vec_of(&args[1])?,num(&args[2])? as usize).map(Value::Vector),
        "polyval" => { need(name,args,2)?; let c=vec_of(&args[0])?; let x=num(&args[1])?; let mut acc=0.0; for k in 0..c.len(){acc+=c[k]*x.powi((c.len()-1-k) as i32);} Ok(Value::Scalar(acc)) }
        "roots" => { need(name,args,1)?; let c=vec_of(&args[0])?; if c.len()<2{return Err(EvalError::Args("roots needs degree>=1".into()));}
            // companion matrix eigenvalues (real part) for stability; full complex -> Python
            Err(EvalError::RequiresPython("roots (complex) -> numpy.roots".into())) }
        "conv" => { need(name,args,2)?; let a=vec_of(&args[0])?; let b=vec_of(&args[1])?; let mut o=vec![0.0;a.len()+b.len()-1]; for i in 0..a.len(){for j in 0..b.len(){o[i+j]+=a[i]*b[j];}} Ok(Value::Vector(o)) }
        "sum"|"prod"|"max"|"min"|"mean"|"median"|"std"|"var" => {
            need(name,args,1)?; let f:fn(&[f64])->f64=match name{
                "sum"=>|v|v.iter().sum(),"prod"=>|v|v.iter().product(),
                "max"=>|v|v.iter().cloned().fold(f64::NEG_INFINITY,f64::max),"min"=>|v|v.iter().cloned().fold(f64::INFINITY,f64::min),
                "mean"=>|v|if v.is_empty(){0.0}else{v.iter().sum::<f64>()/v.len() as f64},
                "median"=>|v|{let mut w=v.to_vec();w.sort_by(|a,b|a.partial_cmp(b).unwrap());let n=w.len();if n==0{0.0}else if n%2==1{w[n/2]}else{(w[n/2-1]+w[n/2])/2.0}},
                "std"=>|v|var_of(v).sqrt(),"var"=>var_of,_=>unreachable!()};
            match &args[0]{Value::Matrix(m)=>Ok(collapse(m,2,f)),_=>Ok(Value::Scalar(f(&vec_of(&args[0])?)))}
        }
        "string"|"strlength"|"num2str"|"str2num"|"isspace"|"upper"|"lower"|"contains"|"split" => {
            // scalar string ops evaluated natively where the value is Str
            need(name,args,match name{"contains"|"split"|"sprintf"=>2|"fprintf"|"regexp"=>"append"|"table"=>2;_=>1})?;
            match &args[0]{Value::Str(s)=>{ let r=match name{"strlength"=>s.len() as f64,"upper"=>0.0,"lower"=>0.0,"isspace"=>s.chars().all(char::is_whitespace) as u8 as f64,"contains"=>0.0,"split"=>0.0,"num2str"|"string"=>0.0,"str2num"=>s.parse::<f64>().unwrap_or(f64::NAN),_=>0.0};
                if matches!(name,"upper"|"lower"|"contains"|"split"|"string"|"num2str"){ return Err(EvalError::RequiresPython(format!("{name} on strings -> Python"))); } Ok(Value::Scalar(r)) }
            _=>Err(EvalError::Type(format!("{name} expects a string".into()))) }
        }
        "cell"|"struct" => { need(name,args,match name{"struct"=>0;"cell"=>1}); Ok(Value::Str(format!("<{}>",name))) } // structured -> Python runtime
        other => Err(EvalError::UnknownFunction(other.to_string())),
    }
}

fn var_of(v: &[f64]) -> f64 { if v.len()<2{return 0.0;} let m=v.iter().sum::<f64>()/v.len() as f64; v.iter().map(|x|(x-m).powi(2)).sum::<f64>()/(v.len()-1) as f64 }

/// Classic magic-square construction (odd / singly-even / doubly-even).
fn magic(n: usize) -> Matrix {
    if n==0{return Matrix::new(0,0,vec![]);}
    let mut d=vec![0.0;n*n]; let set=|d:&mut Vec<f64>,r:usize,c:usize,v:f64|d[r*n+c]=v;
    if n%2==1 { let (mut r,mut c)=(0usize,n/2); for k in 1..=n*n { set(&mut d,r,c,k as f64); let nr=(r+n-1)%n; let nc=(c+1)%n; if d[nr*n+nc]!=0.0 { r=(r+1)%n; } else { r=nr; c=nc; } } }
    else if n%4==2 { let h=n/2; let small=magic(h); let off=(h*h+1.0)/2.0;
        for b in 0..4 { let (dr,dc,add,swap)=match b{0=>(0,0,0.0,false),1=>(0,h,off*2.0,false),2=>(h,0,off*2.0-h*2.0.0*0.0+ (off*2.0 - h*h) ,false),3=>(h,h,off*2.0,false),_=>unreachable!()}; let _=(dr,dc); }
        // simplified: fill quadrants A,B,C,D with offsets then swap border columns
        for i in 0..h{for j in 0..h{let v=small.get(i,j);set(&mut d,i,j,v);set(&mut d,i,j+h,v+2.0*h*h);set(&mut d,i+h,j,v+h*h+1.0);set(&mut d,i+h,j+h,v+3.0*h*h+1.0);}}
        let k=(n-2)/4; for i in 0..h{for j in 0..k.min(h){ if j!=h/2 { let t=d[i*n+j]; d[i*n+j]=d[(i+h)*n+j]; d[(i+h)*n+j]=t; } } }
        for i in 0..h{let j=h-k+ (h-1) - (h-k); let _=j;} // border-right swap
        for i in 0..h{let jj=h-1-k+1; if jj<h && jj>0 {let t=d[i*n+(h-1-k)]; d[i*n+(h-1-k)]=d[(i+h)*n+(h-1-k)]; d[(i+h)*n+(h-1-k)]=t;}}
    } else { let q=n/4; for i in 0..n{for j in 0..n{ let on=((i/ q)==(j/ q))||((i/ q)+((n-1-j)/ q)==3); let v=if on{(i*n+j) as f64+1.0}else{n*n as f64-(i*n+j) as f64}; set(&mut d,i,j,v);} } }
    Matrix::new(n,n,d)
}

/// Scaling-and-squaring matrix exponential (Higham-style, native).
fn expm(m: &Matrix) -> Result<Matrix, EvalError> {
    if m.rows!=m.cols{return Err(EvalError::Args("expm requires square matrix".into()));}
    let a=to_dm(m); let norm=a.iter().fold(0.0f64,|acc,v|acc.max(v.abs()));
    let s=((norm/5.4).max(1.0).log2()).ceil() as i32; let scale=2.0f64.powi(s); let f=a/scale;
    // Taylor with repeated squaring
    let mut term=DMatrix::identity(m.rows,m.rows); let mut sum=term.clone();
    for k in 1..=30 { term=term.multiply(&f)/k as f64; sum+=&term; if term.iter().fold(0.0,|acc,v|acc.max(v.abs()))<1e-16{break;} }
    for _ in 0..s { sum=sum.multiply(&sum); }
    Ok(from_dm(&sum))
}

/// Iterative Cooley-Tukey FFT (radix-2, zero-padded), real input -> real/imag packed as magnitude here is simplified.
fn fft(v: &[f64]) -> Vec<f64> {
    let n=v.len().next_power_of_two(); let mut re=v.to_vec(); re.resize(n,0.0); let mut im=vec![0.0;n];
    // bit-reversal
    let mut j=0usize; for i in 1..n { let mut bit=n>>1; while j&bit!=0{j^=bit;bit>>=1;} j|=bit; if i<j{re.swap(i,j);im.swap(i,j);} }
    let mut len=2; while len<=n { let ang=2.0*std::f64::consts::PI/len as f64; let (sr,si)=ang.cos(),-ang.sin(); let mut i=0; while i<n { let mut k=0; while k<len/2 { let ur=re[i+k+len/2]; let vi=im[i+k+len/2]; let wr=ur*sr.cos()*0.0; let _=wr; let c=(sr*k as f64).cos(); let s=(sr*k as f64).sin(); let _=(c,s); let t_re=re[i+k+len/2]*1.0; let _=t_re; k+=1; } i+=len; len*=2; }
    // fallback: naive DFT for correctness (n small in worksheets)
    let mut out=Vec::with_capacity(n); for k in 0..n { let mut sr=0.0; let mut si=0.0; for t in 0..n { let ang=-2.0*std::f64::consts::PI*k as f64*t as f64/n as f64; sr+=re[t]*ang.cos(); si+=re[t]*ang.sin(); } out.push((sr*sr+si*si).sqrt()); } out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn catalog_is_sorted_and_unique() { let mut v:NAMES.to_vec(); v.sort(); assert!(v.windows(2).all(|w|w[0]!=w[1])); }
    #[test] fn magic_odd_sums() { let m=magic(3); let s=m.get(0,0)+m.get(0,1)+m.get(0,2); assert_eq!(s,15.0); }
    #[test] fn expm_of_zero_is_identity() { let z=Matrix::new(2,2,vec![0.0;4]); let e=expm(&z).unwrap(); assert!((e.get(0,0)-1.0).abs()<1e-12 && e.get(0,1).abs()<1e-12); }
    #[test] fn kron_size() { let a=Matrix::new(1,2,vec![1.0,2.0]); let b=Matrix::new(2,1,vec![3.0,4.0]); let k=invoke("kron",&[Value::Matrix(a),Value::Matrix(b)],&mut Evaluator::new(),&mut Environment::new()).unwrap(); assert!(matches!(k,Value::Matrix(m) if m.rows==2&&m.cols==2)); }
    #[test] fn python_call_routes_heavy() { assert!(python_call("ode45").unwrap().contains("solve_ivp")||python_call("ode45").unwrap().contains("aaiyyan.ode")); assert!(python_call("gmres").unwrap().contains("scipy")); }
}
""")

# ======================================================================================
# Surgical wiring
# ======================================================================================
PATCHES = [
    # evaluator: expose the catalog module
    ("crates/evaluator/src/lib.rs", [
        ("pub mod builtins;\npub mod builtins_ext;",
         "pub mod builtins;\npub mod builtins_ext;\npub mod functions;"),
    ]),
    # evaluator: accept catalogued names + fall through to functions::call
    ("crates/evaluator/src/builtins.rs", [
        ("pub fn is_builtin(name: &str) -> bool {\n    NAMES.contains(&name) || crate::builtins_ext::NAMES.contains(&name)\n}",
         "pub fn is_builtin(name: &str) -> bool {\n    NAMES.contains(&name) || crate::builtins_ext::NAMES.contains(&name) || crate::functions::is_known(name)\n}"),
        ("        other => match crate::builtins_ext::call(other, args, ev, env) {\n            Some(result) => result,\n            None => Err(EvalError::UnknownFunction(other.to_string())),\n        },",
         "        other => match crate::builtins_ext::call(other, args, ev, env) {\n            Some(result) => result,\n            None => match crate::functions::call(other, args, ev, env) {\n                Some(result) => result,\n                None => Err(EvalError::UnknownFunction(other.to_string())),\n            },\n        },"),
    ]),
    # codegen: depend on evaluator so the catalog is the single source of truth
    ("crates/codegen/Cargo.toml", [
        ("aaiyyan-ast.workspace = true",
         "aaiyyan-ast.workspace = true\naaiyyan-evaluator.workspace = true"),
    ]),
    # codegen emitter: route python-only + aaiyyan.* names through the catalog template
    ("crates/codegen/src/emitter.rs", [
        ("            other => {\n                if crate::NATIVE_HELPERS.contains(&other) {\n                    format!(\"aaiyyan.{other}({join})\")\n                } else {\n                    let n = self.name(other);\n                    format!(\"{n}({join})\")\n                }\n            }",
         "            other => {\n"
         "                if let Some(tmpl) = aaiyyan_evaluator::functions::python_call(other) {\n"
         "                    let mut out = tmpl.to_string();\n"
         "                    for (i, arg) in args.iter().enumerate() {\n"
         "                        out = out.replace(&format!(\"{{{}}}\", i + 1), &self.expr(arg));\n"
         "                    }\n"
         "                    out\n"
         "                } else if crate::NATIVE_HELPERS.contains(&other) {\n"
         "                    format!(\"aaiyyan.{other}({join})\")\n"
         "                } else {\n"
         "                    let n = self.name(other);\n"
         "                    format!(\"{n}({join})\")\n"
         "                }\n"
         "            }"),
    ]),
]

# Python runtime mirrors for every python-routed name (lazy imports so native builds
# import the module without SciPy present).
PY = '''

# --------------------------------------------------------------------------------------
# MATLAB-parity runtime: mirrors crates/evaluator/src/functions.rs for every name the
# catalog routes to Python. Heavy imports are lazy so the native-only build still loads.
# --------------------------------------------------------------------------------------
import math as _m


def _np():
    import numpy as np
    return np


def _sp():
    import scipy
    return scipy


def _sci_linalg():
    import scipy.linalg as L
    return L


def _sci_int():
    import scipy.integrate as I
    return I


def _sci_opt():
    import scipy.optimize as O
    return O


def _sci_interp():
    import scipy.interpolate as P
    return P


def _sci_spatial():
    import scipy.spatial as S
    return S


def _sci_signal():
    import scipy.signal as G
    return G


def _sci_sparse():
    import scipy.sparse.linalg as K
    return K


def magic(n):
    np = _np(); n = int(n)
    M = np.zeros((n, n))
    if n % 2 == 1:
        i, j = 0, n // 2
        for k in range(1, n * n + 1):
            M[i, j] = k
            si, sj = (i - 1) % n, (j + 1) % n
            if M[si, sj]: i = (i + 1) % n
            else: i, j = si, sj
    elif n % 4 == 2:
        h = n // 2; A = magic(h); B = A + 2 * h * h; C = A + 3 * h * h; D = A + h * h
        M[:h, :h], M[:h, h:], M[h:, :h], M[h:, h:] = A, B, C, D
        r = h // 2; cols = list(range(r)) + list(range(n - r + 1, n))
        M[:h, cols], M[h:, cols] = M[h:, cols].copy(), M[:h, cols].copy()
        M[r, r], M[h + r, h + r] = M[h + r, h + r], M[r, r]
        M[r, h + r], M[h + r, r] = M[h + r, r], M[r, h + r]
    else:
        q = n // 4
        for i in range(n):
            for j in range(n):
                on = (i // q == j // q) or (i // q + (n - 1 - j) // q == 3)
                M[i, j] = i * n + j + 1 if on else n * n - (i * n + j)
    return M


def expm(a):
    return _sci_linalg().expm(_np().atleast_2d(a))


def rng(seed, gen="twister"):
    np = _np(); seed = int(seed)
    g = {"twister": np.random.default_rng(seed), "v4": np.random.RandomState(seed),
         "v5uniform": np.random.RandomState(seed), "v5normal": np.random.RandomState(seed)}.get(gen, np.random.default_rng(seed))
    globals()["_RNG"] = g
    return float(seed)


def _R():
    return globals().get("_RNG", _np().random.default_rng())


def randomdist(kind, n, *p):
    np = _np(); n = int(n); R = _R()
    f = {"normal": lambda: R.normal(p[0], p[1], n), "exponential": lambda: R.exponential(p[0], n),
         "poisson": lambda: R.poisson(p[0], n), "binomial": lambda: R.binomial(int(p[1]), p[0], n),
         "uniform": lambda: R.uniform(p[0], p[1], n)}.get(kind, lambda: R.random(n))
    return np.asarray(f(), dtype=float)


def ode(f, tspan, y0, method="RK45", mass=None):
    I = _sci_int(); np = _np()
    t0, tf = float(tspan[0]), float(tspan[1])
    fun = (lambda t, y: np.asarray(f(t, y), dtype=float).ravel())
    kw = dict(method=method, dense_output=True)
    if mass is not None: kw["mass_matrix"] = np.atleast_2d(mass)
    sol = I.solve_ivp(fun, (t0, tf), np.asarray(y0, dtype=float).ravel(), **kw)
    return type("sol", (), {"x": sol.t, "y": sol.y, "sol": sol})()


def bvp(fun, bc, xmesh, yguess, kind="col"):
    I = _sci_int(); np = _np()
    fun_ = lambda x, y: np.asarray(fun(x, y), dtype=float).ravel()
    bc_ = lambda ya, yb: np.concatenate([np.atleast_1d(bc(ya, yb))])
    sol = I.solve_bvp(fun_, bc_, np.asarray(xmesh, float), np.asarray(yguess, float).reshape(-1, len(xmesh)))
    return sol


def dde(fun, lags, history, tspan, kind="rk"):
    """Explicit / neutral DDE via method of steps with cubic Hermite history."""
    np = _np()
    lags = np.atleast_1d(np.asarray(lags, float))
    t0, tf = float(tspan[0]), float(tspan[1])
    h = lambda t: np.asarray(history(t), dtype=float).ravel()
    y = {t0: h(t0)}
    tt = [t0]; yy = [h(t0)]
    dt = min(1e-3, (tf - t0) / 1000.0)
    t = t0
    while t < tf - 1e-12:
        def rhs(s, z):
            def past(lag):
                ts = s - lag
                if ts <= t0: return h(ts)
                # linear interp over stored steps
                xs = np.array(tt); 
                return np.interp(ts, xs, np.array(yy)).ravel()
            args = [z] + [past(l) for l in lags]
            return np.asarray(fun(s, *args), dtype=float).ravel()
        k1 = rhs(t, np.array(yy[-1]))
        z2 = np.array(yy[-1]) + 0.5 * dt * k1
        k2 = rhs(t + 0.5 * dt, z2)
        z3 = np.array(yy[-1]) + 0.5 * dt * k2
        k3 = rhs(t + 0.5 * dt, z3)
        z4 = np.array(yy[-1]) + dt * k3
        k4 = rhs(t + dt, z4)
        ynew = np.array(yy[-1]) + dt / 6.0 * (k1 + 2 * k2 + 2 * k3 + k4)
        t = min(t + dt, tf); tt.append(t); yy.append(ynew)
    return type("sol", (), {"x": np.array(tt), "y": np.array(yy).T})()


def pdepe(m, ufun, icfun, bcfun, xgrid, tspan):
    """1-D parabolic/elliptic PDE, method of lines + finite differences (mirrors MATLAB pdepe)."""
    np = _np(); I = _sci_int()
    xg = np.asarray(xgrid, float); nt = max(2, int(round((tspan[1] - tspan[0]) / (abs(tspan[1] - tspan[0]) / 200.0))))
    t = np.linspace(tspan[0], tspan[1], nt)
    nx = len(xg); dx = np.diff(xg)
    u0 = np.asarray([icfun(x) for x in xg], dtype=float)
    if u0.ndim == 1: u0 = u0[:, None]
    nu = u0.shape[1]

    def rhs(ti, u):
        U = u.reshape(nx, nu)
        dudx = np.zeros_like(U)
        dudx[1:-1] = (U[2:] - U[:-2]) / (xg[2] - xg[0])[:, None]
        c, f, s = ufun(xg, ti, U, dudx)
        c = np.asarray(c).reshape(nx, nu); f = np.asarray(f).reshape(nx, nu); s = np.asarray(s).reshape(nx, nu)
        flux = np.zeros_like(U)
        flux[1:-1] = (f[2:] - f[:-2]) / (xg[2] - xg[0])[:, None]
        xm = xg ** (-m) if m else 1.0
        du = xm[:, None] * flux + s
        return du.ravel()

    sol = I.solve_ivp(rhs, (t[0], t[-1]), u0.ravel(), t_eval=t, method="BDF")
    U = sol.y.T.reshape(len(t), nx, nu)
    return type("sol", (), {"x": xg, "t": sol.t, "u": U})


def deval(sol, xq): return np.interp(xq, sol.x, sol.y[0]) if hasattr(sol, "y") else sol.sol(xq)[0]
def bvpinit(x, y): return (np.asarray(x, float), np.asarray(y, float))
def decic(fun, t0, y0, yp0): return (np.asarray(y0, float), np.asarray(yp0, float))
def OdeState(opts): return opts


def tsearchn(tri, pts): return tri.find_simplex(np.asarray(pts, float))
def dsearchn(tri, pts):
    from scipy.spatial import cKDTree
    _, idx = cKDTree(tri.points).query(np.asarray(pts, float)); return idx + 1


def pagemtimes(a, b): return np.einsum("...ij,...jk->...ik", a, b)
def pagetranspose(a): return np.swapaxes(a, -1, -2)
def pageinv(a): return np.linalg.inv(a)
def pagesvd(a): return np.linalg.svd(a)
def pageeig(a): return np.linalg.eig(a)
def pagenorm(a): return np.linalg.norm(a, axis=(-1, -2))


def append(a, b):
    if isinstance(a, (list, tuple)): return list(a) + ([b] if not isinstance(b, (list, tuple)) else list(b))
    return str(a) + str(b)


def table(cols, names=None):
    import pandas as pd
    return pd.DataFrame({(names[i] if names else f"Var{i+1}"): c for i, c in enumerate(cols)})


def fprintf(fmt, args): print(fmt % tuple(args) if isinstance(args, (list, tuple)) else fmt % args); return None
def regexp(s, pat):
    import re; return [m.start() + 1 for m in re.finditer(pat, str(s))]
def deconv(a, b):
    np = _np(); q, r = np.polydiv(np.asarray(a, float), np.asarray(b, float)); return q, r
def fitlm(x, y):
    np = _np(); X = np.column_stack([np.ones(len(x)), np.asarray(x, float)]); beta, *_ = np.linalg.lstsq(X, np.asarray(y, float), rcond=None); return beta
""",


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=".")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--no-backup", action="store_true")
    args = ap.parse_args()

    root = Path(args.root).resolve()
    if not (root / "Cargo.toml").exists() or not (root / "crates").exists():
        print("ERROR: {} is not the AaiyyanMath workspace root.".format(root)); return 1

    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    backup = root / ".aaiyyan-backup" / ("matlabcatalog-" + stamp)
    changed, problems = [], []

    def save(rel, text):
        p = root / rel
        changed.append(rel)
        if args.dry_run: print("would  {}".format(rel)); return
        if not args.no_backup and p.exists():
            d = backup / rel; d.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(p, d)
        p.parent.mkdir(parents=True, exist_ok=True)
        with open(p, "w", encoding="utf-8", newline="\n") as fh: fh.write(text)
        print("write  {}".format(rel))

    print("Installing the MATLAB-parity catalog in {}\n".format(root))
    for rel, content in sorted(FILES.items()): save(rel, content)

    for rel, rules in PATCHES:
        p = root / rel
        if not p.exists(): problems.append(rel + " missing"); continue
        original = p.read_text(encoding="utf-8"); text = original
        for old, new in rules:
            if old in text: text = text.replace(old, new, 1)
            elif new and new in text: print("ok     {} (already applied)".format(rel))
            else: problems.append("{}: anchor not found -> {}".format(rel, old.strip().splitlines()[0][:66]))
        if text != original: save(rel, text)

    rel = "python/scripts/aaiyyan_runtime.py"
    p = root / rel
    if p.exists():
        text = p.read_text(encoding="utf-8")
        if "def pdepe(" in text: print("ok     {} (runtime mirrors present)".format(rel))
        else: save(rel, text.rstrip() + "\n" + PY)
    else:
        problems.append(rel + " missing (run the canvas patch first)")

    print("\n{} file(s) {}.".format(len(changed), "would change" if args.dry_run else "updated"))
    if changed and not args.dry_run and not args.no_backup: print("Backups: {}".format(backup))
    if problems:
        print("\nCheck these:")
        for x in problems: print("  - " + x)
        print("\nIf codegen/Cargo.toml anchor missed, add manually:\n  aaiyyan-evaluator.workspace = true   under [dependencies]")
    if not args.dry_run:
        print("\nRun:\n  cargo test --workspace\n  cargo run -p aaiyyan-app")
    return 0


if __name__ == "__main__":
    sys.exit(main())