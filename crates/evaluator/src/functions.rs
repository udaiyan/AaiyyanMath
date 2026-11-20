//! MATLAB-parity function catalog (learn_matlab.pdf / matlab_math.pdf).
//!
//! One authoritative table of named operations. Each entry records a shape class
//! and a routing decision: evaluated natively here, or dispatched to the embedded
//! Python backend (NumPy/SciPy) via `EvalError::RequiresPython`. Code generation
//! routes every catalogued name through `aaiyyan.mcall(...)` so exported Python is
//! always runnable and arity-flexible.
use crate::env::Environment;
use crate::interp::{dimensionless, Evaluator};
use crate::linalg::{from_dm, to_dm};
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
    pub python: bool,
    pub arity: u8,
    pub doc: &'static str,
}

const N: bool = false;
const P: bool = true;

pub const CATALOG: &[Spec] = &[
    // -- arrays & matrices (learn ch.1-2, math ch.1) --
    Spec { name: "zeros", class: Class::Array, python: N, arity: 2, doc: "m-by-n zeros" },
    Spec { name: "ones", class: Class::Array, python: N, arity: 2, doc: "m-by-n ones" },
    Spec { name: "eye", class: Class::Matrix, python: N, arity: 1, doc: "identity" },
    Spec { name: "magic", class: Class::Matrix, python: N, arity: 1, doc: "magic square" },
    Spec { name: "rand", class: Class::Array, python: N, arity: 2, doc: "uniform random" },
    Spec { name: "randn", class: Class::Array, python: N, arity: 2, doc: "normal random" },
    Spec { name: "randi", class: Class::Array, python: N, arity: 3, doc: "random integers" },
    Spec { name: "randperm", class: Class::Array, python: N, arity: 2, doc: "random permutation" },
    Spec { name: "linspace", class: Class::Array, python: N, arity: 3, doc: "evenly spaced" },
    Spec { name: "logspace", class: Class::Array, python: N, arity: 3, doc: "log spaced" },
    Spec { name: "colon", class: Class::Array, python: N, arity: 3, doc: "start:step:stop" },
    Spec { name: "reshape", class: Class::Array, python: N, arity: 3, doc: "reshape (column-major)" },
    Spec { name: "diag", class: Class::Array, python: N, arity: 1, doc: "diagonal" },
    Spec { name: "trace", class: Class::Matrix, python: N, arity: 1, doc: "sum of diagonal" },
    Spec { name: "flipud", class: Class::Array, python: N, arity: 1, doc: "flip rows" },
    Spec { name: "fliplr", class: Class::Array, python: N, arity: 1, doc: "flip columns" },
    Spec { name: "rot90", class: Class::Array, python: N, arity: 2, doc: "rotate 90 deg" },
    Spec { name: "circshift", class: Class::Array, python: N, arity: 2, doc: "circular shift" },
    Spec { name: "repmat", class: Class::Array, python: N, arity: 3, doc: "replicate/tile" },
    Spec { name: "kron", class: Class::Matrix, python: N, arity: 2, doc: "Kronecker product" },
    Spec { name: "cat", class: Class::Array, python: N, arity: 3, doc: "concatenate along dim" },
    Spec { name: "vertcat", class: Class::Array, python: N, arity: 2, doc: "vertical concat" },
    Spec { name: "horzcat", class: Class::Array, python: N, arity: 2, doc: "horizontal concat" },
    Spec { name: "size", class: Class::Array, python: N, arity: 1, doc: "dimensions" },
    Spec { name: "numel", class: Class::Array, python: N, arity: 1, doc: "element count" },
    Spec { name: "length", class: Class::Array, python: N, arity: 1, doc: "largest dimension" },
    Spec { name: "ndims", class: Class::Array, python: N, arity: 1, doc: "dimension count" },
    Spec { name: "isempty", class: Class::Array, python: N, arity: 1, doc: "empty test" },
    Spec { name: "sort", class: Class::Array, python: N, arity: 1, doc: "sort ascending" },
    Spec { name: "sortrows", class: Class::Array, python: N, arity: 1, doc: "sort rows" },
    Spec { name: "find", class: Class::Array, python: N, arity: 1, doc: "nonzero indices (1-based)" },
    Spec { name: "unique", class: Class::Array, python: N, arity: 1, doc: "unique values" },
    Spec { name: "ismember", class: Class::Array, python: N, arity: 2, doc: "membership" },
    Spec { name: "vec", class: Class::Array, python: N, arity: 1, doc: "flatten column-major" },
    Spec { name: "row", class: Class::Array, python: N, arity: 2, doc: "extract row" },
    Spec { name: "col", class: Class::Array, python: N, arity: 2, doc: "extract column" },
    Spec { name: "submatrix", class: Class::Array, python: N, arity: 5, doc: "extract block" },
    // -- linear algebra (math ch.2) --
    Spec { name: "expm", class: Class::Matrix, python: N, arity: 1, doc: "matrix exponential" },
    Spec { name: "solve", class: Class::Matrix, python: N, arity: 2, doc: "direct Ax=B" },
    Spec { name: "mldivide", class: Class::Matrix, python: N, arity: 2, doc: "backslash" },
    Spec { name: "rank", class: Class::Matrix, python: N, arity: 1, doc: "rank" },
    Spec { name: "det", class: Class::Matrix, python: N, arity: 1, doc: "determinant" },
    Spec { name: "inv", class: Class::Matrix, python: N, arity: 1, doc: "inverse" },
    Spec { name: "norm", class: Class::Array, python: N, arity: 1, doc: "norm" },
    Spec { name: "eigenvals", class: Class::Matrix, python: N, arity: 1, doc: "symmetric eigenvalues" },
    Spec { name: "eigenvectors", class: Class::Matrix, python: N, arity: 1, doc: "symmetric eigenvectors" },
    Spec { name: "lu", class: Class::Matrix, python: P, arity: 1, doc: "LU factorization" },
    Spec { name: "qr", class: Class::Matrix, python: P, arity: 1, doc: "QR factorization" },
    Spec { name: "chol", class: Class::Matrix, python: P, arity: 1, doc: "Cholesky" },
    Spec { name: "svd", class: Class::Matrix, python: P, arity: 1, doc: "singular values" },
    Spec { name: "eig", class: Class::Matrix, python: P, arity: 1, doc: "eigen decomposition" },
    Spec { name: "eigs", class: Class::Matrix, python: P, arity: 2, doc: "few eigenvalues" },
    Spec { name: "svds", class: Class::Matrix, python: P, arity: 2, doc: "few singular values" },
    Spec { name: "cond", class: Class::Matrix, python: P, arity: 1, doc: "condition number" },
    Spec { name: "logm", class: Class::Matrix, python: P, arity: 1, doc: "matrix logarithm" },
    Spec { name: "sqrtm", class: Class::Matrix, python: P, arity: 1, doc: "matrix square root" },
    Spec { name: "sylvester", class: Class::Matrix, python: P, arity: 3, doc: "AX+XB=C" },
    Spec { name: "lyap", class: Class::Matrix, python: P, arity: 2, doc: "Lyapunov" },
    Spec { name: "pagemtimes", class: Class::Matrix, python: P, arity: 2, doc: "page-wise multiply" },
    Spec { name: "pagetranspose", class: Class::Matrix, python: P, arity: 1, doc: "page-wise transpose" },
    Spec { name: "pageinv", class: Class::Matrix, python: P, arity: 1, doc: "page-wise inverse" },
    Spec { name: "pagesvd", class: Class::Matrix, python: P, arity: 1, doc: "page-wise SVD" },
    Spec { name: "pageeig", class: Class::Matrix, python: P, arity: 1, doc: "page-wise eig" },
    Spec { name: "pagenorm", class: Class::Matrix, python: P, arity: 1, doc: "page-wise norm" },
    // -- iterative solvers (math ch.2 / sparse ch.4) --
    Spec { name: "pcg", class: Class::Matrix, python: P, arity: 2, doc: "precond. conjugate gradients" },
    Spec { name: "bicg", class: Class::Matrix, python: P, arity: 2, doc: "biconjugate gradients" },
    Spec { name: "bicgstab", class: Class::Matrix, python: P, arity: 2, doc: "BiCG stabilized" },
    Spec { name: "cgs", class: Class::Matrix, python: P, arity: 2, doc: "conjugate gradients squared" },
    Spec { name: "gmres", class: Class::Matrix, python: P, arity: 2, doc: "generalized min residual" },
    Spec { name: "minres", class: Class::Matrix, python: P, arity: 2, doc: "minimum residual" },
    Spec { name: "qmr", class: Class::Matrix, python: P, arity: 2, doc: "quasi-minimal residual" },
    Spec { name: "symmlq", class: Class::Matrix, python: P, arity: 2, doc: "symmetric LQ" },
    Spec { name: "tfqmr", class: Class::Matrix, python: P, arity: 2, doc: "transpose-free QMR" },
    Spec { name: "lsqr", class: Class::Matrix, python: P, arity: 2, doc: "LSQR" },
    Spec { name: "lsmr", class: Class::Matrix, python: P, arity: 2, doc: "LSMR" },
    // -- random (learn ch.3, math ch.3) --
    Spec { name: "rng", class: Class::Structured, python: N, arity: 2, doc: "seed + generator" },
    Spec { name: "unifrnd", class: Class::Array, python: N, arity: 3, doc: "uniform samples" },
    Spec { name: "normrnd", class: Class::Array, python: N, arity: 3, doc: "normal samples" },
    Spec { name: "exprnd", class: Class::Array, python: P, arity: 2, doc: "exponential samples" },
    Spec { name: "poissrnd", class: Class::Array, python: P, arity: 2, doc: "Poisson samples" },
    Spec { name: "binornd", class: Class::Array, python: P, arity: 3, doc: "binomial samples" },
    // -- ODE / BVP / DDE / PDE (math ch.11-14) --
    Spec { name: "ode45", class: Class::Structured, python: P, arity: 3, doc: "Runge-Kutta (4,5)" },
    Spec { name: "ode23", class: Class::Structured, python: P, arity: 3, doc: "Runge-Kutta (2,3)" },
    Spec { name: "ode113", class: Class::Structured, python: P, arity: 3, doc: "Adams/BDF variable order" },
    Spec { name: "ode15s", class: Class::Structured, python: P, arity: 4, doc: "stiff, mass matrix" },
    Spec { name: "ode23s", class: Class::Structured, python: P, arity: 3, doc: "stiff Rosenbrock" },
    Spec { name: "ode23t", class: Class::Structured, python: P, arity: 3, doc: "trapezoidal" },
    Spec { name: "ode23tb", class: Class::Structured, python: P, arity: 3, doc: "TR-BDF2" },
    Spec { name: "decic", class: Class::Structured, python: P, arity: 4, doc: "consistent initial conditions" },
    Spec { name: "bvp4c", class: Class::Structured, python: P, arity: 4, doc: "collocation BVP" },
    Spec { name: "bvp5c", class: Class::Structured, python: P, arity: 4, doc: "5-stage collocation BVP" },
    Spec { name: "bvpinit", class: Class::Structured, python: P, arity: 2, doc: "BVP initial guess" },
    Spec { name: "deval", class: Class::Structured, python: P, arity: 2, doc: "evaluate solver solution" },
    Spec { name: "dde23", class: Class::Structured, python: P, arity: 4, doc: "constant-delay DDE" },
    Spec { name: "ddesd", class: Class::Structured, python: P, arity: 4, doc: "state-dependent DDE" },
    Spec { name: "ddensd", class: Class::Structured, python: P, arity: 4, doc: "neutral DDE" },
    Spec { name: "pdepe", class: Class::Structured, python: P, arity: 6, doc: "1-D parabolic/elliptic PDE" },
    // -- interpolation / fitting / geometry (math ch.7-8) --
    Spec { name: "interp1", class: Class::Array, python: N, arity: 3, doc: "1-D interpolation" },
    Spec { name: "interp2", class: Class::Array, python: P, arity: 4, doc: "2-D interpolation" },
    Spec { name: "interp3", class: Class::Array, python: P, arity: 5, doc: "3-D interpolation" },
    Spec { name: "interpn", class: Class::Array, python: P, arity: 4, doc: "N-D interpolation" },
    Spec { name: "griddedInterpolant", class: Class::Structured, python: P, arity: 3, doc: "gridded interpolant" },
    Spec { name: "scatteredInterpolant", class: Class::Structured, python: P, arity: 3, doc: "scattered interpolant" },
    Spec { name: "spline", class: Class::Array, python: P, arity: 3, doc: "cubic spline" },
    Spec { name: "pchip", class: Class::Array, python: P, arity: 3, doc: "shape-preserving cubic" },
    Spec { name: "csaps", class: Class::Array, python: P, arity: 3, doc: "smoothing spline" },
    Spec { name: "delaunay", class: Class::Structured, python: P, arity: 2, doc: "2-D Delaunay" },
    Spec { name: "delaunayn", class: Class::Structured, python: P, arity: 1, doc: "N-D Delaunay" },
    Spec { name: "tsearchn", class: Class::Structured, python: P, arity: 2, doc: "simplex location" },
    Spec { name: "dsearchn", class: Class::Structured, python: P, arity: 2, doc: "nearest vertex" },
    Spec { name: "convhull", class: Class::Structured, python: P, arity: 2, doc: "convex hull" },
    // -- polynomials, transforms, signal (math ch.6, ch.16) --
    Spec { name: "polyfit", class: Class::Array, python: N, arity: 3, doc: "polynomial fit" },
    Spec { name: "polyval", class: Class::Array, python: N, arity: 2, doc: "evaluate polynomial" },
    Spec { name: "polyder", class: Class::Array, python: N, arity: 1, doc: "polynomial derivative" },
    Spec { name: "polyint", class: Class::Array, python: N, arity: 2, doc: "polynomial integral" },
    Spec { name: "roots", class: Class::Array, python: P, arity: 1, doc: "polynomial roots" },
    Spec { name: "conv", class: Class::Array, python: N, arity: 2, doc: "convolution" },
    Spec { name: "deconv", class: Class::Array, python: N, arity: 2, doc: "polynomial division" },
    Spec { name: "trapz", class: Class::Array, python: N, arity: 2, doc: "trapezoidal integration" },
    Spec { name: "cumtrapz", class: Class::Array, python: N, arity: 2, doc: "cumulative trapezoid" },
    Spec { name: "gradient", class: Class::Array, python: N, arity: 2, doc: "central differences" },
    Spec { name: "diff", class: Class::Array, python: N, arity: 1, doc: "forward differences" },
    Spec { name: "nextpow2", class: Class::Array, python: N, arity: 1, doc: "next power of two" },
    Spec { name: "fftshift", class: Class::Array, python: N, arity: 1, doc: "zero-centred shift" },
    Spec { name: "fft", class: Class::Array, python: P, arity: 1, doc: "fast Fourier transform" },
    Spec { name: "ifft", class: Class::Array, python: P, arity: 1, doc: "inverse FFT" },
    Spec { name: "fft2", class: Class::Array, python: P, arity: 1, doc: "2-D FFT" },
    Spec { name: "conv2", class: Class::Array, python: P, arity: 2, doc: "2-D convolution" },
    Spec { name: "filter", class: Class::Array, python: P, arity: 3, doc: "digital filter" },
    Spec { name: "pwelch", class: Class::Array, python: P, arity: 2, doc: "Welch PSD" },
    Spec { name: "periodogram", class: Class::Array, python: P, arity: 2, doc: "periodogram PSD" },
    // -- optimization (math ch.9-10) --
    Spec { name: "fminbnd", class: Class::Structured, python: P, arity: 3, doc: "bounded 1-D minimum" },
    Spec { name: "fminsearch", class: Class::Structured, python: P, arity: 2, doc: "Nelder-Mead" },
    Spec { name: "fmincon", class: Class::Structured, python: P, arity: 3, doc: "constrained minimum" },
    Spec { name: "fsolve", class: Class::Structured, python: P, arity: 2, doc: "nonlinear system root" },
    Spec { name: "fzero", class: Class::Structured, python: P, arity: 3, doc: "scalar root" },
    Spec { name: "lsqnonlin", class: Class::Structured, python: P, arity: 2, doc: "nonlinear least squares" },
    Spec { name: "lsqcurvefit", class: Class::Structured, python: P, arity: 4, doc: "curve fitting" },
    Spec { name: "fitlm", class: Class::Structured, python: P, arity: 2, doc: "linear model" },
    Spec { name: "maximize", class: Class::Structured, python: N, arity: 3, doc: "golden-section max" },
    Spec { name: "newton", class: Class::Structured, python: N, arity: 2, doc: "Newton root" },
    Spec { name: "root", class: Class::Structured, python: N, arity: 3, doc: "bracketed root" },
    // -- strings / cells / tables (learn ch.2) --
    Spec { name: "string", class: Class::Structured, python: N, arity: 1, doc: "to string" },
    Spec { name: "strlength", class: Class::Structured, python: N, arity: 1, doc: "string length" },
    Spec { name: "num2str", class: Class::Structured, python: N, arity: 1, doc: "number to string" },
    Spec { name: "str2num", class: Class::Structured, python: N, arity: 1, doc: "string to number" },
    Spec { name: "upper", class: Class::Structured, python: N, arity: 1, doc: "uppercase" },
    Spec { name: "lower", class: Class::Structured, python: N, arity: 1, doc: "lowercase" },
    Spec { name: "contains", class: Class::Structured, python: N, arity: 2, doc: "substring test" },
    Spec { name: "isspace", class: Class::Structured, python: N, arity: 1, doc: "whitespace test" },
    Spec { name: "append", class: Class::Structured, python: N, arity: 2, doc: "concatenate text" },
    Spec { name: "split", class: Class::Structured, python: P, arity: 2, doc: "split text" },
    Spec { name: "sprintf", class: Class::Structured, python: P, arity: 2, doc: "formatted string" },
    Spec { name: "fprintf", class: Class::Structured, python: P, arity: 2, doc: "formatted output" },
    Spec { name: "regexp", class: Class::Structured, python: P, arity: 2, doc: "regular expression" },
    Spec { name: "cell", class: Class::Structured, python: P, arity: 1, doc: "cell array" },
    Spec { name: "struct", class: Class::Structured, python: P, arity: 0, doc: "structure" },
    Spec { name: "table", class: Class::Structured, python: P, arity: 2, doc: "mixed-data table" },
    // -- reductions with dimension argument --
    Spec { name: "sumrange", class: Class::Array, python: P, arity: 3, doc: "sum of f(k) for k = lo..hi" },
    Spec { name: "prodrange", class: Class::Array, python: P, arity: 3, doc: "product of f(k) for k = lo..hi" },
    Spec { name: "sum", class: Class::Array, python: N, arity: 2, doc: "sum (optionally along dim)" },
    Spec { name: "prod", class: Class::Array, python: N, arity: 2, doc: "product" },
    Spec { name: "max", class: Class::Array, python: N, arity: 2, doc: "maximum" },
    Spec { name: "min", class: Class::Array, python: N, arity: 2, doc: "minimum" },
    Spec { name: "mean", class: Class::Array, python: N, arity: 2, doc: "mean" },
    Spec { name: "median", class: Class::Array, python: N, arity: 2, doc: "median" },
    Spec { name: "std", class: Class::Array, python: N, arity: 2, doc: "standard deviation" },
    Spec { name: "var", class: Class::Array, python: N, arity: 2, doc: "variance" },
    Spec { name: "rms", class: Class::Array, python: N, arity: 1, doc: "root mean square" },
    Spec { name: "geomean", class: Class::Array, python: N, arity: 1, doc: "geometric mean" },
    Spec { name: "harmmean", class: Class::Array, python: N, arity: 1, doc: "harmonic mean" },
    Spec { name: "cumsum", class: Class::Array, python: N, arity: 1, doc: "cumulative sum" },
    Spec { name: "cumprod", class: Class::Array, python: N, arity: 1, doc: "cumulative product" },
];

pub fn spec(name: &str) -> Option<&'static Spec> { CATALOG.iter().find(|s| s.name == name) }
pub fn is_known(name: &str) -> bool { spec(name).is_some() }

thread_local! { static RNG: RefCell<u64> = RefCell::new(0x2545F4914F6CDD1D); }
fn unit() -> f64 {
    RNG.with(|r| { let mut s = r.borrow_mut(); *s ^= *s << 13; *s ^= *s >> 7; *s ^= *s << 17; (*s >> 11) as f64 / 9007199254740992.0 })
}
fn num(v: &Value) -> Result<f64, EvalError> { dimensionless(v) }
fn vec_of(v: &Value) -> Result<Vec<f64>, EvalError> {
    match v {
        Value::Vector(a) => Ok(a.clone()),
        // MATLAB flattens any array in column-major order for numel/reductions.
        Value::Matrix(m) => Ok(column_major(m)),
        Value::Scalar(x) => Ok(vec![*x]),
        o => Err(EvalError::Type(format!("expected an array, got {}", o.type_name()))),
    }
}
fn mat_of(v: &Value) -> Result<Matrix, EvalError> {
    match v {
        Value::Matrix(m) => Ok(m.clone()),
        Value::Vector(a) => Ok(Matrix::new(a.len(), 1, a.clone())),
        o => Err(EvalError::Type(format!("expected a matrix, got {}", o.type_name()))),
    }
}
fn need(name: &str, got: usize, min: usize) -> Result<(), EvalError> {
    if got < min { Err(EvalError::Args(format!("{name} needs at least {min} argument(s), got {got}"))) } else { Ok(()) }
}
fn maxabs(m: &DMatrix<f64>) -> f64 { m.iter().fold(0.0, |a, v| a.max(v.abs())) }

/// Native evaluation entry. `None` when the name is not in the catalog.
pub fn call(name: &str, args: &[Value], ev: &mut Evaluator, env: &mut Environment) -> Option<Result<Value, EvalError>> {
    let s = spec(name)?;
    if s.python {
        return Some(Err(EvalError::RequiresPython(format!("{name} ({}) — routed to embedded Python", s.doc))));
    }
    Some(invoke(name, args, ev, env))
}

#[allow(clippy::too_many_arguments)]
fn invoke(name: &str, args: &[Value], ev: &mut Evaluator, env: &mut Environment) -> Result<Value, EvalError> {
    match name {
        "zeros" | "ones" => {
            need(name, args.len(), 1)?;
            let r = num(&args[0])? as usize;
            let c = if args.len() > 1 { num(&args[1])? as usize } else { r };
            let fill = if name == "zeros" { 0.0 } else { 1.0 };
            Ok(Value::Matrix(Matrix::new(r, c, vec![fill; r * c])))
        }
        "eye" => {
            need(name, args.len(), 1)?;
            let n = num(&args[0])? as usize;
            let mut d = vec![0.0; n * n];
            for i in 0..n { d[i * n + i] = 1.0; }
            Ok(Value::Matrix(Matrix::new(n, n, d)))
        }
        "magic" => {
            need(name, args.len(), 1)?;
            let n = num(&args[0])? as usize;
            Ok(Value::Matrix(magic(n)))
        }
        "rand" | "randn" => {
            need(name, args.len(), 1)?;
            let r = num(&args[0])? as usize;
            let c = if args.len() > 1 { num(&args[1])? as usize } else { 1 };
            let mut v = Vec::with_capacity(r * c);
            for _ in 0..r * c {
                if name == "rand" { v.push(unit()); }
                else {
                    let u1 = unit().max(1e-12);
                    let u2 = unit();
                    v.push((-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos());
                }
            }
            Ok(Value::Matrix(Matrix::new(r, c, v)))
        }
        "randi" => {
            need(name, args.len(), 2)?;
            let hi = num(&args[0])?.max(1.0) as i64;
            let n = num(&args[1])? as usize;
            Ok(Value::Vector((0..n).map(|_| 1.0 + (unit() * hi as f64).floor().min((hi - 1) as f64)).collect()))
        }
        "randperm" => {
            need(name, args.len(), 1)?;
            let n = num(&args[0])? as usize;
            let mut v: Vec<f64> = (1..=n).map(|x| x as f64).collect();
            for i in (1..n).rev() { let j = (unit() * (i + 1) as f64).floor() as usize; v.swap(i, j); }
            Ok(Value::Vector(v))
        }
        "linspace" => {
            need(name, args.len(), 3)?;
            let (a, b, n) = (num(&args[0])?, num(&args[1])?, num(&args[2])? as usize);
            if n < 2 { return Err(EvalError::Args("linspace needs n >= 2".into())); }
            Ok(Value::Vector((0..n).map(|i| a + (b - a) * i as f64 / (n - 1) as f64).collect()))
        }
        "logspace" => {
            need(name, args.len(), 3)?;
            let (a, b, n) = (num(&args[0])?, num(&args[1])?, num(&args[2])? as usize);
            Ok(Value::Vector((0..n).map(|i| 10f64.powf(a + (b - a) * i as f64 / (n - 1) as f64)).collect()))
        }
        "colon" => {
            need(name, args.len(), 2)?;
            let (a, step, b) = match args.len() {
                2 => (num(&args[0])?, 1.0, num(&args[1])?),
                _ => (num(&args[0])?, num(&args[1])?, num(&args[2])?),
            };
            if step == 0.0 { return Err(EvalError::Math("colon step cannot be zero".into())); }
            let mut v = Vec::new();
            let mut x = a;
            while (step > 0.0 && x <= b + 1e-12) || (step < 0.0 && x >= b - 1e-12) { v.push(x); x += step; }
            Ok(Value::Vector(v))
        }
        "reshape" => {
            need(name, args.len(), 3)?;
            let v = column_major(&mat_of(&args[0])?);
            let (r, c) = (num(&args[1])? as usize, num(&args[2])? as usize);
            if r * c != v.len() || r == 0 || c == 0 { return Err(EvalError::Args("reshape: element count mismatch".into())); }
            // MATLAB preserves linear (column-major) order: output element
            // (row, col) is v[col * r + row]. Our Matrix stores row-major data.
            let mut data = vec![0.0; r * c];
            for row in 0..r {
                for col in 0..c {
                    data[row * c + col] = v[col * r + row];
                }
            }
            Ok(Value::Matrix(Matrix::new(r, c, data)))
        }
        "diag" => {
            need(name, args.len(), 1)?;
            match &args[0] {
                Value::Matrix(m) => { let n = m.rows.min(m.cols); Ok(Value::Vector((0..n).map(|i| m.get(i, i)).collect())) }
                _ => {
                    let v = vec_of(&args[0])?;
                    let n = v.len();
                    let mut d = vec![0.0; n * n];
                    for i in 0..n { d[i * n + i] = v[i]; }
                    Ok(Value::Matrix(Matrix::new(n, n, d)))
                }
            }
        }
        "trace" => { need(name, args.len(), 1)?; let m = mat_of(&args[0])?; let n = m.rows.min(m.cols); Ok(Value::Scalar((0..n).map(|i| m.get(i, i)).sum())) }
        "flipud" | "fliplr" => {
            need(name, args.len(), 1)?;
            let m = mat_of(&args[0])?;
            let mut d = m.data.clone();
            if name == "flipud" {
                for r in 0..m.rows / 2 { for c in 0..m.cols { d.swap(r * m.cols + c, (m.rows - 1 - r) * m.cols + c); } }
            } else {
                for r in 0..m.rows { for c in 0..m.cols / 2 { d.swap(r * m.cols + c, r * m.cols + m.cols - 1 - c); } }
            }
            Ok(Value::Matrix(Matrix::new(m.rows, m.cols, d)))
        }
        "rot90" => {
            need(name, args.len(), 1)?;
            let m = mat_of(&args[0])?;
            let k = if args.len() > 1 { (num(&args[1])? as i32).rem_euclid(4) } else { 1 };
            let mut cur = m;
            for _ in 0..k {
                let mut d = vec![0.0; cur.rows * cur.cols];
                for r in 0..cur.rows { for c in 0..cur.cols { d[(cur.cols - 1 - c) * cur.rows + r] = cur.get(r, c); } }
                cur = Matrix::new(cur.cols, cur.rows, d);
            }
            Ok(Value::Matrix(cur))
        }
        "circshift" => {
            need(name, args.len(), 2)?;
            let m = mat_of(&args[0])?;
            let s = num(&args[1])? as isize;
            let mut d = vec![0.0; m.rows * m.cols];
            for c in 0..m.cols {
                let tc = (c as isize + s).rem_euclid(m.cols as isize) as usize;
                for r in 0..m.rows { d[r * m.cols + tc] = m.get(r, c); }
            }
            Ok(Value::Matrix(Matrix::new(m.rows, m.cols, d)))
        }
        "repmat" => {
            need(name, args.len(), 3)?;
            let m = mat_of(&args[0])?;
            let (r, c) = (num(&args[1])? as usize, num(&args[2])? as usize);
            let mut d = Vec::with_capacity(m.rows * r * m.cols * c);
            for _ in 0..r { for rr in 0..m.rows { for _ in 0..c { for cc in 0..m.cols { d.push(m.get(rr, cc)); } } } }
            Ok(Value::Matrix(Matrix::new(m.rows * r, m.cols * c, d)))
        }
        "kron" => {
            need(name, args.len(), 2)?;
            let (a, b) = (mat_of(&args[0])?, mat_of(&args[1])?);
            let mut d = vec![0.0; a.rows * b.rows * a.cols * b.cols];
            for i in 0..a.rows { for j in 0..a.cols { for p in 0..b.rows { for q in 0..b.cols {
                d[(i * b.rows + p) * (a.cols * b.cols) + (j * b.cols + q)] = a.get(i, j) * b.get(p, q);
            } } } }
            Ok(Value::Matrix(Matrix::new(a.rows * b.rows, a.cols * b.cols, d)))
        }
        "cat" | "vertcat" | "horzcat" => {
            need(name, args.len(), 2)?;
            let (a, b) = (mat_of(&args[0])?, mat_of(&args[1])?);
            let along = if name == "horzcat" { 2 } else if name == "vertcat" { 1 } else { num(&args[2])? as usize };
            if along == 2 {
                if a.rows != b.rows { return Err(EvalError::Args("horzcat: row counts differ".into())); }
                let mut d = Vec::new();
                for r in 0..a.rows { for c in 0..a.cols { d.push(a.get(r, c)); } for c in 0..b.cols { d.push(b.get(r, c)); } }
                Ok(Value::Matrix(Matrix::new(a.rows, a.cols + b.cols, d)))
            } else {
                if a.cols != b.cols { return Err(EvalError::Args("vertcat: column counts differ".into())); }
                let mut d = a.data.clone(); d.extend_from_slice(&b.data);
                Ok(Value::Matrix(Matrix::new(a.rows + b.rows, a.cols, d)))
            }
        }
        "size" => { need(name, args.len(), 1)?; let m = mat_of(&args[0])?; Ok(Value::Vector(vec![m.rows as f64, m.cols as f64])) }
        "numel" => { need(name, args.len(), 1)?; Ok(Value::Scalar(vec_of(&args[0])?.len() as f64)) }
        "length" => { need(name, args.len(), 1)?; let m = mat_of(&args[0])?; Ok(Value::Scalar(m.rows.max(m.cols) as f64)) }
        "ndims" => { need(name, args.len(), 1)?; Ok(Value::Scalar(if matches!(args[0], Value::Matrix(_)) { 2.0 } else { 1.0 })) }
        "isempty" => { need(name, args.len(), 1)?; Ok(Value::Bool(vec_of(&args[0])?.is_empty())) }
        "sort" => {
            need(name, args.len(), 1)?;
            match &args[0] {
                Value::Matrix(m) => {
                    let mut d = m.data.clone();
                    for c in 0..m.cols {
                        let mut col: Vec<f64> = (0..m.rows).map(|r| m.get(r, c)).collect();
                        col.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        for r in 0..m.rows { d[r * m.cols + c] = col[r]; }
                    }
                    Ok(Value::Matrix(Matrix::new(m.rows, m.cols, d)))
                }
                _ => { let mut v = vec_of(&args[0])?; v.sort_by(|a, b| a.partial_cmp(b).unwrap()); Ok(Value::Vector(v)) }
            }
        }
        "sortrows" => {
            need(name, args.len(), 1)?;
            let m = mat_of(&args[0])?;
            let mut rows: Vec<Vec<f64>> = (0..m.rows).map(|r| (0..m.cols).map(|c| m.get(r, c)).collect()).collect();
            rows.sort_by(|a, b| { for i in 0..a.len().min(b.len()) { match a[i].partial_cmp(&b[i]).unwrap() { std::cmp::Ordering::Equal => continue, o => return o, } } std::cmp::Ordering::Equal });
            let d: Vec<f64> = rows.concat();
            Ok(Value::Matrix(Matrix::new(m.rows, m.cols, d)))
        }
        "find" => { need(name, args.len(), 1)?; Ok(Value::Vector(vec_of(&args[0])?.iter().enumerate().filter(|(_, x)| **x != 0.0).map(|(i, _)| i as f64 + 1.0).collect())) }
        "unique" => { need(name, args.len(), 1)?; let mut v = vec_of(&args[0])?; v.sort_by(|a, b| a.partial_cmp(b).unwrap()); v.dedup_by(|a, b| (*a - *b).abs() < 1e-12); Ok(Value::Vector(v)) }
        "ismember" => {
            need(name, args.len(), 2)?;
            let (a, b) = (vec_of(&args[0])?, vec_of(&args[1])?);
            Ok(Value::Vector(a.iter().map(|x| if b.iter().any(|y| (x - y).abs() < 1e-12) { 1.0 } else { 0.0 }).collect()))
        }
        "vec" => { need(name, args.len(), 1)?; Ok(Value::Vector(column_major(&mat_of(&args[0])?))) }
        "row" | "col" => {
            need(name, args.len(), 2)?;
            let m = mat_of(&args[0])?;
            let i = num(&args[1])? as usize;
            if i >= if name == "row" { m.rows } else { m.cols } { return Err(EvalError::Index("row/col index out of range".into())); }
            Ok(Value::Vector(if name == "row" { (0..m.cols).map(|c| m.get(i, c)).collect() } else { (0..m.rows).map(|r| m.get(r, i)).collect() }))
        }
        "submatrix" => {
            need(name, args.len(), 5)?;
            let m = mat_of(&args[0])?;
            let (r0, r1) = (num(&args[1])? as usize, num(&args[2])? as usize);
            let (c0, c1) = (num(&args[3])? as usize, num(&args[4])? as usize);
            if r1 < r0 || c1 < c0 || r1 >= m.rows || c1 >= m.cols { return Err(EvalError::Index("submatrix range out of bounds".into())); }
            let mut d = Vec::new();
            for r in r0..=r1 { for c in c0..=c1 { d.push(m.get(r, c)); } }
            Ok(Value::Matrix(Matrix::new(r1 - r0 + 1, c1 - c0 + 1, d)))
        }
        "expm" => { need(name, args.len(), 1)?; Ok(Value::Matrix(expm(&mat_of(&args[0])?)?)) }
        "solve" | "mldivide" => crate::linalg::solve(&mat_of(&args[0])?, &mat_of(&args[1])?).map(Value::Matrix),
        "rank" => crate::linalg::rank(&mat_of(&args[0])?).map(|r| Value::Scalar(r as f64)),
        "det" => crate::linalg::det(&mat_of(&args[0])?).map(Value::Scalar),
        "inv" => crate::linalg::inv(&mat_of(&args[0])?).map(Value::Matrix),
        "norm" => {
            need(name, args.len(), 1)?;
            match &args[0] {
                Value::Matrix(m) => Ok(Value::Scalar(to_dm(m).norm())),
                _ => { let v = vec_of(&args[0])?; Ok(Value::Scalar(v.iter().map(|x| x * x).sum::<f64>().sqrt())) }
            }
        }
        "eigenvals" => crate::linalg::eigen_symmetric(&mat_of(&args[0])?).map(|(w, _)| Value::Vector(w)),
        "eigenvectors" => crate::linalg::eigen_symmetric(&mat_of(&args[0])?).map(|(_, v)| Value::Matrix(v)),
        "rng" => {
            need(name, args.len(), 1)?;
            let seed = num(&args[0])? as u64;
            RNG.with(|r| *r.borrow_mut() = if seed == 0 { 1 } else { seed });
            Ok(Value::Scalar(seed as f64))
        }
        "unifrnd" | "normrnd" => {
            // MATLAB signatures: unifrnd(a, b, n) and normrnd(mu, sigma, n).
            // The internal helpers take (n, a, b), so reorder here. Also accept
            // the legacy (n, a, b) form when only two arguments are supplied.
            need(name, args.len(), 2)?;
            let (p1, p2, count) = if args.len() >= 3 {
                (num(&args[0])?, num(&args[1])?, num(&args[2])?)
            } else {
                (num(&args[0])?, num(&args[1])?, 1.0)
            };
            let mapped: Vec<Value> = vec![
                Value::Scalar(count),
                Value::Scalar(p1),
                Value::Scalar(p2),
            ];
            let target = if name == "unifrnd" { "runif" } else { "rnorm" };
            crate::builtins_ext::call(target, &mapped, ev, env)
                .unwrap_or_else(|| Err(EvalError::UnknownFunction(target.into())))
        }
        "maximize" | "newton" | "root" => {
            crate::builtins_ext::call(name, args, ev, env).unwrap_or_else(|| Err(EvalError::UnknownFunction(name.into())))
        }
        "interp1" => crate::builtins::call("interp1", args, ev, env),
        "polyfit" => crate::linalg::polyfit(&vec_of(&args[0])?, &vec_of(&args[1])?, num(&args[2])? as usize).map(Value::Vector),
        "polyval" => {
            need(name, args.len(), 2)?;
            let c = vec_of(&args[0])?;
            let x = num(&args[1])?;
            let mut acc = 0.0;
            for k in 0..c.len() { acc += c[k] * x.powi((c.len() - 1 - k) as i32); }
            Ok(Value::Scalar(acc))
        }
        "polyder" | "polyint" => {
            need(name, args.len(), 1)?;
            let c = vec_of(&args[0])?;
            let n = c.len();
            if name == "polyder" {
                if n <= 1 { return Ok(Value::Vector(vec![0.0])); }
                Ok(Value::Vector((0..n - 1).map(|i| c[i] * (n - 1 - i) as f64).collect()))
            } else {
                let k = if args.len() > 1 { num(&args[1])? } else { 0.0 };
                let mut out: Vec<f64> = (0..n).map(|i| c[i] / (n - i) as f64).collect();
                out.push(k);
                Ok(Value::Vector(out))
            }
        }
        "conv" => {
            need(name, args.len(), 2)?;
            let (a, b) = (vec_of(&args[0])?, vec_of(&args[1])?);
            let mut o = vec![0.0; a.len() + b.len() - 1];
            for i in 0..a.len() { for j in 0..b.len() { o[i + j] += a[i] * b[j]; } }
            Ok(Value::Vector(o))
        }
        "deconv" => {
            need(name, args.len(), 2)?;
            let mut r = vec_of(&args[0])?;
            let b = vec_of(&args[1])?;
            if b.is_empty() || b[0] == 0.0 { return Err(EvalError::Math("deconv: divisor leading term is zero".into())); }
            if r.len() < b.len() { return Ok(Value::Vector(vec![0.0])); }
            // Polynomial long division, highest power first (MATLAB deconv).
            let nq = r.len() - b.len() + 1;
            let mut q = vec![0.0; nq];
            for i in 0..nq {
                q[i] = r[i] / b[0];
                for j in 0..b.len() { r[i + j] -= q[i] * b[j]; }
            }
            Ok(Value::Vector(q))
        }
        "trapz" | "cumtrapz" => {
            need(name, args.len(), 1)?;
            let (y, dx) = if args.len() > 1 { (vec_of(&args[1])?, num(&args[0])?) } else { (vec_of(&args[0])?, 1.0) };
            if y.len() < 2 { return Ok(Value::Scalar(0.0)); }
            let mut acc = 0.0;
            let mut out = Vec::with_capacity(y.len());
            out.push(0.0);
            for i in 1..y.len() { acc += 0.5 * dx * (y[i] + y[i - 1]); out.push(acc); }
            Ok(if name == "trapz" { Value::Scalar(acc) } else { Value::Vector(out) })
        }
        "gradient" | "diff" => {
            need(name, args.len(), 1)?;
            let v = vec_of(&args[0])?;
            let h = if args.len() > 1 { num(&args[1])? } else { 1.0 };
            if v.len() < 2 { return Ok(Value::Vector(vec![])); }
            let out = if name == "diff" {
                (1..v.len()).map(|i| v[i] - v[i - 1]).collect()
            } else {
                let mut g = Vec::with_capacity(v.len());
                g.push((v[1] - v[0]) / h);
                for i in 1..v.len() - 1 { g.push((v[i + 1] - v[i - 1]) / (2.0 * h)); }
                g.push((v[v.len() - 1] - v[v.len() - 2]) / h);
                g
            };
            Ok(Value::Vector(out))
        }
        "nextpow2" => { need(name, args.len(), 1)?; let x = num(&args[0])?.abs(); Ok(Value::Scalar(if x <= 1.0 { 0.0 } else { (x.log2().ceil()) as f64 })) }
        "fftshift" => {
            need(name, args.len(), 1)?;
            let v = vec_of(&args[0])?;
            let n = v.len();
            let h = (n as f64 / 2.0).ceil() as usize;
            let mut o = v[h..].to_vec();
            o.extend_from_slice(&v[..h]);
            Ok(Value::Vector(o))
        }
        "sum" | "prod" | "max" | "min" | "mean" | "median" | "std" | "var" | "rms" | "geomean" | "harmmean" => {
            need(name, args.len(), 1)?;
            let f = reducer(name);
            match &args[0] {
                Value::Matrix(m) if args.len() == 1 => Ok(Value::Vector((0..m.cols).map(|c| f(&(0..m.rows).map(|r| m.get(r, c)).collect::<Vec<_>>())).collect())),
                Value::Matrix(m) => {
                    let dim = num(&args[1])? as usize;
                    if dim == 1 { Ok(Value::Vector((0..m.cols).map(|c| f(&(0..m.rows).map(|r| m.get(r, c)).collect::<Vec<_>>())).collect())) }
                    else { Ok(Value::Vector((0..m.rows).map(|r| f(&(0..m.cols).map(|c| m.get(r, c)).collect::<Vec<_>>())).collect())) }
                }
                _ => Ok(Value::Scalar(f(&vec_of(&args[0])?))),
            }
        }
        "cumsum" | "cumprod" => {
            need(name, args.len(), 1)?;
            let v = vec_of(&args[0])?;
            let mut acc = if name == "cumsum" { 0.0 } else { 1.0 };
            let mut o = Vec::with_capacity(v.len());
            for x in v { acc = if name == "cumsum" { acc + x } else { acc * x }; o.push(acc); }
            Ok(Value::Vector(o))
        }
        "string" | "num2str" => { need(name, args.len(), 1)?; Ok(Value::Str(args[0].display())) }
        "strlength" => { need(name, args.len(), 1)?; Ok(Value::Scalar(as_str(&args[0])?.chars().count() as f64)) }
        "str2num" => { need(name, args.len(), 1)?; Ok(Value::Scalar(as_str(&args[0])?.trim().parse::<f64>().unwrap_or(f64::NAN))) }
        "upper" => { need(name, args.len(), 1)?; Ok(Value::Str(as_str(&args[0])?.to_uppercase())) }
        "lower" => { need(name, args.len(), 1)?; Ok(Value::Str(as_str(&args[0])?.to_lowercase())) }
        "contains" => { need(name, args.len(), 2)?; Ok(Value::Bool(as_str(&args[0])?.contains(&as_str(&args[1])?))) }
        "isspace" => { need(name, args.len(), 1)?; let s = as_str(&args[0])?; Ok(Value::Bool(!s.is_empty() && s.chars().all(char::is_whitespace))) }
        "append" => { need(name, args.len(), 2)?; Ok(Value::Str(format!("{}{}", as_str(&args[0])?, as_str(&args[1])?))) }
        other => Err(EvalError::UnknownFunction(other.to_string())),
    }
}

fn as_str(v: &Value) -> Result<String, EvalError> {
    match v { Value::Str(s) => Ok(s.clone()), other => Ok(other.display()) }
}

fn reducer(name: &str) -> fn(&[f64]) -> f64 {
    match name {
        "sum" => |v| v.iter().sum(),
        "prod" => |v| v.iter().product(),
        "max" => |v| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        "min" => |v| v.iter().cloned().fold(f64::INFINITY, f64::min),
        "mean" => |v| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 },
        "median" => |v| { let mut w = v.to_vec(); w.sort_by(|a, b| a.partial_cmp(b).unwrap()); let n = w.len(); if n == 0 { 0.0 } else if n % 2 == 1 { w[n / 2] } else { (w[n / 2 - 1] + w[n / 2]) / 2.0 } },
        "std" => |v| var_of(v).sqrt(),
        "var" => var_of,
        "rms" => |v| if v.is_empty() { 0.0 } else { (v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64).sqrt() },
        "geomean" => |v| if v.is_empty() { 0.0 } else { (v.iter().map(|x| x.ln()).sum::<f64>() / v.len() as f64).exp() },
        _ => |v| if v.is_empty() { 0.0 } else { v.len() as f64 / v.iter().map(|x| 1.0 / x).sum::<f64>() },
    }
}

fn var_of(v: &[f64]) -> f64 {
    if v.len() < 2 { return 0.0; }
    let m = v.iter().sum::<f64>() / v.len() as f64;
    v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64
}

/// Column-major flattening (MATLAB storage order).
fn column_major(m: &Matrix) -> Vec<f64> {
    let mut v = Vec::with_capacity(m.rows * m.cols);
    for c in 0..m.cols { for r in 0..m.rows { v.push(m.get(r, c)); } }
    v
}

fn magic(n: usize) -> Matrix {
    if n == 0 { return Matrix::new(0, 0, vec![]); }
    if n == 1 { return Matrix::new(1, 1, vec![1.0]); }
    if n == 2 { return Matrix::new(2, 2, vec![1.0, 3.0, 4.0, 2.0]); } // no 2x2 magic square exists
    let mut d = vec![0.0f64; n * n];
    if n % 2 == 1 {
        // Siamese (de la Loubere) method for odd orders.
        let (mut r, mut c) = (0usize, n / 2);
        for k in 1..=(n * n) {
            d[r * n + c] = k as f64;
            let (nr, nc) = ((r + n - 1) % n, (c + 1) % n);
            if d[nr * n + nc] != 0.0 { r = (r + 1) % n; } else { r = nr; c = nc; }
        }
    } else if n % 4 == 0 {
        // Doubly-even: complement pattern.
        for i in 0..n { for j in 0..n {
            let pattern = (i % 4 == j % 4) || ((i % 4) + (j % 4) == 3);
            let idx = i * n + j;
            d[idx] = if pattern { (n * n - idx) as f64 } else { (idx + 1) as f64 };
        } }
    } else {
        // Singly-even: Strachey method. Build four odd-order quadrants
        // A C / D B with offsets 0, 2h^2, 3h^2, h^2, then swap columns.
        let h = n / 2;
        let sub = magic(h);
        let hh = (h * h) as f64;
        for i in 0..h { for j in 0..h {
            let v = sub.get(i, j);
            d[i * n + j] = v;                       // A
            d[i * n + (j + h)] = v + 2.0 * hh;      // C
            d[(i + h) * n + j] = v + 3.0 * hh;      // D
            d[(i + h) * n + (j + h)] = v + hh;      // B
        } }
        let k = (n - 2) / 4;
        // Swap the first k columns between A and D, except the middle row
        // where the swap is shifted one column right.
        for i in 0..h {
            for j in 0..k {
                let col = if i == h / 2 { j + 1 } else { j };
                let a = i * n + col;
                let b = (i + h) * n + col;
                d.swap(a, b);
            }
        }
        // Swap the last k-1 columns between C and B.
        if k > 1 {
            for i in 0..h {
                for j in (n - k + 1)..n {
                    let a = i * n + j;
                    let b = (i + h) * n + j;
                    d.swap(a, b);
                }
            }
        }
    }
    Matrix::new(n, n, d)
}

/// Scaling-and-squaring matrix exponential with a Taylor/Pade-style series.
fn expm(m: &Matrix) -> Result<Matrix, EvalError> {
    if m.rows != m.cols { return Err(EvalError::Args("expm requires a square matrix".into())); }
    let a = to_dm(m);
    let nrm = maxabs(&a);
    let s = ((nrm / 5.4).max(1.0).log2()).ceil() as i32;
    let f = &a / 2.0f64.powi(s);
    let mut term = DMatrix::identity(m.rows, m.cols);
    let mut sum = term.clone();
    for k in 1..=30 {
        let fk = &f / k as f64;
        term = &term * &fk;
        sum = &sum + &term;
        if maxabs(&term) < 1e-16 { break; }
    }
    let mut out = sum;
    for _ in 0..s { out = &out * &out; }
    Ok(from_dm(&out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_no_duplicate_names() {
        for i in 0..CATALOG.len() {
            for j in (i + 1)..CATALOG.len() {
                assert_ne!(CATALOG[i].name, CATALOG[j].name, "duplicate catalog entry");
            }
        }
    }

    #[test]
    fn magic_odd_and_doubly_even_sum() {
        for n in [3usize, 4] {
            let m = magic(n);
            let target = n as f64 * (n * n + 1) as f64 / 2.0;
            let row0: f64 = (0..n).map(|c| m.get(0, c)).sum();
            assert!((row0 - target).abs() < 1e-9, "magic({n}) row sum");
        }
    }

    #[test]
    fn expm_of_zero_is_identity() {
        let z = Matrix::new(2, 2, vec![0.0; 4]);
        let e = expm(&z).unwrap();
        assert!((e.get(0, 0) - 1.0).abs() < 1e-12 && e.get(0, 1).abs() < 1e-12);
    }

    #[test]
    fn reshape_is_column_major_like_matlab() {
        let a = Matrix::new(3, 4, (1..=12).map(|x| x as f64).collect());
        let env = &mut Environment::new();
        let ev = &mut Evaluator::new(Default::default());
        let b = invoke("reshape", &[Value::Matrix(a), Value::Scalar(2.0), Value::Scalar(6.0)], ev, env).unwrap();
        match b {
            Value::Matrix(m) => {
                // MATLAB: column-major linear order is preserved, matching
                // np.reshape(a.ravel(order='F'), (2, 6), order='F').
                let row0: Vec<f64> = (0..6).map(|c| m.get(0, c)).collect();
                let row1: Vec<f64> = (0..6).map(|c| m.get(1, c)).collect();
                assert_eq!(row0, vec![1.0, 9.0, 6.0, 3.0, 11.0, 8.0]);
                assert_eq!(row1, vec![5.0, 2.0, 10.0, 7.0, 4.0, 12.0]);
            }
            _ => panic!("matrix expected"),
        }
    }
}

#[cfg(test)]
mod parity_tests {
    use super::*;

    fn nat(name: &str, args: &[Value]) -> Value {
        let mut e = Evaluator::new(Default::default());
        let mut env = Environment::new();
        invoke(name, args, &mut e, &mut env)
            .unwrap_or_else(|err| panic!("{name} failed: {err}"))
    }
    fn sc(name: &str, args: &[Value]) -> f64 {
        match nat(name, args) { Value::Scalar(x) => x, _ => panic!("{name}: scalar expected") }
    }
    fn vv(name: &str, args: &[Value]) -> Vec<f64> {
        match nat(name, args) {
            Value::Vector(a) => a, Value::Matrix(m) => m.data, Value::Scalar(x) => vec![x],
            _ => panic!("{name}: array expected"),
        }
    }
    fn mm(name: &str, args: &[Value]) -> Matrix {
        match nat(name, args) { Value::Matrix(m) => m, Value::Vector(a) => Matrix::new(a.len(),1,a), _ => panic!("{name}: matrix expected") }
    }
    fn bl(name: &str, args: &[Value]) -> bool {
        match nat(name, args) { Value::Bool(b) => b, _ => panic!("{name}: bool expected") }
    }
    fn m2(r: usize, c: usize, d: &[f64]) -> Value { Value::Matrix(Matrix::new(r, c, d.to_vec())) }
    fn v1(d: &[f64]) -> Value { Value::Vector(d.to_vec()) }
    fn near(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }
    fn nearv(a: &[f64], b: &[f64]) -> bool { a.len() == b.len() && a.iter().zip(b).all(|(x, y)| near(*x, *y)) }

    #[test] fn zeros_and_ones() {
        let z = mm("zeros", &[Value::Scalar(2.0), Value::Scalar(3.0)]);
        assert_eq!((z.rows, z.cols), (2, 3)); assert!(z.data.iter().all(|v| *v == 0.0));
        assert_eq!(mm("zeros", &[Value::Scalar(4.0)]).rows, 4);
        assert!(mm("ones", &[Value::Scalar(2.0), Value::Scalar(2.0)]).data.iter().all(|v| *v == 1.0));
    }
    #[test] fn eye_identity() {
        let e = mm("eye", &[Value::Scalar(3.0)]);
        for r in 0..3 { for c in 0..3 { assert!(near(e.get(r, c), if r == c { 1.0 } else { 0.0 })); } }
    }
    #[test] fn magic_exact_3_and_4() {
        let m = mm("magic", &[Value::Scalar(3.0)]);
        let r0: Vec<f64> = (0..3).map(|c| m.get(0, c)).collect();
        let r2: Vec<f64> = (0..3).map(|c| m.get(2, c)).collect();
        assert!(nearv(&r0, &[8.0, 1.0, 6.0]));
        assert!(nearv(&r2, &[4.0, 9.0, 2.0]));
        let f = mm("magic", &[Value::Scalar(4.0)]);
        let g0: Vec<f64> = (0..4).map(|c| f.get(0, c)).collect();
        let g3: Vec<f64> = (0..4).map(|c| f.get(3, c)).collect();
        assert!(nearv(&g0, &[16.0, 2.0, 3.0, 13.0]));
        assert!(nearv(&g3, &[4.0, 14.0, 15.0, 1.0]));
    }
    #[test] fn magic_all_rows_and_cols_sum() {
        for n in [3usize, 4, 5] {
            let m = mm("magic", &[Value::Scalar(n as f64)]);
            let target = n as f64 * (n * n + 1) as f64 / 2.0;
            for i in 0..n {
                let row: f64 = (0..n).map(|c| m.get(i, c)).sum();
                let col: f64 = (0..n).map(|r| m.get(r, i)).sum();
                assert!(near(row, target) && near(col, target), "magic({n}) line {i}");
            }
        }
    }
    #[test] fn random_shapes_and_ranges() {
        let r = mm("rand", &[Value::Scalar(2.0), Value::Scalar(2.0)]);
        assert!(r.data.iter().all(|v| *v >= 0.0 && *v < 1.0));
        let n = mm("randn", &[Value::Scalar(4000.0), Value::Scalar(1.0)]).data;
        let mean = n.iter().sum::<f64>() / n.len() as f64;
        let sd = (n.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n.len() as f64).sqrt();
        assert!(mean.abs() < 0.1 && (sd - 1.0).abs() < 0.15, "randn stats {mean} {sd}");
        assert!(vv("randi", &[Value::Scalar(10.0), Value::Scalar(6.0)]).iter().all(|v| *v >= 1.0 && *v <= 10.0));
        let mut p = vv("randperm", &[Value::Scalar(6.0)]); p.sort_by(|a,b|a.partial_cmp(b).unwrap());
        assert!(nearv(&p, &[1.0,2.0,3.0,4.0,5.0,6.0]));
    }
    #[test] fn rng_is_reproducible() {
        nat("rng", &[Value::Scalar(7.0)]);
        let a = vv("rand", &[Value::Scalar(5.0), Value::Scalar(1.0)]);
        nat("rng", &[Value::Scalar(7.0)]);
        let b = vv("rand", &[Value::Scalar(5.0), Value::Scalar(1.0)]);
        assert!(nearv(&a, &b));
    }
    #[test] fn linspace_logspace_colon() {
        assert!(nearv(&vv("linspace", &[Value::Scalar(0.0),Value::Scalar(1.0),Value::Scalar(5.0)]), &[0.0,0.25,0.5,0.75,1.0]));
        assert!(nearv(&vv("logspace", &[Value::Scalar(0.0),Value::Scalar(2.0),Value::Scalar(3.0)]), &[1.0,10.0,100.0]));
        assert!(nearv(&vv("colon", &[Value::Scalar(1.0),Value::Scalar(3.0)]), &[1.0,2.0,3.0]));
        assert!(nearv(&vv("colon", &[Value::Scalar(0.0),Value::Scalar(2.0),Value::Scalar(10.0)]), &[0.0,2.0,4.0,6.0,8.0,10.0]));
        assert!(nearv(&vv("colon", &[Value::Scalar(5.0),Value::Scalar(-1.0),Value::Scalar(1.0)]), &[5.0,4.0,3.0,2.0,1.0]));
    }
    #[test] fn reshape_is_column_major() {
        let a = m2(3, 4, &(1..=12).map(|x| x as f64).collect::<Vec<_>>());
        let b = mm("reshape", &[a.clone(), Value::Scalar(2.0), Value::Scalar(6.0)]);
        assert!(nearv(&(0..6).map(|c| b.get(0,c)).collect::<Vec<_>>(), &[1.0,9.0,6.0,3.0,11.0,8.0]));
        assert!(nearv(&(0..6).map(|c| b.get(1,c)).collect::<Vec<_>>(), &[5.0,2.0,10.0,7.0,4.0,12.0]));
    }
    #[test] fn diag_trace() {
        let d = mm("diag", &[v1(&[1.0,2.0,3.0])]);
        assert_eq!((d.rows,d.cols),(3,3)); assert!(near(d.get(1,1),2.0) && near(d.get(0,2),0.0));
        assert!(nearv(&vv("diag",&[m2(2,2,&[1.0,2.0,3.0,4.0])]), &[1.0,4.0]));
        assert!(near(sc("trace",&[m2(2,2,&[1.0,2.0,3.0,4.0])]), 5.0));
    }
    #[test] fn flips_rot_circ_repmat() {
        assert!(nearv(&mm("flipud",&[m2(2,2,&[1.0,2.0,3.0,4.0])]).data, &[3.0,4.0,1.0,2.0]));
        assert!(nearv(&mm("fliplr",&[m2(2,2,&[1.0,2.0,3.0,4.0])]).data, &[2.0,1.0,4.0,3.0]));
        assert!(nearv(&mm("rot90",&[m2(2,2,&[1.0,2.0,3.0,4.0])]).data, &[2.0,4.0,1.0,3.0]));
        assert!(nearv(&mm("rot90",&[m2(2,2,&[1.0,2.0,3.0,4.0]),Value::Scalar(2.0)]).data, &[4.0,3.0,2.0,1.0]));
        assert!(nearv(&mm("circshift",&[m2(2,3,&[1.0,2.0,3.0,4.0,5.0,6.0]),Value::Scalar(1.0)]).data, &[3.0,1.0,2.0,6.0,4.0,5.0]));
        let rp = mm("repmat",&[m2(1,2,&[1.0,2.0]),Value::Scalar(2.0),Value::Scalar(3.0)]);
        assert_eq!((rp.rows,rp.cols),(2,6)); assert!(nearv(&rp.data, &[1.0,2.0,1.0,2.0,1.0,2.0,1.0,2.0,1.0,2.0,1.0,2.0]));
    }
    #[test] fn kron_cat_size_meta() {
        let k = mm("kron",&[m2(1,2,&[1.0,2.0]), m2(2,2,&[1.0,0.0,0.0,1.0])]);
        assert_eq!((k.rows,k.cols),(2,4)); assert!(near(k.get(0,2),2.0) && near(k.get(1,3),2.0) && near(k.get(0,0),1.0));
        assert_eq!(mm("vertcat",&[m2(1,2,&[1.0,2.0]),m2(1,2,&[3.0,4.0])]).rows, 2);
        assert_eq!(mm("horzcat",&[m2(1,2,&[1.0,2.0]),m2(1,2,&[3.0,4.0])]).cols, 4);
        assert!(nearv(&vv("size",&[m2(3,4,&[0.0;12])]), &[3.0,4.0]));
        assert!(near(sc("numel",&[m2(3,4,&[0.0;12])]), 12.0));
        assert!(near(sc("length",&[m2(3,4,&[0.0;12])]), 4.0));
        assert!(near(sc("ndims",&[m2(2,2,&[0.0;4])]), 2.0));
        assert!(bl("isempty",&[v1(&[])]));
    }
    #[test] fn sort_find_unique_ismember() {
        assert!(nearv(&vv("sort",&[v1(&[3.0,1.0,2.0])]), &[1.0,2.0,3.0]));
        assert!(nearv(&mm("sort",&[m2(2,2,&[3.0,1.0,2.0,4.0])]).data, &[2.0,1.0,3.0,4.0]));
        assert!(nearv(&mm("sortrows",&[m2(2,2,&[3.0,1.0,1.0,2.0])]).data, &[1.0,2.0,3.0,1.0]));
        assert!(nearv(&vv("find",&[v1(&[0.0,2.0,0.0,4.0])]), &[2.0,4.0]));
        assert!(nearv(&vv("unique",&[v1(&[1.0,2.0,2.0,3.0,1.0])]), &[1.0,2.0,3.0]));
        assert!(nearv(&vv("ismember",&[v1(&[1.0,2.0,3.0]),v1(&[2.0,3.0,4.0])]), &[0.0,1.0,1.0]));
    }
    #[test] fn indexing_helpers() {
        assert!(nearv(&vv("row",&[m2(2,2,&[1.0,2.0,3.0,4.0]),Value::Scalar(0.0)]), &[1.0,2.0]));
        assert!(nearv(&vv("col",&[m2(2,2,&[1.0,2.0,3.0,4.0]),Value::Scalar(0.0)]), &[1.0,3.0]));
        let a = m2(3,3,&[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0]);
        assert!(nearv(&mm("submatrix",&[a,Value::Scalar(0.0),Value::Scalar(1.0),Value::Scalar(1.0),Value::Scalar(2.0)]).data, &[2.0,3.0,5.0,6.0]));
    }
    #[test] fn expm_nilpotent_and_linalg() {
        let e = mm("expm",&[m2(2,2,&[0.0,1.0,0.0,0.0])]);
        assert!(near(e.get(0,0),1.0) && near(e.get(0,1),1.0) && near(e.get(1,0),0.0) && near(e.get(1,1),1.0));
        let x = mm("solve",&[m2(2,2,&[2.0,1.0,1.0,3.0]), v1(&[5.0,10.0])]);
        assert!(near(x.get(0,0),1.0) && near(x.get(1,0),3.0));
        let iv = mm("inv",&[m2(2,2,&[2.0,0.0,0.0,4.0])]);
        assert!(near(iv.get(0,0),0.5) && near(iv.get(1,1),0.25));
        assert!(near(sc("det",&[m2(2,2,&[1.0,2.0,3.0,4.0])]), -2.0));
        assert!(near(sc("rank",&[m2(2,2,&[1.0,2.0,2.0,4.0])]), 1.0));
        assert!(near(sc("norm",&[v1(&[3.0,4.0])]), 5.0));
        let mut w = vv("eigenvals",&[m2(2,2,&[3.0,0.0,0.0,1.0])]); w.sort_by(|a,b|a.partial_cmp(b).unwrap());
        assert!(nearv(&w, &[1.0,3.0]));
    }
    #[test] fn random_dist_wrappers() {
        // MATLAB order: unifrnd(a, b, n), normrnd(mu, sigma, n).
        let u = vv("unifrnd", &[Value::Scalar(0.0), Value::Scalar(1.0), Value::Scalar(10.0)]);
        assert_eq!(u.len(), 10);
        assert!(u.iter().all(|v| *v >= 0.0 && *v <= 1.0));
        let n = vv("normrnd", &[Value::Scalar(0.0), Value::Scalar(1.0), Value::Scalar(8.0)]);
        assert_eq!(n.len(), 8);
        // Two-argument form yields a single sample.
        assert_eq!(vv("unifrnd", &[Value::Scalar(0.0), Value::Scalar(1.0)]).len(), 1);
    }
    #[test] fn poly_and_interp() {
        assert!(near(sc("interp1",&[v1(&[0.0,1.0,2.0]),v1(&[0.0,1.0,4.0]),Value::Scalar(1.5)]), 2.5));
        assert!(nearv(&vv("polyfit",&[v1(&[0.0,1.0,2.0]),v1(&[0.0,1.0,4.0]),Value::Scalar(2.0)]), &[1.0,0.0,0.0]));
        assert!(near(sc("polyval",&[v1(&[1.0,0.0,0.0]),Value::Scalar(3.0)]), 9.0));
        assert!(nearv(&vv("polyder",&[v1(&[1.0,0.0,0.0])]), &[2.0,0.0]));
        assert!(nearv(&vv("polyint",&[v1(&[2.0,0.0]),Value::Scalar(0.0)]), &[1.0,0.0,0.0]));
    }
    #[test] fn conv_deconv_trapezoid_grad() {
        assert!(nearv(&vv("conv",&[v1(&[1.0,2.0]),v1(&[3.0,4.0])]), &[3.0,10.0,8.0]));
        assert!(nearv(&vv("deconv",&[v1(&[3.0,10.0,8.0]),v1(&[1.0,2.0])]), &[3.0,4.0]));
        assert!(near(sc("trapz",&[Value::Scalar(1.0),v1(&[1.0,2.0,3.0])]), 4.0));
        assert!(nearv(&vv("cumtrapz",&[Value::Scalar(1.0),v1(&[1.0,2.0,3.0])]), &[0.0,1.5,4.0]));
        assert!(nearv(&vv("gradient",&[v1(&[1.0,4.0,9.0])]), &[3.0,4.0,5.0]));
        assert!(nearv(&vv("diff",&[v1(&[1.0,4.0,9.0])]), &[3.0,5.0]));
        assert!(near(sc("nextpow2",&[Value::Scalar(5.0)]), 3.0));
        assert!(near(sc("nextpow2",&[Value::Scalar(1.0)]), 0.0));
        assert!(nearv(&vv("fftshift",&[v1(&[1.0,2.0,3.0,4.0])]), &[3.0,4.0,1.0,2.0]));
    }
    #[test] fn reductions_with_dim() {
        let m = m2(2,2,&[1.0,2.0,3.0,4.0]);
        assert!(nearv(&vv("sum",&[m.clone()]), &[4.0,6.0]));            // default: down columns
        assert!(nearv(&vv("sum",&[m.clone(),Value::Scalar(1.0)]), &[4.0,6.0]));
        assert!(nearv(&vv("sum",&[m.clone(),Value::Scalar(2.0)]), &[3.0,7.0]));  // along rows
        assert!(nearv(&vv("mean",&[m.clone()]), &[2.0,3.0]));
        assert!(near(sc("max",&[v1(&[1.0,5.0,3.0])]), 5.0));
        assert!(near(sc("min",&[v1(&[1.0,5.0,3.0])]), 1.0));
        assert!(near(sc("prod",&[v1(&[2.0,3.0,4.0])]), 24.0));
        assert!(near(sc("median",&[v1(&[1.0,2.0,3.0,4.0])]), 2.5));
        assert!(near(sc("var",&[v1(&[1.0,2.0,3.0])]), 1.0));            // sample variance
        assert!(near(sc("std",&[v1(&[1.0,2.0,3.0])]), 1.0));
        assert!(near(sc("rms",&[v1(&[3.0,4.0])]), (12.5f64).sqrt()));
        assert!(near(sc("geomean",&[v1(&[1.0,4.0])]), 2.0));
        assert!(near(sc("harmmean",&[v1(&[1.0,4.0])]), 1.6));
        assert!(nearv(&vv("cumsum",&[v1(&[1.0,2.0,3.0])]), &[1.0,3.0,6.0]));
        assert!(nearv(&vv("cumprod",&[v1(&[1.0,2.0,3.0])]), &[1.0,2.0,6.0]));
    }
    #[test] fn string_operations() {
        match nat("upper",&[Value::Str("abc".into())]) { Value::Str(s)=>assert_eq!(s,"ABC"), _=>panic!() }
        match nat("lower",&[Value::Str("ABC".into())]) { Value::Str(s)=>assert_eq!(s,"abc"), _=>panic!() }
        assert!(near(sc("strlength",&[Value::Str("hello".into())]), 5.0));
        assert!(bl("contains",&[Value::Str("hello".into()),Value::Str("ell".into())]));
        assert!(bl("isspace",&[Value::Str("  \t".into())]));
        match nat("append",&[Value::Str("a".into()),Value::Str("b".into())]) { Value::Str(s)=>assert_eq!(s,"ab"), _=>panic!() }
        assert!(near(sc("str2num",&[Value::Str("3.5".into())]), 3.5));
    }
    // ---- routing contract, isolated through functions::call ----
    fn routed(name: &str) {
        let mut e = Evaluator::new(Default::default());
        let mut env = Environment::new();
        match call(name, &[], &mut e, &mut env) {
            Some(Err(EvalError::RequiresPython(_))) => {}
            _ => panic!("{name} must route to embedded Python"),
        }
    }
    #[test] fn python_routed_names_require_python() {
        for n in ["lu","qr","chol","svd","eig","cond","sqrtm","logm","fft","ifft","fft2",
                  "spline","pchip","csaps","gmres","lsqr","lsmr","bicgstab","ode45","ode15s",
                  "bvp4c","bvp5c","dde23","pdepe","fminsearch","fmincon","fsolve","fzero",
                  "roots","delaunay","tsearchn","table","regexp","split","sprintf"] { routed(n); }
    }
    #[test] fn native_names_do_not_require_python() {
        let mut e = Evaluator::new(Default::default());
        let mut env = Environment::new();
        for n in ["magic","eye","zeros","kron","colon","linspace","expm","det","inv","solve","sum","sort","upper"] {
            match call(n, &[Value::Scalar(2.0)], &mut e, &mut env) {
                Some(Err(EvalError::RequiresPython(_))) => panic!("{n} should be native"),
                _ => {}
            }
        }
    }
    #[test] fn unknown_name_falls_through() {
        let mut e = Evaluator::new(Default::default());
        let mut env = Environment::new();
        assert!(call("frobnicate", &[], &mut e, &mut env).is_none());
        assert!(!is_known("frobnicate"));
    }
    #[test] fn catalog_flags_are_consistent() {
        assert!(!spec("magic").unwrap().python);
        assert!(spec("lu").unwrap().python);
        assert!(spec("ode45").unwrap().python);
        assert!(is_known("pdepe") && is_known("gmres") && is_known("fftshift"));
    }
}
