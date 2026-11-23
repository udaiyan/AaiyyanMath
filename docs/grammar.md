# Grammar

One statement per region / per Python line.

## Statements

| Form | Meaning |
| --- | --- |
| `x := 3` | definition (LaTeX `\coloneqq` also accepted) |
| `x = 3` | definition, Python side |
| `1+2` | expression — value shown beside the field |
| `1+2 =` | trailing `=` stripped; still an expression |

Definitions show no result; expressions do. Nothing is inserted into a field.

## Expressions

Precedence, low to high: `+ -` (1), `* /` (2), unary (3), `^` / `**` (4, right
associative). Implicit multiplication binds at 3: `2x`, `3(x+1)`.

Atoms: numbers (incl. `1.5e-3`), identifiers, `( … )`, calls, and the LaTeX
structures below.

## LaTeX structures

| LaTeX | AST |
| --- | --- |
| `\frac{a}{b}` | `a / b` |
| `a^{b}` | `a ^ b` |
| `a_{b}` | identifier `a_b` |
| `\sqrt{a}` | `sqrt(a)` |
| `\sqrt[n]{a}` | `nroot(a, n)` |
| `\left\|a\right\|` | `abs(a)` |
| `\int_{a}^{b} f \, dx` | `Int { body: f, var: x, lo: a, hi: b }` |
| `\sum_{k=a}^{b} f` | `Sum { body: f, var: k, lo: a, hi: b }` |
| `\prod_{k=a}^{b} f` | `Sum { …, prod: true }` |
| `\mathrm{GPa}` | identifier `GPa` |
| `\pi`, `\alpha`, … | identifier `pi`, `alpha`, … |

The differential is found by scanning from the **end** of the integral body, so
`dx` (one token) and `d x` / `d\rho` (two tokens) both work, and a body variable
such as `density` is never split into `d` + `ensity`.

## Python structures

```python
integrate(lambda x: x**2, 0, 1)
sumrange(lambda k: k**2, 1, 10)
prodrange(lambda k: k, 1, 5)
```

## Functions

`sqrt nroot abs exp ln log10 log2 sin cos tan asin acos atan sinh cosh tanh
floor ceil round sign min max sum mean std fact deg rad`

Constants: `pi`, `e`, `tau`. `#` starts a comment (Python side).
