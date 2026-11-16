# Verifies interpreter and library availability for the embedded backend.
import importlib
import json
import sys


def main():
    report = {"python": sys.version.split()[0], "executable": sys.executable, "libraries": {}}
    ok = True
    for name in ("numpy", "scipy", "sympy", "pandas", "matplotlib", "pint"):
        try:
            module = importlib.import_module(name)
            report["libraries"][name] = getattr(module, "__version__", "unknown")
        except Exception as exc:
            report["libraries"][name] = "MISSING ({})".format(exc)
            ok = False
    print(json.dumps(report, indent=2))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
