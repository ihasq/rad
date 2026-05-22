# Rad

Rad is a source control management protocol for the LLM era.

## Implementations

- `rust/` — Native implementation (Rust)
- `ts/` — Edge implementation (WinterTC TypeScript)

Both implementations are spec-identical. The shared test suite
in `tests/` verifies output parity.

## Specification

- `spec/PHILOSOPHY.md` — Protocol philosophy (10 principles)
- `spec/relay/openapi.yaml` — Rad Relay HTTP API

## Build

```sh
# Rust
cd rust && cargo build --release

# TypeScript
cd ts && bun install && bun build src/main.ts --outdir dist --target node
```

## Test

```sh
bash tests/runner.sh
```
