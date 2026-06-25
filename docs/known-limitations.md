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

## Compiled `.class` execution (bytecode → RIR)

`rava run File.class` and **`rava run File.jar`** execute pre-compiled Java by lowering its JVM
bytecode to RIR and running it on the existing interpreter
(`crates/micrort/src/{classfile,bytecode}.rs`); output matches the JVM. A JAR's `.class` entries
are loaded into one module so cross-class calls link (`bytecode::load_jar`/`load_classes_module`).
**Supported subset:** int/long/float/double arithmetic + conversions + bitwise/shifts, control
flow + loops, static/virtual/special calls + recursion, objects/fields/constructors (incl.
cross-class), arrays, `String` + library method calls (routed to builtins), `System.out.println`,
stack ops (`dup`/`swap`/…), `throw`, and `try`/`catch`. **Not yet:** `invokedynamic` (so string
`+` concatenation and lambdas in compiled code), `tableswitch`/`lookupswitch`, and catching
*library* exceptions (catch matches by class name — user exception types work; built-in types like
`ArithmeticException` need name normalization). So a *self-contained* compiled JAR runs end-to-end,
but typical *Maven* JARs (which use `invokedynamic`) only partially load. The separate JVM-bytecode
VM in `interpreter.rs` remains an unused stub — the bytecode→RIR path supersedes it.

The README's MicroRT "dynamic Java" escape hatch (dynamic reflection / proxy / class loading, JNI)
is still **aspirational — not implemented**.

## Interpreter semantics (verified via differential testing vs OpenJDK 17)

Two known correctness gaps remain (both stem from value representation; deferred because a
fix is invasive and risks regressing the 393-test suite):

- **`int` arithmetic does not wrap at 32 bits.** Integers are held as 64-bit, so
  `Integer.MAX_VALUE + 1` yields `2147483648` instead of Java's `-2147483648`. Programs that
  rely on 32-bit overflow (hashing, checksums) will differ.
- **`char` in arithmetic context concatenates instead of promoting.** `char` is represented as
  a 1-char string, so `int sum = 0; sum += someChar;` concatenates rather than adding the code
  point. Use an explicit `(int)` cast as a workaround. A proper fix needs a distinct char type.
- **`finally` blocks do not run when the `try`/`catch` body `return`s** (they run on normal
  fall-through). High-priority correctness bug.
- **Mixed-type ternary / numeric widening on assignment**: `double d = cond ? 1 : 2.5;` keeps
  the `int` branch as `1` rather than widening to `1.0`. Assigning an int literal to a declared
  `double` is not coerced.
- **`new String[]{ ... }` array-initializer-with-`new` syntax does not parse** (use a separate
  `String[] x = { ... };` declaration, which works).
- `StringBuilder.setLength(n)` is not implemented.
- Explicit reference casts do not throw `ClassCastException` on a bad cast (the cast is a no-op
  in the interpreter).

## Toolchain

- **Dependency resolution is not wired into builds.** `rava add` / `update` / `deps` only edit
  `rava.hcl`; `rava build` never downloads or links the dependency JARs, so projects with
  external dependencies cannot be built (executing `.class` from JARs is the larger blocker).
- **Transitive POM resolution is partial.** `pom::parse_pom_dependencies` +
  `registry::resolve_closure` resolve transitive deps for POMs that declare versions literally
  or via same-POM `${...}` properties, filtering test/provided/optional. They do **not** yet
  resolve **parent POMs**, `<dependencyManagement>`, or BOM imports — so a dependency whose
  version is inherited from a parent (common: Jackson, Spring) is skipped rather than guessed.
- CLI commands mentioned in the product vision (`lint`, `repl`, `publish`, `doctor`,
  `upgrade`, `export`) are not implemented. Implemented: `run`, `build`, `init`, `add`,
  `remove`, `update`, `deps`, `test`, `fmt`.
- Fully-qualified references to builtin types (e.g. `java.util.List.of(...)`) do not resolve;
  use the simple name (`List.of(...)`). Builtins are keyed by simple name.

## History

For the historical investigation and fix details of the previous assignment-in-condition loop
issue, see `docs/assignment-in-condition-investigation.md`.
