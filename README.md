# AaiyyanMath

A Mathcad-inspired worksheet math app built with Rust, egui/eframe
and exmex. Write math regions top-to-bottom, bind results
with `name := expression`, carry SI units through every calculation, take numerical
derivatives and integrals, plot live — and export or run the whole worksheet as Python.

## Run

```bash
cargo run
```

The Python pane (▶ Run) executes the generated script with your local Python 3.
It needs these packages:

```bash
pip install numpy pint matplotlib
```

If Python or a package is missing, the output panel tells you — the Rust engine
works fully offline without Python.

## Worksheets

- **Workspaces** (left sidebar): independent worksheets, rename inline, add new ones.
- **Regions** (center): Text notes, Math (`name := expr`), and Plots. Reorder ⬆⬇ / remove ❌.
- Math regions evaluate **top-down**: a region can use any name bound above it.
  Errors break the chain below them, exactly like Mathcad. Empty name = anonymous
  result (printed, not bound).

## Units engine (`src/quantity.rs`)

Every value is a `Quantity { value, dim }` with a 7-exponent SI dimension vector
(kg, m, s, A, K, mol, cd). Addition/subtraction require matching dimensions;
multiplication/division compose them; transcendental functions require
dimensionless arguments.

- Base units: `m s kg A K mol cd`; prefixed: `km cm mm g ms us ns`; constants `pi e`.
- Implicit multiplication (`5 m`, `2(x)`, `3 sin(x)`) binds **tighter than `*` and `/`**
  (Mathcad semantics); powers bind tighter still. `ms` is one unit (milliseconds),
  `m s` is metres·seconds.
- Integer powers (`x^2`), `sqrt` halves dimensions, `min`/`max` require matching dims.

```text
v := 5 m / 2 s        →  2.5 m·s^-1      (juxtaposition groups: 5m / (2s))
v := 5 m / (2 s)      →  2.5 m·s^-1      (parentheses still work)
half := 1 / 2 m       →  0.5 m^-1        (1 / (2m), not (1/2)·m)
area := 2 m^2         →  2 m^2           (power binds tighter: 2·(m^2))
bad := 2 + 3 m        →  Dimension mismatch
```

Explicit `*` and `/` keep ordinary left-to-right precedence: `5 * m / 2 * s` = 2.5 m·s.
The Python pane shares the same preprocessor, so generated scripts evaluate identically.

## Calculus (numerical)

- `deriv(expr, var)` — central difference (h = 1e-5), evaluated at `var`'s current
  binding (0 if unbound).
- `integ(expr, var, a, b)` — composite Simpson, 1000 slices; bounds may be expressions.
- In **plots**, calculus is expanded per sample, so `deriv(x^2, x)` draws `2x` across
  the axis range.

## Python pane

The right panel generates a standalone script (numpy + pint + matplotlib, headless
Agg) mirroring the worksheet: comments for text regions, assignments + prints for
math, `savefig` PNGs for plots. ▶ Run executes it in a subprocess (30 s timeout,
cancel button); stdout/stderr and the rendered plots appear inline. Calculus maps
to generated `_deriv`/`_integ` helpers — no scipy needed, and plot expressions
evaluate over the whole linspace.

- **📋 Copy** puts the generated script on the clipboard; **⬇ Export .py** writes it
  to `aaiyyanmath_export.py` in the working directory (gitignored).
- **Interpreter** overrides the Python executable (name or full path); leave it blank
  for auto-detection (`python`/`py` on Windows, `python3`/`python` elsewhere). The
  override is saved with the document.
- If a run fails with `ModuleNotFoundError`, the output panel names the missing
  package and the fix: `pip install numpy pint matplotlib`.

## Persistence

Save/Load buttons write `aaiyyanmath_doc.json` in the working directory, which is
auto-loaded on startup. The document format is **versioned** (currently `version: 1`):

- Legacy documents without a `version` field load normally and are stamped with the
  current version on the next save.
- Documents written by a **newer** app version are rejected with a clear error
  instead of being silently mis-read.
- On load, empty worksheet lists are repaired, an out-of-range active workspace is
  clamped, and every worksheet is re-evaluated top-down — bindings are never trusted
  from disk.

The working document is **gitignored** (it is your local scratch state, not source).
A committed, version-less example lives at `examples/sample_doc.json` and doubles as
the backward-compatibility fixture in the test suite. To try it, copy it over
`aaiyyanmath_doc.json` before launching the app.

## Architecture

| File | Role |
|---|---|
| `src/main.rs` | eframe bootstrap |
| `src/app.rs` | app state, panels, save/load, Python pane, plot textures |
| `src/worksheet.rs` | region list, top-down evaluation, bindings |
| `src/region.rs` | Text / Math / Plot enum (serde) |
| `src/expression.rs` | one math region: parse → evaluate → render |
| `src/quantity.rs` | dimensions, units, implicit-mult preprocessing, numerical calculus, exmex operators |
| `src/plot.rs` | egui_plot pane with per-sample calculus and caching |
| `src/python.rs` | worksheet → Python code generation |
| `src/runner.rs` | subprocess execution: timeout, cancel, output capture |

## Development

```bash
cargo test --workspace          # unit tests across all modules
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

CI (GitHub Actions) runs tests on Ubuntu + Windows; fmt/clippy are gating.

## Roadmap

- Non-finite calculus guards
- Derived units (N, J, W, Pa, Hz, V, Ω…), fractional exponents, variadic min/max, `root(f, x, guess)`
- Range variables, vectors, matrices
- Symbolic CAS (Symbolica) with the numerical engine as fallback
- 2D WYSIWYG equation editing, real file dialogs, undo/redo
- Packaging/installers