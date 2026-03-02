# Rava

> A Rust-written Java AOT compiler and all-in-one toolchain. What Bun is to TypeScript, Rava is to Java.

**Release channel:** `0.1.x-alpha` (developer preview, not production-ready yet)

```
Bun : TypeScript = Rava : Java
```

One binary. Everything included:

```bash
rava run Main.java          # Run Java source directly
rava build                  # AOT compile to native binary
rava init                   # Initialize project (generates rava.hcl)
rava add junit              # Add dependency
rava remove junit           # Remove dependency
rava update                 # Update dependency versions
rava deps tree              # Show dependency entries
rava deps lock              # Generate rava.lock
rava test                   # Run tests
rava fmt                    # Format Java source files
```

---

## The Problem

Java's ecosystem is powerful but heavy. Developers face:

- **Slow startup**: JVM cold start takes seconds — not viable for CLI tools, Serverless, or edge computing
- **High memory**: Even a Hello World occupies 100MB+
- **Fragmented toolchain**: Maven / Gradle / Ant each with their own verbose configs (XML / Groovy / Kotlin DSL)
- **Complex deployment**: Requires pre-installed JRE/JDK; Docker images are bloated (200MB+)
- **Developer experience gap**: Compared to what Bun/Deno offer TypeScript developers, Java's toolchain is a generation behind

## The Solution

Rava compiles Java to native binaries with no JVM required:

```
Traditional Java:  .java → javac → .class → JVM (JIT) → machine code
Rava:              .java → rava  → native binary (AOT)  → direct execution

Startup time:  ~2s     →  ~10ms
Memory usage:  ~200MB  →  ~20MB
```

---

## Quick Start

```bash
# Install (macOS / Linux)
curl -fsSL https://rava.dev/install.sh | sh

# Homebrew
brew install a3s-lab/tap/rava

# From source
cargo install rava
```

```bash
# No JDK. No pom.xml. No build.gradle.
# One file, just run.
echo 'class Main { public static void main(String[] args) { System.out.println("Hello"); } }' > Main.java
rava run Main.java
# → Hello

# Script mode — no main() needed
cat > script.java << 'EOF'
var name = "World";
System.out.println("Hello, " + name + "!");
var list = List.of(1, 2, 3);
list.forEach(System.out::println);
EOF
rava run script.java
# → Hello, World!
# → 1  2  3

# Compile to native binary
rava build
# → target/my-app  (~15MB, starts in ~10ms)

# Cross-compile
rava build --platform linux-amd64
rava build --platform linux-arm64
```

## Release Automation

Tagging a version triggers cross-platform release packaging in GitHub Actions.

```bash
git tag v0.1.1
git push origin v0.1.1
```

The `Release` workflow builds `rava` for Linux, macOS (Intel/Apple Silicon), and Windows,
then publishes archives plus `SHA256SUMS.txt` to the GitHub Release page.

---

## Project Config: `rava.hcl`

HCL replaces XML/Groovy/Kotlin DSL. Human-readable, like `package.json` or `Cargo.toml`:

```hcl
# rava.hcl
project {
  name    = "my-api"
  version = "0.1.0"
  java    = "21"
  license = "MIT"
}

dependencies {
  "spring-boot-web"        = "3.2.0"         # short name
  "com.google.guava:guava" = "^33.0.0-jre"   # semver range
  "io.netty:netty-all"     = {
    version = "4.1.100.Final"
    exclude = ["io.netty:netty-transport-native-epoll"]
  }
}

dev_dependencies {
  "junit"   = "5.10.1"
  "mockito" = "5.8.0"
}

build {
  target   = "native"            # native | jar | jlink | docker
  main     = "com.example.Main"
  optimize = "speed"             # speed | size | debug
}

run {
  args = ["--server.port=8080"]
  env  = { SPRING_PROFILES_ACTIVE = "dev" }
  watch {
    paths = ["src/", "resources/"]
    delay = "500ms"
  }
}

test {
  framework = "junit5"
  parallel  = true
  coverage {
    min_line   = 80
    min_branch = 70
  }
}
```

**Short name aliases** — built-in for common dependencies:

| Short name | Full coordinates |
|------------|-----------------|
| `spring-boot-web` | `org.springframework.boot:spring-boot-starter-web` |
| `spring-boot-data-jpa` | `org.springframework.boot:spring-boot-starter-data-jpa` |
| `lombok` | `org.projectlombok:lombok` |
| `guava` | `com.google.guava:guava` |
| `jackson` | `com.fasterxml.jackson.core:jackson-databind` |
| `slf4j` | `org.slf4j:slf4j-api` |
| `logback` | `ch.qos.logback:logback-classic` |
| `junit` | `org.junit.jupiter:junit-jupiter` |
| `mockito` | `org.mockito:mockito-core` |
| `assertj` | `org.assertj:assertj-core` |

View full list: `rava alias list`

---

## CLI Reference

```
rava <command> [options] [args]

Project:
  run [file]               Run Java source or project
  build                    AOT compile to native binary
  init [name]              Initialize project
  test [pattern]           Run tests
  fmt [files...]           Format code

Dependencies:
  add <package>            Add dependency
  remove <package>         Remove dependency
  update [package]         Update dependencies
  deps tree                Show dependency tree
  deps lock                Generate or verify rava.lock
```

**`rava run` options:**

| Option | Description |
|--------|-------------|
| `--watch` / `-w` | Watch for file changes, auto-restart |
| `-- <args>` | Pass arguments to the program |

**`rava build` options:**

| Option | Description |
|--------|-------------|
| `--target <native\|jar\|jlink\|docker>` | Target flag is accepted; only `native` is implemented |
| `--optimize <speed\|size\|debug>` | Optimization strategy |
| `--platform <target>` | Cross-compile target |
| `-o, --output <path>` | Output binary path |

**Build output comparison:**

| Target | Output | Size | Startup | Requires JVM |
|--------|--------|------|---------|--------------|
| `native` (implemented) | Single binary | ~15MB | ~10ms | No |
| `jar` | Planned | - | - | - |
| `jlink` | Planned | - | - | - |
| `docker` | Planned | - | - | - |

---

## Java Compatibility

### Language Versions

| Java Version | Support | Notes |
|-------------|---------|-------|
| Java 21 (LTS) | Full | Primary target |
| Java 17 (LTS) | Full | All features |
| Java 11 (LTS) | Full | All features |
| Java 8+ | Full | Lambda, generics, annotation processing |

### Language Features

| Feature | AOT Support | Notes |
|---------|-------------|-------|
| Records | ✅ | Java 16+; compact constructors, component accessors |
| Sealed Classes | ✅ | Java 17+ |
| Pattern Matching (instanceof) | ✅ | Java 16+; binding variables, guarded patterns |
| Pattern Matching (switch) | ✅ | Java 21+; type patterns, `case null`, guarded patterns |
| Switch Expressions | ✅ | Java 14+; arrow syntax, `yield` |
| Virtual Threads | ✅ | Java 21+, native AOT support |
| Text Blocks | ✅ | Java 15+ |
| var local variables | ✅ | Java 10+; including `var` lambda parameters |
| Lambda / Stream | ✅ | Java 8+; full Stream API, Collectors |
| Method References | ✅ | Static, instance, unbound, constructor refs |
| Anonymous Classes | ✅ | Including anonymous Comparator |
| Try-with-resources | ✅ | Single and multiple resources |
| Module System | ✅ | `module-info.java` parsing |
| Reflection (statically resolvable) | ✅ | AOT metadata table, faster than JVM reflection |
| Reflection (dynamic) | ✅ | MicroRT metadata engine, automatic fallback |
| Dynamic Proxy (compile-time interfaces) | ✅ | AOT pre-generated proxy classes |
| Dynamic Proxy (runtime interfaces) | ✅ | MicroRT runtime generation |
| Dynamic Class Loading | ✅ | Embedded MicroRT bytecode runtime |
| JNI Outbound (Java → C) | ✅ | AOT native method stubs + dlopen, Phase 1 |
| JNI Inbound (C → JNIEnv*) | ✅ | Full JNI function table (~230 functions), Phase 3 |

### Framework Compatibility

| Framework | Level | Notes |
|-----------|-------|-------|
| Spring Boot 3.x | Tier 1 | Zero config, reflection/dynamic proxy auto-handled |
| Quarkus | Tier 1 | AOT-friendly by design |
| Micronaut | Tier 1 | Compile-time DI, naturally compatible |
| Vert.x | Tier 1 | No reflection dependency |
| Hibernate / JPA | Tier 2 | Reflection + dynamic proxy auto-handled via MicroRT |
| MyBatis | Tier 2 | Mapper dynamic proxy auto-handled via MicroRT |
| Lombok | Tier 1 | Compile-time annotation processing |

---

## Known Limitations

See [docs/known-limitations.md](docs/known-limitations.md) for detailed technical notes.

**Current status:** See `docs/known-limitations.md` for active implementation gaps and planned targets.

---

## MicroRT: How Rava Handles Dynamic Java

This is the fundamental difference between Rava and GraalVM.

**GraalVM's approach** — Closed-World Assumption: the compiler must know all reachable classes at compile time. Anything it can't resolve requires manual configuration files:

```
GraalVM:  AOT compile → hit reflection → error → user writes reflect-config.json → recompile
```

A medium-sized Spring Boot project can easily have 2000+ entries in `reflect-config.json`. Every dependency upgrade may invalidate the config.

**Rava's approach** — AOT primary + embedded bytecode runtime escape hatch:

```
Rava:  AOT compile → hit reflection → auto-mark → MicroRT handles it → user sees nothing
```

Rava embeds a lightweight bytecode runtime (MicroRT, ~3MB) in the native binary. The compiler automatically decides what goes AOT and what falls to MicroRT — the user does nothing.

```
Compiler decision flow:

  Statically resolvable reflection    → AOT metadata table (zero overhead)
  Dynamic unresolvable reflection     → auto-mark → MicroRT runtime
  Compile-time known proxy interface  → AOT pre-generated proxy (zero overhead)
  Runtime-determined proxy            → MicroRT runtime generation
  Dynamic class loading (unknown)     → MicroRT bytecode interpret/JIT
  Hot interpreted code                → MicroRT auto-JIT → near-AOT speed
```

No `reflect-config.json`. No `proxy-config.json`. No code changes.

**MicroRT components** (~3MB total):

| Component | Size | Role |
|-----------|------|------|
| Bytecode Interpreter | ~500KB | Execute Java bytecode (Rust `match` dispatch) |
| Cranelift JIT | ~2MB | Compile hot paths to native code (Phase 5) |
| Class Loader | ~200KB | Bootstrap → Platform → Application delegation |
| Reflection Metadata Engine | ~100KB | Runtime metadata queries |
| Bytecode Verifier | ~150KB | StackMapTable verification, type safety |

**Performance impact:**

| Scenario | Impact |
|----------|--------|
| Pure AOT code | Zero |
| Reflection on AOT-known class | Near-zero (metadata table lookup) |
| Reflection on unknown class | First call slow (MicroRT loads), subsequent calls cached |
| Dynamic proxy (compile-time-known interfaces) | Zero (AOT pre-generated) |
| Dynamic class loading | Interpreted speed; hot paths JIT to near-AOT |

**Binary size:**

```
AOT compiled application code   ~15MB
AOT compiled dependency code    ~10MB
MicroRT (interpreter)            ~3MB
Reflection metadata table        ~2MB
─────────────────────────────────────
Total                           ~30MB   ← vs JVM 200MB
```

---

## Architecture

Rava follows the **minimal core + external extensions** principle. 7 non-replaceable core components, 10 extension points (all with working defaults).

**Core components (7):**

| # | Component | Crate | Role |
|---|-----------|-------|------|
| 1 | Frontend | `rava-frontend` | Java source → RIR (Lexer, Parser, TypeChecker, SemanticAnalyzer) |
| 2 | RIR | `rava-rir` | Rava Intermediate Representation — SSA-form IR, shared contract between frontend and backends |
| 3 | AOT Backend | `rava-aot` + `rava-codegen-cranelift` | RIR → native machine code via 7-pass optimizer + Cranelift |
| 4 | MicroRT | `rava-micrort` | Escape hatch: bytecode interpreter, class loader, verifier, reflection engine |
| 5 | UnifiedHeap | `rava-heap` | Shared heap + GC for AOT and MicroRT objects; unified object model |
| 6 | PackageManager | `rava-pkg` | Dependency resolution, Maven Central, `rava.lock` |
| 7 | CLI | `rava` (cli) | User entry point, zero-config experience |

**Extension points (10, all trait-based with defaults):**

| Trait | Default | Purpose |
|-------|---------|---------|
| `CodegenBackend` | `CraneliftBackend` | AOT code generation; swap for LLVM |
| `OptPass` | 7-pass chain | Optimization passes; add custom passes |
| `BytecodeDispatcher` | `match` dispatch | MicroRT dispatch strategy |
| `GcStrategy` | `NoopGc` → `StopTheWorldGc` | Garbage collection algorithm |
| `KlassDescriptor` | `AotKlass` / `MicroRtKlass` | Unified type descriptor for AOT ↔ MicroRT interop |
| `ClassResolver` | `MavenCentralResolver` | Maven dependency resolution |
| `ClassLoader` | Three-tier delegation | Bytecode loading (Bootstrap → Platform → App) |
| `DiagnosticEmitter` | `TerminalEmitter` | Compiler error output |
| `Parser` | Java 21 parser | Java source → AST |
| `Lowerer` | RIR lowerer | Typed AST → RIR |

**Crate structure:**

```
crates/
├── common/              # RavaError, Span, JavaType, Diagnostic
├── rir/                 # RirModule / RirFunction / BasicBlock / RirInstr (SSA)
├── heap/                # UnifiedHeap, ObjectHeader (mark word), KlassDescriptor, GcStrategy
├── frontend/            # Compiler + Parser / TypeChecker / Lowerer traits
│   └── src/
│       ├── parser/      # mod.rs, class.rs, types.rs, stmt.rs, expr.rs
│       └── lowerer/     # mod.rs, stmt.rs, expr.rs, helpers.rs, tests.rs
├── aot/                 # AotCompiler + 7 named OptPasses + CodegenBackend trait
├── codegen-cranelift/   # CraneliftBackend: RIR → object file → native binary
│   └── src/translator/  # mod.rs, helpers.rs
├── micrort/             # Interpreter + ClassLoader + BytecodeVerifier + ReflectionEngine
│   └── src/
│       ├── rir_interp/  # mod.rs, rval.rs, interp.rs, helpers.rs, objects.rs
│       └── builtins/    # mod.rs, format.rs, math.rs, numbers.rs, string.rs,
│                        # system.rs, collections.rs, io.rs, concurrent.rs,
│                        # reflect.rs, network.rs
├── hcl/                 # HCL parsing/generation (rava.hcl, rava.lock)
├── pkg/                 # ProjectConfig, DependencyGraph, Lockfile, ShortNameRegistry
└── cli/                 # rava binary (run / build / init / add / test / fmt)
```

**Dependency rules (one-way, no cycles):**

```
cli       → frontend, aot, codegen-cranelift, pkg, hcl
frontend  → rir, common
aot       → rir, common
micrort   → heap, rir, common
heap      → common
pkg       → hcl, common
hcl       → common
rir       → common
common    → (none)
```

**AOT compilation pipeline:**

```
.java source
  → Frontend (Lexer → Parser → TypeChecker → Lowerer)
  → Analysis Passes (ReflectionPass, ProxyPass, ClassLoadPass)
  → RIR (SSA form)
  → AOT Optimizer:
      1. EscapeAnalysisPass   — stack vs heap allocation decision (implemented)
      2. InliningPass         — identify and inline small/hot methods (implemented)
      3. DeadCodeElimPass     — remove unreachable blocks and dead values (implemented)
      4. ConstFoldingPass     — evaluate constants at compile time (implemented)
      5. MetadataTableGenPass — embed reflection metadata (Phase 2, scaffolded)
      6. ProxyPregenPass      — pre-generate proxy classes (Phase 4, pending)
      7. MicroRtBridgePass    — generate MicroRT interop stubs (Phase 3, pending)
  → CraneliftBackend: CLIF IR → machine code → .o file
  → System linker (cc): .o → native binary
```

---

## Performance Targets

| Metric | Target | vs Traditional |
|--------|--------|----------------|
| Single-file run startup | < 200ms | javac + java: ~2s |
| AOT binary startup | < 20ms | JVM: ~2s, GraalVM native: ~50ms |
| Incremental compile | < 1s (single file change) | Maven: ~5s |
| Full compile (medium project) | < 30s | Maven: ~60s |
| Memory (AOT binary, medium web app) | < 50MB | JVM: ~200MB |
| Binary size (medium web app) | < 30MB | GraalVM: ~60MB |
| Dependency resolution (cached) | < 2s | Maven: ~10s |
| Rava self binary size | < 50MB | GraalVM: ~400MB |

---

## Implementation Roadmap

| Phase | Deliverable | Status |
|-------|------------|--------|
| Framework | Workspace skeleton: 10 crates, all traits defined, Cranelift wired up | ✅ |
| Phase 1 (6-12mo) | Basic AOT: `rava run`, `rava build`, `rava add/remove/update`, `rava deps`, `rava init`, static Java | ✅ (lexer ✅, parser ✅, type checker ✅, lowerer ✅; RIR interpreter ✅; builtins ✅ (String, Math, Collections, Format, I/O, Concurrency, Reflection, Network, Time, Regex); CLI ✅ (`run`, `build`, `init`, `add`, `remove`, `update`, `deps`, `test`, `fmt`); CI/CD ✅; 393/393 e2e tests 100%) |
| Phase 2 (3-6mo) | Reflection: AOT metadata table + dual-path dispatch | 🚧 (RIR metadata structures ✅; MetadataTableGenPass scaffolded ✅; function pointer resolution and field offsets pending) |
| Phase 3 (6-12mo) | MicroRT v1: bytecode interpreter + class loader + unified object model | ⬜ |
| Phase 4 (2-3mo) | Dynamic proxy AOT: pre-generated proxy classes | ⬜ |
| Phase 5 (6-12mo) | MicroRT JIT: Cranelift JIT for hot interpreted code | ⬜ |

### Near-Term Development Plan (Java Coverage)

Current status: parser and type checker cover common Java syntax; generic inference and overload resolution significantly improved; 393/393 e2e tests passing. Full JLS-level parity is the next target.

**P0 — Runtime Semantics Completion** *(do first)*

- [ ] Implement missing interpreter semantics paths in MicroRT
- [ ] Complete verifier behavior for stricter Java correctness checks
- [ ] Expand reflection / runtime metadata behavior to match expected Java usage

**P1 — Type System and Resolution Parity**

- [ ] Refine generic type inference in complex nested and overloaded scenarios
- [ ] Extend overload resolution toward closer JLS method-selection parity
- [ ] Improve bound checking and edge-case diagnostics for generic constraints

**P2 — Syntax and Frontend Completeness**

- [ ] Chapter-by-chapter JLS parser audit — fill syntax edge gaps
- [ ] Complete annotation semantics pipeline beyond declaration parsing
- [ ] Expand module system semantic checks for `module-info` directives

**P3 — Standard Library and Behavioral Compatibility**

- [ ] Increase long-tail JDK API compatibility coverage in runtime behavior
- [ ] Add behavior-focused regression tests for subtle API / exception differences
- [ ] Validate edge-case parity with representative Java reference outputs

**Execution order:** P0 → P1 → P2 → P3.
**Gate:** `cargo test --workspace` must stay green after every change.

---

## Test Coverage

559 tests passing (`cargo test --workspace`):

| Crate | Tests |
|-------|-------|
| `rava-aot` | 2 — 7 passes registered in correct order |
| `rava-codegen-cranelift` | 40 — AOT e2e (38: hello world, arithmetic, control flow, classes, fields, inheritance, arrays, strings, exceptions, generics, static methods), translator helpers (2) |
| `rava-frontend` | 90 — checker (38: generic type params, overload resolution, bounds validation, duplicate detection), lexer (10: hex/binary literals, char, operators, keywords), parser (29: hello world, local var, do-while, for-each, break/continue, try/catch, lambda, enum, instanceof pattern, method ref, record, sealed class, text block, switch arrow, yield, module-info, guarded patterns, case null), lowerer (9: hello world, arithmetic, do-while, break/continue, ternary, for-each, record pattern), compiler/resolver (4) |
| `rava-heap` | 6 — object header, GC strategy |
| `rava-micrort` | 22 — builtin dispatch (math, string, collections, format, I/O, concurrency, reflection, network, time, regex) |
| `rava-micrort` (e2e) | 393/393 (100%) — comprehensive Java language feature tests covering classes, inheritance, generics, lambdas, streams, collections, exceptions, pattern matching, switch expressions, records, sealed classes, text blocks, annotations, reflection, concurrency, I/O, networking, and more |
| `rava-pkg` | 4 — config parsing, lockfile, shortname registry |
| `rava-rir` | 1 — module construction |
| `rava` (cli) | 1 — PascalCase conversion |

---

## Comparison

| Dimension | Bun (TypeScript) | Rava (Java) |
|-----------|-----------------|-------------|
| Implementation language | Zig + C++ | Rust |
| Replaces | Node.js + npm + webpack | JDK + Maven/Gradle + GraalVM |
| Run | `bun run index.ts` | `rava run Main.java` |
| Build | `bun build` | `rava build` |
| Package management | `bun add express` | `rava add spring-boot-web` |
| Config | `package.json` | `rava.hcl` |
| Lock file | `bun.lockb` | `rava.lock` |
| Package registry | npm | Maven Central |

| Dimension | GraalVM native-image | Rava |
|-----------|---------------------|------|
| Reflection | `reflect-config.json` (manual) | Zero config (auto) |
| Dynamic proxy | `proxy-config.json` (manual) | Zero config (auto) |
| Dynamic class loading | Not supported | MicroRT bytecode interpreter |
| Spring Boot | Requires `spring-aot` plugin | Zero config |
| User experience | Config errors → runtime crash | User sees nothing |
| Architecture | Closed-World Assumption | AOT + open escape hatch |
| Toolchain size | ~400MB | < 50MB |

---

## Platform Support

| Platform | Support |
|----------|---------|
| Linux x86_64 | Tier 1 (CI tested) |
| Linux aarch64 | Tier 1 (CI tested) |
| macOS x86_64 | Tier 1 (CI tested) |
| macOS aarch64 (Apple Silicon) | Tier 1 (CI tested) |
| Windows x86_64 | Tier 2 (community tested) |

---

## Docs

- [`docs/rava.md`](docs/rava.md) — Full PRD + Technical Architecture

---

> Rava's goal: give Java developers the same experience Bun/Deno users have — one binary, zero-config execution, fast compilation, human-readable project config. Java programs that start in 10ms, use 20MB of memory, and deploy as a single file.

---

## License

MIT © [A3S Lab](https://github.com/A3S-Lab)

See [LICENSE](LICENSE) for the full text.
