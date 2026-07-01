# deadpy

A fast, Rust-based dead code detector for Python, built on [tree-sitter](https://tree-sitter.github.io/tree-sitter/).

`deadpy` scans a Python codebase, parses every file in parallel (via [rayon](https://github.com/rayon-rs/rayon)), and reports functions, classes, methods, variables, and imports that are defined but never referenced anywhere else in the project.

## Why

Static "find unused code" tools for Python (e.g. [vulture](https://github.com/jendrikseipp/vulture)) are invaluable in CI but can be slow on large codebases, since they re-parse and re-walk the AST with the Python interpreter itself. `deadpy` uses tree-sitter to parse everything up front and in parallel, then resolves references in a single in-memory pass — no Python runtime involved.

## Installation

### From source

Requires a [Rust toolchain](https://rustup.rs/):

```sh
git clone https://github.com/brightwave-inc/deadpy.git
cd deadpy
cargo build --release
./target/release/deadpy --help
```

> **Coming soon:** `pip install deadpy` (prebuilt wheels, no Rust toolchain required) and downloadable binaries on the [Releases](https://github.com/brightwave-inc/deadpy/releases) page.

## Usage

```sh
# Scan the current directory (or the paths from pyproject.toml / CLI args)
deadpy

# Scan specific paths
deadpy src/ scripts/

# Only report unused functions and classes
deadpy --report function,class

# Machine-readable output
deadpy --format json
deadpy --format count   # just the number of findings, for CI gating
```

`deadpy` exits `1` if any dead code is found and `0` otherwise, so it drops straight into CI.

### CLI flags

| Flag | Description |
|---|---|
| `[paths]...` | Paths to scan (overrides config) |
| `--config <PATH>` | Path to `pyproject.toml` (default: `./pyproject.toml`) |
| `--exclude <PATTERN>` | Additional exclude pattern (repeatable) |
| `--ignore-decorators <PATTERN>` | Additional decorator pattern to ignore (repeatable) |
| `--ignore-names <PATTERN>` | Additional name glob to ignore (repeatable) |
| `--min-confidence <N>` | Minimum confidence (0-100) to report, default `60` |
| `--whitelist <FILE>` | Additional whitelist file (repeatable) |
| `--report <KINDS>` | Comma-separated kinds to report: `function,class,method,variable,import` |
| `--sort-by-size` | Sort findings by code size (largest first) instead of file:line |
| `--format <FMT>` | Output format: `text` (default), `json`, `count` |
| `-q, --quiet` | Only print findings, no scan summary |

### Configuration

All of the above (except `--config` itself) can be set under `[tool.deadpy]` in your `pyproject.toml`. CLI flags extend or override the config:

```toml
[tool.deadpy]
paths = ["src/"]
exclude = [".venv/", "*/migrations/*"]
ignore_names = ["_legacy_*"]
ignore_decorators = ["@app.route", "@celery.task"]
min_confidence = 70
report = ["function", "class", "method"]
whitelist = ["whitelist.py"]
```

## How it decides something is dead

A definition (function, class, method, variable, or import) is flagged unless one of the following holds:

- Its **name** — or, for methods, its qualified `Class.method` name — appears as a reference anywhere else in the scanned code: a call, attribute access, import, or bare identifier.
- It's a Python **dunder method** (`__init__`, `__str__`, `__enter__`, …). These are part of the data model and invoked implicitly by the interpreter, so they're always excluded.
- It matches an `ignore_names` glob (e.g. `"test_*"`), or carries a decorator matching an `ignore_decorators` pattern (e.g. `"@app.route"` — useful for framework-registered handlers never called directly in Python).
- It's referenced from a **whitelist file** — a `.py` file of dummy references (e.g. `MyClass.some_method`) documenting known-but-indirect usages (dynamic dispatch, string-based lookup, etc.), the same escape hatch vulture uses.
- Names listed only in `__all__` are **not** treated as references — re-export alone doesn't prove a symbol is consumed, so a name that appears in `__all__` but is otherwise unused is still flagged.

Every finding carries a `confidence` score, gated by `--min-confidence`. Today all findings score a flat 60; the field exists so future heuristics (e.g. lower confidence for `__all__`-only exports or dynamically dispatched names) can be layered in without changing the config surface.

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## License

MIT — see [LICENSE](LICENSE).
