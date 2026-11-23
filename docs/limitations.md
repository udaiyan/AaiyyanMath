# Limitations

## Language

1. Letter runs lex as one identifier: `xy` is the name `xy`. Write `x*y`.
2. No matrices, vectors, ranges or indexing yet. `a_{b}` becomes the identifier
   `a_b`, not a subscripted access.
3. No comparison or boolean operators, no conditionals, no statement blocks.
4. No function definitions (`f(x) := …`); only scalar definitions.
5. One statement per region. Multi-statement lines are rejected.

## Numerics

6. `f64` only — no complex numbers, no arbitrary precision, no units or
   dimensional checking.
7. Integrals use fixed Simpson's rule (n = 2000). Singular or highly oscillatory
   integrands will be inaccurate; there is no adaptive refinement or error
   estimate.
8. Sums and products are capped at 1e6 terms and round their limits to integers.
9. `fact` is limited to 0..170 (f64 range).
10. Division by zero is an error; overflow silently yields `inf`.

## Document

11. Python lines map to regions **by index**. Deleting a line shifts the
    positions of everything below it.
12. Blank Python lines are dropped, so they cannot be used as separators.
13. No save/load, no export to file, no plotting, no tables, no controls.
    `Copy script` is the only export.
14. Undo is snapshot-based and excludes pointer moves.

## Front end

15. Placeholder sizing targets MathLive's internal `.ML__placeholder` class,
    which is not public API; a MathLive major upgrade may need it retargeted.
    Caret placement and Tab cycling use `executeCommand`, which is stable.
16. `#grid` height assumes a 41 px toolbar; a wrapped toolbar at very narrow
    widths will misalign the panes.
17. Pane sizes and visibility persist in `localStorage`; the document does not.

## Packaging

18. Icons are generated for Windows and Linux. macOS `.icns` and Windows Store
    tiles need `cargo tauri icon src-tauri/icons/icon.png`.
19. CI covers Ubuntu and Windows; no macOS runner.
