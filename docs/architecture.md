# Architecture

Three panes, one document, two representations of the same mathematics.

```
┌──────────┬───────────────────────────────┐
│  tools   │            canvas             │
│ palette  │   MathLive regions, free place │
│ + size   ├───────────────────────────────┤
│          │            Python             │
│          │   one line per region, in order│
└──────────┴───────────────────────────────┘
```

## Layers

| Path | Responsibility |
| --- | --- |
| `src/main.ts` | Panes, splitters, palette, region DOM, two-way sync |
| `src/styles.css` | Layout, region chrome, MathLive placeholder styling |
| `src-tauri/src/main.rs` | Document state, undo/redo, Tauri commands |
| `src-tauri/src/math.rs` | AST, Python parser/renderer, native evaluator |
| `src-tauri/src/latex.rs` | LaTeX parser/renderer |

## The document

`Doc { regions: Vec<Region>, next: u32 }`, where `Region { id, latex, x, y }`.
LaTeX is the stored form. Everything else — Python line, value, error — is
derived on every command by `build()`, which walks the regions in order with a
shared environment, so a later region sees earlier definitions.

## Two-way sync

* Field edited → `set_latex(id, latex)` → `latex::parse_latex` → AST →
  `math::to_python` regenerates that line.
* Python edited → `set_python(text)` → one region per non-blank line, positions
  preserved by index → `latex::to_latex` regenerates each field.

Neither side writes into a pane that currently has focus, so typing is never
interrupted. The Python pane is debounced at 280 ms.

## Evaluation

Native Rust, no Python runtime required. Simpson's rule (n = 2000) for definite
integrals; bounded loops for sums and products (max 1e6 terms). `Copy script`
prepends `math::PRELUDE` so the clipboard contents run under CPython.

## Undo

Snapshot-based. `record()` pushes a `Doc` clone before each mutating command,
capped at 100. Pointer moves are not recorded.
