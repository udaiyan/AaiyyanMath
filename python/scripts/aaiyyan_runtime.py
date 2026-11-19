# AaiyyanMath embedded runtime support module.
# Loaded by the Rust PyO3 worker; also importable by exported scripts.
# Rust injects cancel_requested() into this module when embedded.

import contextlib
import io
import json
import math
import sys
import time
import traceback


class AaiyyanCancelled(BaseException):
    pass


_GLOBALS = {"__name__": "aaiyyan_session"}
_VALUES = []
_TICKS = 0


def _cancel_flag():
    try:
        return bool(cancel_requested())
    except NameError:
        return False


def _tracer(frame, event, arg):
    global _TICKS
    _TICKS += 1
    if _TICKS % 256 == 0 and _cancel_flag():
        raise AaiyyanCancelled()
    return _tracer


def reset():
    _GLOBALS.clear()
    _GLOBALS["__name__"] = "aaiyyan_session"


def report(name, value):
    _VALUES.append(_serialize(str(name), value))


def report_expr(value):
    report("_expr", value)


def _serialize(name, value):
    out = {"name": name, "kind": "unknown", "display": str(value)}
    try:
        import pint
        if isinstance(value, pint.Quantity):
            base = value.to_base_units()
            out.update({
                "kind": "quantity",
                "si": float(base.magnitude),
                "number": float(value.magnitude),
                "unit": str(value.units),
                "display": "{:.6g} {}".format(float(value.magnitude), value.units),
            })
            return out
    except Exception:
        pass
    try:
        import sympy as sp
        if isinstance(value, sp.Basic):
            out.update({"kind": "symbolic", "text": str(value), "latex": sp.latex(value), "display": str(value)})
            return out
    except Exception:
        pass
    try:
        import pandas as pd
        if isinstance(value, pd.DataFrame):
            out.update({
                "kind": "table",
                "columns": [str(c) for c in value.columns],
                "rows": [[str(c) for c in row] for row in value.head(500).itertuples(index=False)],
                "display": "DataFrame {}x{}".format(len(value), len(value.columns)),
            })
            return out
        if isinstance(value, pd.Series):
            out.update({"kind": "vector", "data": [float(x) for x in value.to_numpy().ravel()], "shape": [int(value.size)]})
            return out
    except Exception:
        pass
    try:
        import numpy as np
        if isinstance(value, np.ndarray):
            flat = [float(x) for x in value.ravel()]
            out.update({
                "kind": "matrix" if value.ndim == 2 else "vector",
                "shape": [int(s) for s in value.shape],
                "data": flat,
                "display": "array {}".format(value.shape),
            })
            return out
        if isinstance(value, np.generic):
            value = value.item()
    except Exception:
        pass
    if isinstance(value, bool):
        out.update({"kind": "bool", "number": 1.0 if value else 0.0, "display": str(value)})
    elif isinstance(value, (int, float)):
        out.update({"kind": "scalar", "number": float(value), "display": "{:.6g}".format(value)})
    elif isinstance(value, complex):
        out.update({"kind": "complex", "number": float(value.real), "data": [float(value.imag)], "display": str(value)})
    elif isinstance(value, str):
        out.update({"kind": "str", "text": value, "display": value})
    elif isinstance(value, (list, tuple)):
        try:
            out.update({"kind": "vector", "data": [float(x) for x in value], "shape": [len(value)]})
        except Exception:
            out.update({"kind": "str", "text": str(value)})
    elif callable(value):
        out.update({"kind": "function", "display": getattr(value, "__name__", "function") + "()"})
    return out


def execute(code):
    global _VALUES, _TICKS
    _VALUES = []
    _TICKS = 0
    stdout = io.StringIO()
    stderr = io.StringIO()
    ok = True
    error = None
    tb = None
    cancelled = False
    started = time.perf_counter()
    previous_trace = sys.gettrace()
    try:
        sys.settrace(_tracer)
    except Exception:
        pass
    try:
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            exec(compile(code, "<aaiyyan-worksheet>", "exec"), _GLOBALS)
    except AaiyyanCancelled:
        ok = False
        cancelled = True
        error = "calculation cancelled"
    except BaseException as exc:
        ok = False
        error = "{}: {}".format(type(exc).__name__, exc)
        tb = traceback.format_exc()
    finally:
        try:
            sys.settrace(previous_trace)
        except Exception:
            pass
    return json.dumps({
        "ok": ok,
        "values": _VALUES,
        "stdout": stdout.getvalue(),
        "stderr": stderr.getvalue(),
        "error": error,
        "traceback": tb,
        "duration_ms": (time.perf_counter() - started) * 1000.0,
        "cancelled": cancelled,
    })


def info():
    libs = []
    for name in ("numpy", "scipy", "sympy", "pandas", "matplotlib", "pint"):
        try:
            module = __import__(name)
            libs.append([name, getattr(module, "__version__", "unknown")])
        except Exception:
            libs.append([name, "missing"])
    return json.dumps({
        "version": sys.version.split()[0],
        "executable": sys.executable,
        "libraries": libs,
    })


def math_range(start, end, step=1.0):
    import numpy as np
    if step == 0:
        raise ValueError("range step cannot be zero")
    count = int(math.floor((end - start) / step)) + 1
    if count < 0 or count > 1000000:
        raise ValueError("range produces too many elements")
    return np.array([start + i * step for i in range(count)], dtype=float)


def numeric_derivative(f, x, h=None):
    if h is None:
        h = 1e-5 * (1.0 + abs(x))
    return (f(x + h) - f(x - h)) / (2.0 * h)


def ode_solve(f, t0, t1, y0, n=200, method="RK45", rtol=1e-6, atol=1e-9):
    import numpy as np
    from scipy.integrate import solve_ivp

    scalar = not hasattr(y0, "__len__")
    y_init = [float(y0)] if scalar else [float(v) for v in y0]

    def rhs(t, y):
        result = f(t, y[0] if scalar else y)
        return [float(result)] if scalar else [float(v) for v in result]

    times = np.linspace(t0, t1, int(n) + 1)
    solution = solve_ivp(rhs, (t0, t1), y_init, t_eval=times, method=method, rtol=rtol, atol=atol)
    if not solution.success:
        raise RuntimeError(solution.message)
    return np.column_stack([solution.t, solution.y.T])


def bvp_solve(fun, bc, x, y, tol=1e-4):
    import numpy as np
    from scipy.integrate import solve_bvp
    xs = np.linspace(x[0], x[1], int(x[2]) if len(x) > 2 else 50)
    ys = np.zeros((len(y), xs.size))
    solution = solve_bvp(fun, bc, xs, ys, tol=tol)
    if not solution.success:
        raise RuntimeError(solution.message)
    return np.column_stack([solution.x, solution.y.T])


def fit(f, xdata, ydata, p0):
    import numpy as np
    from scipy.optimize import curve_fit
    params, _covariance = curve_fit(f, np.asarray(xdata, dtype=float), np.asarray(ydata, dtype=float), p0=p0)
    return params


def read_table(path):
    import pandas as pd
    if str(path).lower().endswith((".xlsx", ".xls")):
        return pd.read_excel(path)
    return pd.read_csv(path)


def export_figure(series, path, title="", xlabel="x", ylabel="y"):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    fig, ax = plt.subplots(figsize=(7, 4))
    for label, xs, ys in series:
        ax.plot(xs, ys, label=label)
    ax.set_title(title)
    ax.set_xlabel(xlabel)
    ax.set_ylabel(ylabel)
    ax.grid(True, alpha=0.3)
    if series:
        ax.legend()
    fig.tight_layout()
    fig.savefig(path, dpi=150)
    plt.close(fig)
    return path


# --------------------------------------------------------------------------------------
# Extended function library: mirrors crates/evaluator/src/builtins_ext.rs so exported
# Python produces the same results as the native engine.
# --------------------------------------------------------------------------------------
import math as _math
import random as _random


def sec(x): return 1.0 / _math.cos(x)
def csc(x): return 1.0 / _math.sin(x)
def cot(x): return 1.0 / _math.tan(x)
def asec(x): return _math.acos(1.0 / x)
def acsc(x): return _math.asin(1.0 / x)
def acot(x): return _math.atan(1.0 / x)
def sech(x): return 1.0 / _math.cosh(x)
def csch(x): return 1.0 / _math.sinh(x)
def coth(x): return 1.0 / _math.tanh(x)
def sinc(x): return 1.0 if x == 0 else _math.sin(x) / x
def frac(x): return x - _math.trunc(x)
def clamp(x, lo, hi): return max(min(lo, hi), min(max(lo, hi), x))
def gcd(a, b): return _math.gcd(int(abs(a)), int(abs(b)))
def lcm(a, b): return abs(int(a) * int(b)) // max(1, _math.gcd(int(abs(a)), int(abs(b))))
def nroot(x, n): return -((-x) ** (1.0 / n)) if x < 0 and int(n) % 2 else x ** (1.0 / n)
def log1p(x): return _math.log1p(x)
def expm1(x): return _math.expm1(x)


def _arr(v):
    import numpy as np
    return np.asarray(v, dtype=float).ravel()


def geomean(v):
    a = _arr(v)
    return float(_math.exp(sum(_math.log(x) for x in a) / len(a)))


def harmmean(v):
    a = _arr(v)
    return float(len(a) / sum(1.0 / x for x in a))


def rms(v):
    import numpy as np
    return float(np.sqrt(np.mean(_arr(v) ** 2)))


def mode(v):
    a = list(_arr(v))
    return float(max(set(a), key=a.count))


def percentile(v, p):
    import numpy as np
    return float(np.percentile(_arr(v), p))


def corr(x, y):
    import numpy as np
    return float(np.corrcoef(_arr(x), _arr(y))[0, 1])


def cov(x, y):
    import numpy as np
    return float(np.cov(_arr(x), _arr(y))[0, 1])


def skew(v):
    import numpy as np
    a = _arr(v)
    s = a.std()
    return 0.0 if s == 0 else float(((a - a.mean()) ** 3).mean() / s ** 3)


def kurt(v):
    import numpy as np
    a = _arr(v)
    s = a.std()
    return 0.0 if s == 0 else float(((a - a.mean()) ** 4).mean() / s ** 4 - 3.0)


def augment(a, b):
    import numpy as np
    return np.hstack((np.atleast_2d(a), np.atleast_2d(b)))


def stack(a, b):
    import numpy as np
    return np.vstack((np.atleast_2d(a), np.atleast_2d(b)))


def submatrix(a, r0, r1, c0, c1):
    import numpy as np
    return np.atleast_2d(a)[int(r0):int(r1) + 1, int(c0):int(c1) + 1]


def row(a, i):
    import numpy as np
    return np.atleast_2d(a)[int(i), :]


def col(a, i):
    import numpy as np
    return np.atleast_2d(a)[:, int(i)]


def last(v): return float(_arr(v)[-1])
def vec(a): return _arr(a)


def reshape(v, r, c):
    import numpy as np
    return _arr(v).reshape(int(r), int(c))


def cumsum(v):
    import numpy as np
    return np.cumsum(_arr(v))


def cumprod(v):
    import numpy as np
    return np.cumprod(_arr(v))


def logspace(a, b, n):
    import numpy as np
    return np.logspace(a, b, int(n))


def linterp(x, y, xq):
    import numpy as np
    return float(np.interp(xq, _arr(x), _arr(y)))


def linfit(x, y):
    import numpy as np
    m, b = np.polyfit(_arr(x), _arr(y), 1)
    return np.array([m, b])


def expfit(x, y):
    import numpy as np
    m, b = np.polyfit(_arr(x), np.log(_arr(y)), 1)
    return np.array([_math.exp(b), m])


def logfit(x, y):
    import numpy as np
    m, b = np.polyfit(np.log(_arr(x)), _arr(y), 1)
    return np.array([m, b])


def pwrfit(x, y):
    import numpy as np
    m, b = np.polyfit(np.log(_arr(x)), np.log(_arr(y)), 1)
    return np.array([_math.exp(b), m])


def maximize(f, a, b):
    from scipy.optimize import minimize_scalar
    return float(minimize_scalar(lambda x: -f(x), bounds=(a, b), method="bounded").x)


def newton(f, x0):
    from scipy.optimize import newton as _newton
    return float(_newton(f, x0))


def seed(n):
    _random.seed(int(n))
    return float(n)


def runif(n, a, b):
    import numpy as np
    return np.array([_random.uniform(a, b) for _ in range(int(n))])


def rnorm(n, mu, sigma):
    import numpy as np
    return np.array([_random.gauss(mu, sigma) for _ in range(int(n))])


def unitsOf(x): return str(x.units)
def SIUnitsOf(x): return float(x.to_base_units().magnitude)


# --------------------------------------------------------------------------------------
# MATLAB-parity runtime: solvers + aaiyyan.mcall dispatch used by generated Python.
# Mirrors crates/evaluator/src/functions.rs; heavy numerics use SciPy/NumPy.
# --------------------------------------------------------------------------------------
import math as _m


def _N():
    import numpy as np
    return np


def _SI():
    import scipy.integrate as m
    return m


def _SL():
    import scipy.linalg as m
    return m


def _SO():
    import scipy.optimize as m
    return m


def _SP():
    import scipy.interpolate as m
    return m


def _SS():
    import scipy.spatial as m
    return m


def _SG():
    import scipy.signal as m
    return m


def _SK():
    import scipy.sparse.linalg as m
    return m


def _sz(*a):
    if len(a) == 1:
        n = int(a[0])
        return (n, n)
    return tuple(int(x) for x in a)


def magic(n):
    np = _N()
    n = int(n)
    M = np.zeros((n, n))
    if n % 2 == 1:
        i, j = 0, n // 2
        for k in range(1, n * n + 1):
            M[i, j] = k
            si, sj = (i - 1) % n, (j + 1) % n
            if M[si, sj]:
                i = (i + 1) % n
            else:
                i, j = si, sj
    elif n % 4 == 0:
        idx = np.arange(n * n).reshape(n, n)
        ii, jj = np.meshgrid(np.arange(n), np.arange(n), indexing='ij')
        pat = (ii % 4 == jj % 4) | ((ii % 4 + jj % 4) == 3)
        M = np.where(pat, n * n - idx, idx + 1).astype(float)
    else:
        h = n // 2
        A = magic(h)
        M[:h, :h] = A
        M[:h, h:] = A + 3 * h * h
        M[h:, :h] = A + h * h
        M[h:, h:] = A + 2 * h * h
        k = (n - 2) // 4
        cols = list(range(k)) + list(range(n - k + 1, n))
        tmp = M[:h, cols].copy()
        M[:h, cols] = M[h:, cols]
        M[h:, cols] = tmp
        r = h // 2
        M[r, k], M[h + r, k] = M[h + r, k], M[r, k]
        M[r, h + r], M[h + r, h + r] = M[h + r, h + r], M[r, h + r]
    return M


def expm(a):
    return _SL().expm(_N().atleast_2d(a))


def rng(seed, gen=None):
    _N().random.seed(int(seed))
    return float(seed)


def ode(f, tspan, y0, method='RK45', mass=None):
    np = _N()
    t0, tf = float(tspan[0]), float(tspan[1])
    fun = lambda t, y: np.asarray(f(t, y), dtype=float).ravel()
    kw = dict(method=method, dense_output=True)
    if mass is not None:
        kw['mass_matrix'] = np.atleast_2d(mass)
    return _SI().solve_ivp(fun, (t0, tf), np.asarray(y0, dtype=float).ravel(), **kw)


def bvp(fun, bc, xmesh, yguess, kind='col'):
    np = _N()
    f = lambda x, y: np.asarray(fun(x, y), dtype=float).ravel()
    g = lambda ya, yb: np.concatenate([np.atleast_1d(bc(ya, yb))])
    return _SI().solve_bvp(f, g, np.asarray(xmesh, float), np.asarray(yguess, float).reshape(-1, len(xmesh)))


def bvpinit(x, y):
    return (_N().asarray(x, float), _N().atleast_2d(y))


def deval(sol, xq):
    if hasattr(sol, 'sol'):
        return sol.sol(xq)
    return _N().interp(xq, sol.x, sol.y[0])


def dde(fun, lags, history, tspan, kind='rk'):
    np = _N()
    lags = np.atleast_1d(np.asarray(lags, float))
    t0, tf = float(tspan[0]), float(tspan[1])
    h = lambda t: np.asarray(history(t), dtype=float).ravel()
    tt, yy = [t0], [h(t0)]
    dt = min(1e-3, (tf - t0) / 1000.0)
    t = t0
    while t < tf - 1e-12:
        def rhs(s, z):
            def past(lag):
                ts = s - lag
                if ts <= t0:
                    return h(ts)
                return np.interp(ts, np.array(tt), np.array(yy)).ravel()
            return np.asarray(fun(s, z, *[past(l) for l in lags]), dtype=float).ravel()
        y0 = np.array(yy[-1])
        k1 = rhs(t, y0)
        k2 = rhs(t + 0.5 * dt, y0 + 0.5 * dt * k1)
        k3 = rhs(t + 0.5 * dt, y0 + 0.5 * dt * k2)
        k4 = rhs(t + dt, y0 + dt * k3)
        t = min(t + dt, tf)
        tt.append(t)
        yy.append(y0 + dt / 6.0 * (k1 + 2 * k2 + 2 * k3 + k4))
    out = type('Sol', (), {})()
    out.x = np.array(tt)
    out.y = np.array(yy).T
    return out


def pdepe(sym, ufun, icfun, bcfun, xgrid, tspan):
    np = _N()
    xg = np.asarray(xgrid, float)
    t = np.asarray(tspan, float)
    u0 = np.asarray([icfun(x) for x in xg], dtype=float)
    if u0.ndim == 1:
        u0 = u0[:, None]
    nx, nu = u0.shape

    def rhs(ti, u):
        U = u.reshape(nx, nu)
        dudx = np.zeros_like(U)
        dudx[1:-1] = (U[2:] - U[:-2]) / (xg[2] - xg[0])[:, None]
        c, f, s = ufun(xg, ti, U, dudx)
        c = np.asarray(c, float).reshape(nx, nu)
        f = np.asarray(f, float).reshape(nx, nu)
        s = np.asarray(s, float).reshape(nx, nu)
        flux = np.zeros_like(U)
        flux[1:-1] = (f[2:] - f[:-2]) / (xg[2] - xg[0])[:, None]
        xm = xg ** (-sym) if sym else 1.0
        return (xm[:, None] * flux + s).ravel()

    sol = _SI().solve_ivp(rhs, (t[0], t[-1]), u0.ravel(), t_eval=t, method='BDF')
    out = type('Sol', (), {})()
    out.x = xg
    out.t = sol.t
    out.u = sol.y.T.reshape(len(sol.t), nx, nu)
    return out


def decic(fun, t0, y0, yp0):
    return (_N().asarray(y0, float), _N().asarray(yp0, float))


_MCALL = {
    'zeros': lambda *a: _N().zeros(_sz(*a)),
    'ones': lambda *a: _N().ones(_sz(*a)),
    'eye': lambda n, m=None: _N().eye(int(n), int(m or n)),
    'magic': magic,
    'rand': lambda *a: _N().random.rand(*_sz(*a)),
    'randn': lambda *a: _N().random.randn(*_sz(*a)),
    'randi': lambda hi, *sz: _N().random.randint(1, int(hi) + 1, _sz(*sz)),
    'randperm': lambda n, k=None: _N().random.permutation(int(n))[: int(k or n)] + 1.0,
    'linspace': lambda a, b, n=100: _N().linspace(a, b, int(n)),
    'logspace': lambda a, b, n=50: _N().logspace(a, b, int(n)),
    'colon': lambda a, b, c=None: _N().arange(a, b + 1e-12) if c is None else _N().arange(a, c + 1e-12, b),
    'reshape': lambda v, r, c: _N().reshape(_N().asarray(v).ravel(order='F'), (int(r), int(c)), order='F'),
    'diag': lambda v: _N().diag(v),
    'trace': lambda a: _N().trace(a),
    'flipud': lambda a: _N().flipud(a),
    'fliplr': lambda a: _N().fliplr(a),
    'rot90': lambda a, k=1: _N().rot90(a, int(k)),
    'circshift': lambda a, s: _N().roll(a, int(s), axis=1),
    'repmat': lambda a, r, c: _N().tile(a, (int(r), int(c))),
    'kron': lambda a, b: _N().kron(a, b),
    'cat': lambda dim, a, b: _N().concatenate((a, b), axis=int(dim) - 1),
    'vertcat': lambda a, b: _N().vstack((a, b)),
    'horzcat': lambda a, b: _N().hstack((a, b)),
    'size': lambda a: _N().shape(a),
    'numel': lambda a: _N().size(a),
    'length': lambda a: max(_N().shape(a)),
    'ndims': lambda a: _N().ndim(a),
    'isempty': lambda a: _N().size(a) == 0,
    'sort': lambda a: _N().sort(a, axis=0),
    'sortrows': lambda a: a[_N().lexsort(_N().atleast_2d(a)[:, ::-1].T)],
    'find': lambda a: _N().flatnonzero(a) + 1,
    'unique': lambda a: _N().unique(a),
    'ismember': lambda a, b: _N().isin(a, b),
    'vec': lambda a: _N().asarray(a).ravel(order='F'),
    'row': lambda a, i: _N().atleast_2d(a)[int(i), :],
    'col': lambda a, i: _N().atleast_2d(a)[:, int(i)],
    'submatrix': lambda a, r0, r1, c0, c1: _N().atleast_2d(a)[int(r0):int(r1) + 1, int(c0):int(c1) + 1],
    'expm': expm,
    'logm': lambda a: _SL().logm(a),
    'sqrtm': lambda a: _SL().sqrtm(a),
    'lu': lambda a: _SL().lu(a),
    'qr': lambda a: _N().linalg.qr(a),
    'chol': lambda a: _SL().cholesky(a),
    'svd': lambda a: _N().linalg.svd(a),
    'eig': lambda a: _N().linalg.eig(a),
    'eigs': lambda a, k=6: _SK().eigs(a, k=int(k)),
    'svds': lambda a, k=6: _SK().svds(a, k=int(k)),
    'cond': lambda a: _N().linalg.cond(a),
    'sylvester': lambda a, b, c: _SL().solve_sylvester(a, b, c),
    'lyap': lambda a, b: _SL().solve_lyapunov(a, b),
    'pagemtimes': lambda a, b: _N().einsum('...ij,...jk->...ik', a, b),
    'pagetranspose': lambda a: _N().swapaxes(a, -1, -2),
    'pageinv': lambda a: _N().linalg.inv(a),
    'pagesvd': lambda a: _N().linalg.svd(a),
    'pageeig': lambda a: _N().linalg.eig(a),
    'pagenorm': lambda a: _N().linalg.norm(a, axis=(-1, -2)),
    'solve': lambda a, b: _N().linalg.solve(a, b),
    'mldivide': lambda a, b: _N().linalg.solve(a, b),
    'rank': lambda a: _N().linalg.matrix_rank(a),
    'det': lambda a: _N().linalg.det(a),
    'inv': lambda a: _N().linalg.inv(a),
    'norm': lambda a: _N().linalg.norm(a),
    'eigenvals': lambda a: _N().linalg.eigvalsh(a),
    'eigenvectors': lambda a: _N().linalg.eigh(a)[1],
    'pcg': lambda a, b: _SK().cg(a, b)[0],
    'bicg': lambda a, b: _SK().bicg(a, b)[0],
    'bicgstab': lambda a, b: _SK().bicgstab(a, b)[0],
    'cgs': lambda a, b: _SK().cgs(a, b)[0],
    'gmres': lambda a, b: _SK().gmres(a, b)[0],
    'minres': lambda a, b: _SK().minres(a, b)[0],
    'qmr': lambda a, b: _SK().qmr(a, b)[0],
    'symmlq': lambda a, b: _SK().symmlq(a, b)[0],
    'tfqmr': lambda a, b: _SK().tfqmr(a, b)[0],
    'lsqr': lambda a, b: _SK().lsqr(a, b)[0],
    'lsmr': lambda a, b: _SK().lsmr(a, b)[0],
    'rng': rng,
    'unifrnd': lambda n, a, b: _N().random.uniform(a, b, int(n)),
    'normrnd': lambda n, mu, sd: _N().random.normal(mu, sd, int(n)),
    'exprnd': lambda n, mu: _N().random.exponential(mu, int(n)),
    'poissrnd': lambda n, lam: _N().random.poisson(lam, int(n)),
    'binornd': lambda n, p, k: _N().random.binomial(int(k), p, int(n)),
    'ode45': lambda f, ts, y0: ode(f, ts, y0, 'RK45'),
    'ode23': lambda f, ts, y0: ode(f, ts, y0, 'RK23'),
    'ode113': lambda f, ts, y0: ode(f, ts, y0, 'LSODA'),
    'ode15s': lambda f, ts, y0, mass=None: ode(f, ts, y0, 'BDF', mass),
    'ode23s': lambda f, ts, y0: ode(f, ts, y0, 'Radau'),
    'ode23t': lambda f, ts, y0: ode(f, ts, y0, 'DOP853'),
    'ode23tb': lambda f, ts, y0: ode(f, ts, y0, 'BDF'),
    'decic': decic,
    'bvp4c': lambda f, bc, x, y: bvp(f, bc, x, y, 'col'),
    'bvp5c': lambda f, bc, x, y: bvp(f, bc, x, y, 'col5'),
    'bvpinit': bvpinit,
    'deval': deval,
    'dde23': lambda f, l, h, ts: dde(f, l, h, ts, 'rk'),
    'ddesd': lambda f, l, h, ts: dde(f, l, h, ts, 'state'),
    'ddensd': lambda f, l, h, ts: dde(f, l, h, ts, 'neutral'),
    'pdepe': pdepe,
    'interp1': lambda x, y, xq: _N().interp(xq, x, y),
    'interp2': lambda x, y, v, xq, yq=None: _SP().RegularGridInterpolator((x, y), v)((xq, yq)) if yq is not None else _SP().interp2d(x, y, v),
    'interp3': lambda x, y, z, v, q: _SP().RegularGridInterpolator((x, y, z), v)(q),
    'interpn': lambda pts, v, q: _SP().RegularGridInterpolator(pts, v)(q),
    'griddedInterpolant': lambda pts, v: _SP().RegularGridInterpolator(pts, v),
    'scatteredInterpolant': lambda x, v: _SP().LinearNDInterpolator(x, v),
    'spline': lambda x, y, xq: _SP().CubicSpline(x, y)(xq),
    'pchip': lambda x, y, xq: _SP().PchipInterpolator(x, y)(xq),
    'csaps': lambda x, y, xq: _SP().CubicSpline(x, y)(xq),
    'delaunay': lambda x, y: _SS().Delaunay(list(zip(x, y))),
    'delaunayn': lambda p: _SS().Delaunay(p),
    'tsearchn': lambda tri, q: tri.find_simplex(q),
    'dsearchn': lambda tri, q: _SS().cKDTree(tri.points).query(q)[1] + 1,
    'convhull': lambda x, y: _SS().ConvexHull(list(zip(x, y))).vertices,
    'polyfit': lambda x, y, d: _N().polyfit(x, y, int(d)),
    'polyval': lambda c, x: _N().polyval(c, x),
    'polyder': lambda c: _N().polyder(c),
    'polyint': lambda c, k=0: _N().polyint(c, k),
    'roots': lambda c: _N().roots(c),
    'conv': lambda a, b: _N().convolve(a, b),
    'deconv': lambda a, b: _N().polydiv(a, b)[0],
    'trapz': lambda y, x=None: _N().trapz(y, x),
    'cumtrapz': lambda y, x=None: _SI().cumtrapz(y, x, initial=0),
    'gradient': lambda v, h=1: _N().gradient(v, h),
    'diff': lambda v: _N().diff(v),
    'nextpow2': lambda x: 0 if abs(x) <= 1 else int(_m.ceil(_m.log2(abs(x)))),
    'fftshift': lambda v: _N().fft.fftshift(v),
    'fft': lambda v: _N().fft.fft(v),
    'ifft': lambda v: _N().fft.ifft(v),
    'fft2': lambda a: _N().fft.fft2(a),
    'conv2': lambda a, b: _SG().convolve2d(a, b),
    'filter': lambda b, a, x: _SG().lfilter(b, a, x),
    'pwelch': lambda x, fs=1: _SG().welch(x, fs)[1],
    'periodogram': lambda x, fs=1: _SG().periodogram(x, fs)[1],
    'fminbnd': lambda f, a, b: _SO().minimize_scalar(f, bounds=(a, b), method='bounded').x,
    'fminsearch': lambda f, x0: _SO().minimize(f, x0, method='Nelder-Mead').x,
    'fmincon': lambda f, x0, cons=None: _SO().minimize(f, x0, method='SLSQP', constraints=cons or ()).x,
    'fsolve': lambda f, x0: _SO().fsolve(f, x0),
    'fzero': lambda f, a, b: _SO().brentq(f, a, b),
    'lsqnonlin': lambda f, x0: _SO().least_squares(f, x0).x,
    'lsqcurvefit': lambda f, x0, xd, yd: _SO().curve_fit(f, xd, yd, p0=x0)[0],
    'fitlm': lambda x, y: _N().polyfit(x, y, 1),
    'maximize': lambda f, a, b: _SO().minimize_scalar(lambda t: -f(t), bounds=(a, b), method='bounded').x,
    'newton': lambda f, x0: _SO().newton(f, x0),
    'root': lambda f, a, b: _SO().brentq(f, a, b),
    'string': str,
    'strlength': lambda s: len(str(s)),
    'num2str': str,
    'str2num': float,
    'upper': lambda s: str(s).upper(),
    'lower': lambda s: str(s).lower(),
    'contains': lambda s, p: p in str(s),
    'isspace': lambda s: str(s).isspace(),
    'append': lambda a, b: str(a) + str(b),
    'split': lambda s, sep=None: str(s).split(sep),
    'sprintf': lambda fmt, *a: fmt % tuple(a[0]) if len(a) == 1 and isinstance(a[0], (list, tuple)) else fmt % a,
    'fprintf': lambda fmt, *a: print(fmt % tuple(a[0]) if len(a) == 1 and isinstance(a[0], (list, tuple)) else fmt % a),
    'regexp': lambda s, pat: [mm.start() + 1 for mm in __import__('re').finditer(pat, str(s))],
    'cell': list,
    'struct': dict,
    'table': lambda cols, names=None: __import__('pandas').DataFrame({(names[i] if names else 'Var' + str(i + 1)): c for i, c in enumerate(cols)}),
    'sum': lambda a, dim=None: _N().sum(a, axis=None if dim is None else int(dim) - 1),
    'prod': lambda a, dim=None: _N().prod(a, axis=None if dim is None else int(dim) - 1),
    'max': lambda a, dim=None: _N().max(a, axis=None if dim is None else int(dim) - 1),
    'min': lambda a, dim=None: _N().min(a, axis=None if dim is None else int(dim) - 1),
    'mean': lambda a, dim=None: _N().mean(a, axis=None if dim is None else int(dim) - 1),
    'median': lambda a, dim=None: _N().median(a, axis=None if dim is None else int(dim) - 1),
    'std': lambda a, dim=None: _N().std(a, axis=None if dim is None else int(dim) - 1, ddof=1),
    'var': lambda a, dim=None: _N().var(a, axis=None if dim is None else int(dim) - 1, ddof=1),
    'rms': lambda a: float(_N().sqrt(_N().mean(_N().asarray(a) ** 2))),
    'geomean': lambda a: float(_m.exp(_N().mean(_N().log(a)))),
    'harmmean': lambda a: float(len(a) / _N().sum(1.0 / _N().asarray(a))),
    'cumsum': lambda a: _N().cumsum(a),
    'cumprod': lambda a: _N().cumprod(a),
}


def mcall(name, *args):
    fn = _MCALL.get(name)
    if fn is None:
        raise RuntimeError('aaiyyan.mcall: unsupported function ' + str(name))
    return fn(*args)
