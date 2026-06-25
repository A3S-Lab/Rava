# Known Limitations

This document tracks known limitations and edge cases in the Rava compiler and runtime.
It reflects the **verified** state of the code, not aspirational goals.

## Execution model (important)

Rava has two execution paths that share one IR (RIR):

- **`rava run` / `rava test` → RIR interpreter** (`crates/micrort`). This is the **mature,
  supported path**: 393/393 end-to-end Java tests pass. Treat this as the product today.
- **`rava build` → Cranelift AOT** (`crates/codegen-cranelift`). This is **experimental** —
  it compiles only a subset of programs and miscompiles several basics (see below).

`cargo test --workspace` completes and is green (545 passing, 14 AOT tests quarantined).

## AOT backend (`rava build`) — experimental

- **Loops are miscompiled into infinite loops.** A trivial `for (int i=1;i<=5;i++) sum+=i;`
  produces a native binary that never terminates. Affects `for` / `while` / `do-while` /
  `break`/`continue` / loop-bearing switch. Tests quarantined with `#[ignore]` in
  `crates/codegen-cranelift/tests/aot_e2e.rs`.
- **Generic classes segfault.** Field access on a generic class crashes (exit 139). Root
  cause: a global, flat field-slot map in `translator/mod.rs` (no per-class slots). Test:
  `aot_generic_pair` (`#[ignore]`).
- **Exceptions don't unwind.** `throw` lowers to `trap`/abort; there is no try/catch/finally
  unwinding in AOT.
- **Narrowing casts are silently dropped.** `Convert` is a no-op — `(int)longValue` does not
  truncate, causing silent data corruption.
- **`NewMultiArray` only allocates the first dimension.**
- AOT covers only basic OOP/arithmetic; collections, streams, lambdas, String methods,
  StringBuilder, and most of the standard library are **not** supported in AOT yet. These all
  work on the interpreter path.
- `rava build --target` supports `native` only. `jar`, `jlink`, `docker` are not implemented.

## Optimization passes (`crates/aot/src/passes.rs`)

- Escape analysis, dead-code elimination, and constant folding have real implementations.
- **Inlining is analysis-only** — it identifies candidates but performs no inlining.
- **Escape analysis results are not applied** — nothing is stack-allocated yet.
- **MetadataTableGenPass is scaffolded**: function-pointer resolution, field offsets, real
  method signatures, and superclass extraction are TODO; the table is not embedded in the
  binary. `ProxyPregenPass` (Phase 4) and `MicroRtBridgePass` (Phase 3) are empty stubs.

## MicroRT "dynamic Java" escape hatch — not implemented

The README markets an embedded bytecode runtime that transparently handles dynamic
reflection, dynamic proxies, and dynamic class loading. This is **aspirational (Phase 3+)**:

- The JVM bytecode interpreter, class loader, and bytecode verifier are architecture stubs
  (e.g. `aload`/`invoke*` opcodes return `null`). No `.class` bytecode is ever loaded.
- Dynamic reflection / dynamic proxy / dynamic class loading / JNI are **not implemented**.
- All Java currently executes through the RIR interpreter, not a bytecode runtime.

## Toolchain

- **Dependency resolution is not wired into builds.** `rava add` / `update` / `deps` only edit
  `rava.hcl`; `rava build` never downloads or links the dependency JARs, so projects with
  external dependencies cannot be built. Transitive resolution (POM parsing) is also a stub.
- CLI commands mentioned in the product vision (`lint`, `repl`, `publish`, `doctor`,
  `upgrade`, `export`) are not implemented. Implemented: `run`, `build`, `init`, `add`,
  `remove`, `update`, `deps`, `test`, `fmt`.
- Script mode (running a `.java` file without a `main`) is documented but not implemented.

## History

For the historical investigation and fix details of the previous assignment-in-condition loop
issue, see `docs/assignment-in-condition-investigation.md`.
