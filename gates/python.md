# Python Coding Gates

For audit tooling, ML pipelines, scripts, and any other Python surface.

## Project-level

- Python 3.11 minimum (for `tomllib` and improved typing).
- `pyproject.toml` declares dependencies; no bare `setup.py`.
- Virtualenv or `uv` per project; no global installs.

## Style / lint

- `ruff` for lint AND format (replaces `black` + `flake8` + `isort`).
- `ruff check` clean; `ruff format --check` clean.
- `mypy --strict` (or `pyright`) clean on library modules.

## Types

- Every public function has type annotations.
- No `Any` without a comment justifying it.
- Use `Literal`, `TypeAlias`, `TypedDict`, dataclasses, and Pydantic models
  to express intent.

## Error handling

- No bare `except:` — always specify the exception class.
- No `except Exception` without a comment justifying it.
- Errors carry context (custom exception classes with `__cause__` chained).

## Async

- `asyncio` with explicit timeouts; no unbounded `await`.
- Cancellation-safe — no naked `try: ...`/`finally:` blocks that swallow
  `CancelledError`.

## Dependencies

- `pip-audit` clean.
- Lockfile via `uv lock` / `pip-tools` committed.
- No deps with known CVEs without explicit `SHIP-DECISION:`.

## Tests

- `pytest` with `pytest-cov`; coverage tracked.
- `hypothesis` for property tests on parsers, math, validators.
- Tests under `tests/` directory next to source.

## Annotations

Same as Rust:

- `BUG ASSUMPTION:` on every public function.
- `SECURITY:` on every defense-in-depth.

See [`../annotations/README.md`](../annotations/README.md).
