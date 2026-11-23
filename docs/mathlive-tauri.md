# MathLive + Tauri front end

## Why

The hand-written typesetter in `crates/gui/src/template.rs` was the source of
every clipping, overlap and duplicated-keystroke defect. MathLive is a mature
math field: it typesets, handles carets, placeholders, selection and keyboard
input correctly. This front end replaces that file's responsibility and nothing
else.

## What stayed

All eleven engine crates, `core::Session`, the integration tests and CI are
untouched. `Session` already exposed everything a front end needs, so the Tauri
layer is a thin command wrapper.

## Layout

    crates/latex/            LaTeX -> AST -> worksheet source bridge (in workspace, unit-tested)
    apps/desktop/src-tauri/  Tauri v2 shell, detached workspace, commands over Session
    apps/web/                Vite + TypeScript + MathLive

The Tauri crate declares its own `[workspace]` so `cargo test --workspace` at the
repo root never pulls WebKit/GTK system libraries.

## Data flow

    <math-field> LaTeX
      -> crates/latex::bridge()  -> aaiyyan_ast::Statement
      -> Session::update_source() -> dependency graph -> native evaluator
      -> Session::values_for()    -> value + LaTeX
      -> read-only result beside the field

Authoritative state is always the worksheet source. Canonical LaTeX is
regenerated from the AST via `aaiyyan_ast::statement_to_latex`, so the editor and
the document can never disagree.

## Input contract

* Nothing is inserted automatically. A field contains exactly what was typed or
  explicitly chosen from the palette.
* `x := 3` (or `\coloneqq`) defines.
* A trailing `=` is the Mathcad evaluation operator and the only thing that makes
  a result appear.
* An empty MathLive placeholder bridges to `BridgeError::Incomplete`, which is
  treated as unfinished, not as an error.

## Running

```bash
cd apps/web && npm install
cd ../desktop/src-tauri && cargo install tauri-cli --version '^2' && cargo tauri dev
```

## Known limits

* `\sum` / `\prod` bridge to `sumrange` / `prodrange`, which need a callable body;
  a bare expression in the index reports the engine's existing limitation.
* Letter runs lex as one identifier, so `xy` is the name `xy`; write `x*y` or
  `x\,y` for a product.
* The egui app (`crates/app`) still builds and is left in place until this front
  end reaches parity on plots, tables and controls.
