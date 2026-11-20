# AaiyyanMath

A native Rust engineering-mathematics environment with a Mathcad-style grid
worksheet, first-class dimensional units, and an optional embedded-Python
backend (NumPy / SciPy / SymPy) for heavy numerics.

## What it is

- **Worksheet canvas** — an infinite graph-paper grid. Click anywhere, type an
  expression, press Enter. Regions are free-placed cards you can drag, edit and
  disable.
- **Mathcad semantics** — `:=` defines, a trailing `=` evaluates, Greek symbols
  and units come from grouped ribbon palettes, and every result is checked for
  dimensional consistency natively.
- **Hybrid engine** — ~90 functions evaluate in pure Rust (nalgebra): arrays,
  reshaping, `kron`, `magic`, `expm`, reductions, polynomials, fits, solvers.
  ~100 heavier names (ODE/BVP/DDE/PDE solvers, iterative linear solvers, FFT,
  splines, scattered interpolation, optimization, matrix factorizations) route
  to the embedded Python backend. The worksheet AST is always authoritative;
  Python is a compute backend, never the source of truth.
- **Live code generation** — every recalculation emits runnable Python
  (NumPy / SciPy / pint) shown in the bottom dock and exportable as a script.

## Requirements

- Rust stable (pinned via `rust-toolchain.toml`)
- A desktop OS (Windows / macOS / Linux) for the egui front end
- Optional: Python 3.10+ with `numpy scipy sympy pint matplotlib pandas`
  for the `python` feature

## Build and run

```bash
cargo run -p aaiyyan-app

# start windowed instead of maximized
cargo run -p aaiyyan-app -- --windowed

# open a project and pick a backend
cargo run -p aaiyyan-app -- --project path/to/project.aaiyyan.json --backend native
```

## Python backend (SciPy / SymPy)

```bash
python python/bootstrap/setup_env.py

# Windows
set PYO3_PYTHON=%CD%\.venv\Scripts\python.exe
# macOS / Linux
export PYO3_PYTHON=$PWD/.venv/bin/python

cargo run -p aaiyyan-app --features python
```

Without the feature, Python-routed functions do not crash: the region reports
"routed to embedded Python" and the Errors dock lists them.

## Worksheet language, in one table

| Syntax | Meaning |
| --- | --- |
| `E := 210 GPa` | definition with a physical unit |
| `delta =` | Mathcad evaluation operator (show the value) |
| `f(x) := a*sin(x)` | function definition |
| `A * B` | matrix multiplication (`*` is never element-wise) |
| `convert(delta, mm)` | unit conversion |
| `v := colon(0, 2, 10)` | ranged vector, MATLAB `0:2:10` |

## Keyboard and pointer model

`F5` recalculate · `Esc` finish editing · `Enter` commit · `Del` delete region ·
`Ctrl+Z` / `Ctrl+Shift+Z` undo / redo · `Ctrl+S` save · `Ctrl+±` zoom ·
click a region to edit · drag the dotted grip (or alt-drag) to move ·
drag empty grid or middle-drag to pan · `Ctrl+wheel` to zoom about the pointer.

## Workspace layout

```text
crates/
  ast        parser      units       evaluator   dependency
  codegen    symbolic    plotting    worksheet   project
  reporting  python_runtime          core        gui         app
tests/integration        end-to-end worksheet pipelines
python/scripts           aaiyyan_runtime.py (mirror of the Rust catalog)
docs/                    architecture, grammar, units, limitations
```

## Testing

```bash
cargo test --workspace
cargo test -p aaiyyan-evaluator --lib                 # fast maths unit tests
cargo test -p aaiyyan-integration-tests --features python   # needs the venv
```

## License

MIT — see [LICENSE](LICENSE).
