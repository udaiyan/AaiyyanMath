# AaiyyanMath

Tauri + MathLive front end, Rust maths backend. Three resizable, hideable panes:
**tools | canvas | Python**.

## Run

```bash
npm install
cargo install tauri-cli --version "^2" --locked   # once
npm run app                                       # or double-click dev.cmd
```

## Model

* One canvas region corresponds to one line of Python, in order.
* Editing a math field rewrites that Python line. Editing the Python rewrites the
  canvas. Neither side inserts characters into the other while you are typing.
* Rust evaluates everything natively: arithmetic, powers, roots, trig, logs,
  statistics, definite integrals (Simpson), sums and products.
* `x := 3` defines. A bare expression shows its value beside it. Assignments do not.
* **Copy script** puts a runnable Python file on the clipboard, helpers included.

## Panes

`Ctrl+1` tools · `Ctrl+2` canvas · `Ctrl+3` Python · drag the dividers to resize.
Layout is remembered between runs.

## Adding maths

`src-tauri/src/math.rs` holds the AST, both parsers, both renderers and the
evaluator. Add a function to `FUNCS` and `apply()`; add a LaTeX form to
`latex.rs` if it needs one.
