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
