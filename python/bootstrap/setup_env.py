# Creates a managed virtual environment and installs the pinned scientific stack.
# Usage: python python/bootstrap/setup_env.py [--venv .venv] [--dev]
import argparse
import os
import subprocess
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--venv", default=".venv")
    parser.add_argument("--dev", action="store_true")
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    venv = (root / args.venv).resolve()
    requirements = root / "python" / "requirements" / ("requirements-dev.txt" if args.dev else "requirements.txt")

    if not venv.exists():
        print("creating virtual environment at", venv)
        subprocess.check_call([sys.executable, "-m", "venv", str(venv)])

    python = venv / ("Scripts/python.exe" if os.name == "nt" else "bin/python")

    subprocess.check_call([str(python), "-m", "pip", "install", "--upgrade", "pip"])
    subprocess.check_call([str(python), "-m", "pip", "install", "-r", str(requirements)])

    print("")
    print("Environment ready.")
    print("Build the embedded backend with:")
    if os.name == "nt":
        print('  $env:PYO3_PYTHON = "{}"'.format(python))
    else:
        print('  export PYO3_PYTHON="{}"'.format(python))
    print("  cargo run -p aaiyyan-app --features python")
    print("")
    print("If packages are not found, add this path in Settings > Python extra paths:")
    print("  {}".format(venv))


if __name__ == "__main__":
    main()
