# Known Limitations

This document tracks known limitations and edge cases in the Rava compiler and runtime.

## Active Limitations

- `rava build --target` currently supports `native` only. `jar`, `jlink`, and `docker` targets are planned but not implemented.
- CLI commands documented in the product vision (`lint`, `repl`, `publish`, `doctor`, `upgrade`, `export`) are not implemented in the current binary.
- AOT optimization passes are scaffolded but still placeholder implementations (`crates/aot/src/passes.rs`).
- MicroRT bytecode interpreter, reflection engine, and bytecode verifier are present as architecture stubs and currently return "not yet implemented" errors.
- **AOT generic classes**: Generic classes cause a field slot allocation bug that results in segfaults on field access. The lowerer doesn't properly handle field slot assignment for generic type parameters, causing out-of-bounds access. This is a complex issue requiring redesign of how field slots are allocated for parameterized types. (Test: `aot_generic_pair` in `crates/codegen-cranelift/tests/aot_e2e.rs`)

For the historical investigation and fix details of the previous assignment-in-condition loop issue,
see `docs/assignment-in-condition-investigation.md`.
