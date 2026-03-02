# Rava — Product Requirements & Technical Architecture

> Version: 0.1.0-alpha | Date: 2026-03-01 | Author: A3S Team

---

# Part I — Product Requirements

## 1. Overview

### 1.1 Problem Statement

The Java ecosystem is powerful but unwieldy. Developers face:

- **Slow startup**: JVM cold-start takes seconds — unsuitable for CLI tools, serverless, and edge computing
- **High memory footprint**: Even a Hello World consumes 100 MB+
- **Fragmented toolchain**: Maven / Gradle / Ant each do things their own way, with verbose config (XML / Groovy / Kotlin DSL)
- **Painful dependency management**: `pom.xml` files balloon to hundreds of lines; dependency conflicts are a daily headache
- **Complex deployment**: Requires a pre-installed JRE/JDK; Docker images are bloated (200 MB+)
- **Outdated developer experience**: Compared to what Bun/Deno deliver for TypeScript, the Java toolchain feels like the previous decade

### 1.2 Product Vision

**Rava** is a Java AOT compiler and all-in-one toolchain written in Rust. It is to Java what Bun is to TypeScript.

```
Bun : TypeScript = Rava : Java

One binary. Everything included:
  rava run Main.java          → run Java source directly
  rava build                  → AOT-compile to a native binary
  rava init                   → initialize a project (generates rava.hcl)
  rava add spring-boot-web    → add a dependency
  rava test                   → run tests
  rava fmt                    → format code
```

### 1.3 Core Principles

**Principle 1: Zero-configuration out of the box.**

```bash
# No JDK installation, no pom.xml, no build.gradle needed
# One file, just run it
echo 'class Main { public static void main(String[] args) { System.out.println("Hello"); } }' > Main.java
rava run Main.java
# → Hello
```

**Principle 2: One binary replaces the entire toolchain.**

| Traditional Java | Rava |
|-----------------|------|
| JDK (javac) | `rava` |
| Maven / Gradle | `rava` |
| GraalVM native-image | `rava build` |
| JUnit + Maven Surefire | `rava test` |
| google-java-format | `rava fmt` |
| jlink / jpackage | `rava build --bundle` |

**Principle 3: HCL replaces XML/Groovy — human-readable project config.**

```hcl
# rava.hcl — project configuration (analogous to package.json / Cargo.toml)
project {
  name    = "my-api"
  version = "0.1.0"
  java    = "21"

  authors = ["[name] <[email]>"]
  license = "MIT"
}

dependencies {
  "org.springframework.boot:spring-boot-starter-web" = "3.2.0"
  "com.google.guava:guava"                           = "33.0.0-jre"
}

dev_dependencies {
  "org.junit.jupiter:junit-jupiter" = "5.10.1"
  "org.mockito:mockito-core"        = "5.8.0"
}

build {
  target   = "native"       # native | jar | jlink
  main     = "com.example.Main"
  optimize = "speed"        # speed | size | debug
}

run {
  args = ["--server.port=8080"]
  env  = {
    SPRING_PROFILES_ACTIVE = "dev"
  }
}
```

**Principle 4: AOT-first, JIT optional.**

```
Traditional Java:  .java → javac → .class → JVM (JIT) → machine code
Rava:              .java → rava  → native binary (AOT)  → direct execution

Startup time:  traditional ~2s   →  Rava ~10ms
Memory usage:  traditional ~200MB →  Rava ~20MB
```

### 1.4 Target Users

| User Group | Pain Point | Rava Value |
|------------|-----------|------------|
| Java backend developers | Complex toolchain, slow startup | One-command run, instant startup |
| Cloud-native / serverless developers | JVM cold-start unsuitable for Lambda | AOT compilation, 10 ms startup |
| CLI tool developers | Java is awkward for CLI | Compile to single-file native binary |
| Microservice developers | Bloated Docker images | Compile to scratch image (<20 MB) |
| Java beginners | Complex environment setup | Install rava and start coding immediately |

---

## 2. Core Capabilities

### 2.1 Capability Overview

| # | Capability | Command | Description |
|---|-----------|---------|-------------|
| 1 | Direct run | `rava run` | Run .java source directly, no pre-compilation |
| 2 | AOT compile | `rava build` | Compile to native binary or optimized JAR |
| 3 | Project management | `rava init` | Initialize project, generate rava.hcl |
| 4 | Dependency management | `rava add/remove/update` | Manage Maven dependencies, auto-resolve transitive deps |
| 5 | Test runner | `rava test` | Run JUnit tests |
| 6 | Code formatter | `rava fmt` | Format Java code |
| 7 | Linter | `rava lint` | Static analysis and code quality checks |
| 8 | REPL | `rava repl` | Interactive Java REPL |
| 9 | Script mode | `rava script` | Single-file script execution (auto-infers main) |
| 10 | Publish | `rava publish` | Publish to Maven Central or private registry |

### 2.2 Capability 1: Direct Run (`rava run`)

Run Java like a script, no manual compilation:

```bash
# Single-file run
rava run Main.java

# Project run (reads main class from rava.hcl)
rava run

# Run with arguments
rava run Main.java -- --port 8080

# Watch mode: restart on file changes (development)
rava run --watch
```

**Run strategies:**

| Scenario | Strategy | Notes |
|----------|----------|-------|
| Single file, no dependencies | Direct AOT compile-and-run | Fastest path, ~100 ms startup |
| Single file with imports | Auto-resolve stdlib + rava.hcl deps | Transparent dependency injection |
| Project directory (rava.hcl present) | Incremental compile + run | Recompiles only changed files |
| `--watch` mode | File watching + hot reload | Developer experience first |
| `--jit` flag | Fall back to JIT mode | Compatibility first (when specific JVM features are needed) |

**Script mode** (no `public static void main` required):

```java
// script.java — Rava auto-wraps this as an executable entry point
var name = "World";
System.out.println("Hello, " + name + "!");

// Top-level imports auto-available
import java.util.*;
import java.io.*;
var list = List.of(1, 2, 3);
list.forEach(System.out::println);
```

```bash
rava run script.java
# → Hello, World!
# → 1
# → 2
# → 3
```

### 2.3 Capability 2: AOT Compilation (`rava build`)

Compile Java source to a native binary that runs without a JVM:

```bash
# Compile to native binary (default)
rava build
# → target/my-api  (native executable, ~15 MB)

# Compile to optimized JAR
rava build --target jar
# → target/my-api.jar

# Compile to jlink slim runtime
rava build --target jlink
# → target/my-api/  (slim JRE included, ~40 MB)

# Cross-compilation
rava build --platform linux-amd64
rava build --platform linux-arm64
rava build --platform macos-amd64
rava build --platform windows-amd64

# Optimization options
rava build --optimize speed   # optimize for execution speed (default)
rava build --optimize size    # optimize for binary size
rava build --optimize debug   # retain debug information
```

**Output comparison:**

| Target | Output | Size | Startup | Requires JVM |
|--------|--------|------|---------|-------------|
| `native` (default) | Single-file binary | ~15 MB | ~10 ms | No |
| `jar` | Executable JAR | ~5 MB + deps | ~2 s | Yes |
| `jlink` | Slim JRE + JAR | ~40 MB | ~1 s | No (bundled) |
| `docker` | Scratch container image | ~20 MB | ~10 ms | No |

**AOT compilation pipeline:**

```
.java source
  → Rava frontend (parse → AST → type check → semantic analysis)
  → Rava Intermediate Representation (RIR)
  → Optimization passes (escape analysis, inlining, dead code elimination, constant folding)
  → Backend code generation (LLVM / Cranelift)
  → Linking (static-link stdlib + dependencies)
  → Native binary
```

### 2.4 Capability 3: Project Management (`rava init`)

```bash
# Initialize empty project
rava init my-project
# → my-project/
#   ├── rava.hcl
#   ├── src/
#   │   └── Main.java
#   └── test/
#       └── MainTest.java

# Initialize from template
rava init my-api --template spring-web
rava init my-cli --template cli
rava init my-lib --template library

# Migrate from existing Maven/Gradle project
rava init --from-maven     # reads pom.xml, generates rava.hcl
rava init --from-gradle    # reads build.gradle, generates rava.hcl
```

**Project layout:**

```
my-project/
├── rava.hcl                 # project config (the only config file)
├── rava.lock                # dependency lockfile (auto-generated)
├── src/                     # source directory
│   └── com/example/
│       └── Main.java
├── test/                    # test directory
│   └── com/example/
│       └── MainTest.java
├── native/                  # native library bindings (.a, .so, .h) — optional
│   ├── libsqlite3.a
│   └── jni_bridge.c
├── resources/               # resource files
│   └── application.properties
└── target/                  # build output (auto-generated)
    ├── cache/               # incremental compilation cache
    └── my-project           # compiled output
```

### 2.5 Capability 4: Dependency Management (`rava add/remove/update`)

```bash
# Add dependency
rava add spring-boot-starter-web
rava add com.google.guava:guava@33.0.0-jre
rava add lombok --dev

# Remove dependency
rava remove guava

# Update dependencies
rava update                  # update all deps to latest compatible version
rava update guava            # update specific dependency
rava update --latest         # update to absolute latest (may have breaking changes)

# View dependency tree
rava deps tree
# → com.example:my-api@0.1.0
#   ├── org.springframework.boot:spring-boot-starter-web@3.2.0
#   │   ├── org.springframework.boot:spring-boot-starter@3.2.0
#   │   ├── org.springframework.boot:spring-boot-starter-json@3.2.0
#   │   └── org.springframework.boot:spring-boot-starter-tomcat@3.2.0
#   └── com.google.guava:guava@33.0.0-jre

# Check outdated dependencies
rava deps outdated

# Audit for security vulnerabilities
rava deps audit
```

**Dependency resolution:**

| Feature | Description |
|---------|-------------|
| Registries | Maven Central (default), custom private registries |
| Version ranges | Semantic versioning (`^3.2.0`, `~3.2.0`, `>=3.2.0,<4.0.0`) |
| Transitive deps | Auto-resolved; highest compatible version wins on conflict |
| Lockfile | `rava.lock` guarantees reproducible builds |
| Local cache | `~/.rava/cache/` global cache, shared across projects |
| Offline mode | `rava add --offline` uses only local cache |
| Short names | Common deps have aliases (`spring-boot-web` → `org.springframework.boot:spring-boot-starter-web`) |

**rava.hcl dependency config:**

```hcl
dependencies {
  # Standard format
  "org.springframework.boot:spring-boot-starter-web" = "3.2.0"

  # Version range
  "com.google.guava:guava" = "^33.0.0-jre"

  # Detailed config
  "io.netty:netty-all" = {
    version  = "4.1.100.Final"
    exclude  = ["io.netty:netty-transport-native-epoll"]
  }

  # Local path dependency (monorepo)
  "my-common" = { path = "../common" }

  # Git dependency
  "my-lib" = {
    git    = "https://github.com/org/my-lib.git"
    branch = "main"
  }
}
```

**Built-in short name aliases:**

| Short name | Full coordinates |
|-----------|-----------------|
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

The short-name registry is viewable via `rava alias list` and supports user-defined extensions.

### 2.6 Capability 5: Test Runner (`rava test`)

```bash
# Run all tests
rava test

# Run a specific test class
rava test MainTest

# Run a specific test method
rava test MainTest::testHello

# Run tests matching a pattern
rava test --filter "*.service.*"

# Watch mode (auto re-run on file changes)
rava test --watch

# Generate coverage report
rava test --coverage
# → Coverage: 85.3% (lines), 78.2% (branches)
# → Report: target/coverage/index.html
```

**Supported test frameworks:**

| Framework | Support | Notes |
|-----------|---------|-------|
| JUnit 5 | Native | Default test framework |
| JUnit 4 | Compatible | Auto-detected and adapted |
| TestNG | Plugin | Configured via rava.hcl |
| Assertion libraries | Transparent | AssertJ, Hamcrest, etc. work directly |

### 2.7 Capabilities 6–7: Format & Lint (`rava fmt` / `rava lint`)

```bash
# Format
rava fmt                     # format all .java files
rava fmt --check             # check only, no changes (for CI)
rava fmt src/Main.java       # format a specific file

# Lint
rava lint                    # run all lint rules
rava lint --fix              # auto-fix fixable issues
```

### 2.8 Capability 8: Interactive REPL (`rava repl`)

```bash
rava repl
# → Rava REPL v0.1.0 (Java 21)
# → Type :help for help, :quit to exit
#
# rava> var x = 42;
# rava> System.out.println(x * 2);
# 84
# rava> import java.util.stream.*;
# rava> IntStream.range(0, 5).map(i -> i * i).toArray()
# [0, 1, 4, 9, 16]
# rava> :quit
```

### 2.9 Capabilities 9–10: Publish (`rava publish`)

```bash
# Publish to Maven Central
rava publish

# Publish to private registry
rava publish --registry company

# Dry run (no actual publish)
rava publish --dry-run
```

---

## 3. rava.hcl Reference

### 3.1 Top-Level Structure

```hcl
# rava.hcl — Rava project configuration file

project {
  # Project metadata
}

dependencies {
  # Runtime dependencies
}

dev_dependencies {
  # Development/test dependencies
}

repositories {
  # Maven repository sources
}

build {
  # Compilation config
}

run {
  # Run config
}

test {
  # Test config
}

publish {
  # Publish config
}
```

### 3.2 `project` Block

```hcl
project {
  name        = "my-api"                    # project name (required)
  group       = "com.example"               # group ID (required for publishing)
  version     = "0.1.0"                     # semantic version (required)
  java        = "21"                        # Java version (required)
  description = "My awesome API"            # project description
  license     = "MIT"                       # license
  homepage    = "https://github.com/..."    # project homepage
  repository  = "https://github.com/..."    # source repository

  authors = [
    "[name] <[email]>"
  ]
}
```

### 3.3 `dependencies` / `dev_dependencies` Blocks

```hcl
dependencies {
  # Short name + version
  "spring-boot-web" = "3.2.0"

  # Standard format: groupId:artifactId = version
  "com.google.guava:guava" = "33.0.0-jre"

  # Version ranges
  "org.slf4j:slf4j-api" = "^2.0.0"        # >=2.0.0, <3.0.0
  "io.netty:netty-all"  = "~4.1.100"      # >=4.1.100, <4.2.0

  # Detailed config
  "com.fasterxml.jackson.core:jackson-databind" = {
    version = "2.16.0"
    exclude = [
      "com.fasterxml.jackson.core:jackson-annotations"
    ]
  }

  # Local path dependency (monorepo)
  "my-common" = { path = "../common" }

  # Git dependency
  "my-lib" = {
    git    = "https://github.com/org/my-lib.git"
    branch = "main"
  }
}
```

### 3.4 `build` Block

```hcl
build {
  target   = "native"              # native | jar | jlink | docker
  main     = "com.example.Main"    # main class (auto-detected, overridable)
  optimize = "speed"               # speed | size | debug

  # AOT compilation options
  aot {
    reflection_config = "reflect-config.json"   # reflection config (optional)
    initialize_at_build_time = [                 # classes to initialize at build time
      "org.slf4j"
    ]
    enable_preview = true                        # enable preview features
  }

  # Docker build options (when target = "docker")
  docker {
    base_image = "scratch"          # base image
    tag        = "my-api:latest"
    expose     = [8080]
    labels     = {
      maintainer = "[name]"
    }
  }

  # JNI native library config
  jni {
    link      = ["sqlite3", "z", "crypto"]    # link these native libraries
    lib_paths = ["native/", "/usr/local/lib"] # search paths
    static    = ["sqlite3"]                   # force static link (rest are dynamic)
    # Dynamic libs are loaded at runtime via System.loadLibrary() / dlopen
  }

  # Cross-compilation targets
  platforms = ["linux-amd64", "linux-arm64", "macos-amd64"]
}
```

### 3.5 `run` Block

```hcl
run {
  main = "com.example.Main"        # main class (overrides build.main)
  args = ["--server.port=8080"]    # program arguments

  env = {
    SPRING_PROFILES_ACTIVE = "dev"
    DATABASE_URL           = "jdbc:postgresql://localhost:5432/mydb"
  }

  # Development watch config
  watch {
    paths   = ["src/", "resources/"]
    exclude = ["*.class", "target/"]
    delay   = "500ms"
  }

  # JIT fallback config (used with rava run --jit)
  jvm {
    heap_min = "256m"
    heap_max = "1g"
    options  = ["-XX:+UseG1GC"]
  }
}
```

### 3.6 `test` Block

```hcl
test {
  framework = "junit5"             # junit5 | junit4 | testng
  parallel  = true                 # run tests in parallel
  timeout   = "60s"                # per-test timeout

  coverage {
    enabled    = true
    min_line   = 80                # minimum line coverage (%)
    min_branch = 70                # minimum branch coverage (%)
    exclude    = ["**/generated/**"]
  }
}
```

### 3.7 `publish` Block

```hcl
publish {
  registry = "maven_central"       # target registry
  sign     = true                  # GPG signing

  pom {
    # Extra POM info (required for Maven Central)
    scm {
      url        = "https://github.com/org/repo"
      connection = "scm:git:git://github.com/org/repo.git"
    }
    developers = [
      {
        id    = "dev1"
        name  = "[name]"
        email = "[email]"
      }
    ]
  }
}
```

---

## 4. Java Compatibility

### 4.1 Language Version Support

| Java Version | Support Level | Notes |
|-------------|--------------|-------|
| Java 21 (LTS) | Full | Primary target — all features |
| Java 17 (LTS) | Full | All features |
| Java 11 (LTS) | Full | All features |
| Java 8+ | Full | Lambda, generics, annotation processing, all features |

### 4.2 Language Feature Support

| Feature | AOT Support | Notes |
|---------|-------------|-------|
| Records | ✅ | Java 16+ |
| Sealed Classes | ✅ | Java 17+ |
| Pattern Matching | ✅ | Java 21+ |
| Virtual Threads | ✅ | Java 21+, natively supported in AOT |
| Text Blocks | ✅ | Java 15+ |
| Switch Expressions | ✅ | Java 14+ |
| `var` local variables | ✅ | Java 10+ |
| Lambda / Stream | ✅ | Java 8+ |
| Generics | ✅ | Type erasure handled at compile time |
| Annotation processing | ✅ | Compile-time execution (Lombok, etc.) |
| Reflection (statically resolvable) | ✅ Full | AOT metadata table, zero config, faster than JVM reflection |
| Reflection (dynamically unresolvable) | ✅ Full | MicroRT metadata engine, auto-fallback, no reflect-config needed |
| Dynamic proxy (compile-time known interfaces) | ✅ Full | AOT pre-generated proxy classes, zero runtime overhead |
| Dynamic proxy (runtime interfaces) | ✅ Full | MicroRT runtime generation, auto-fallback |
| Dynamic class loading | ✅ Full | Embedded MicroRT bytecode runtime, no JVM required |
| JNI — Outbound (Java → C) | ✅ Full | Phase 1: AOT generates native method stubs, `System.loadLibrary()` via dlopen, `.a` static linking via `rava.hcl` |
| JNI — Inbound (C → JNIEnv\*) | ✅ Full | Phase 3: MicroRT provides full JNI function table (~230 functions), `JavaVM*` singleton, `JNI_OnLoad`/`JNI_OnUnload` lifecycle |

### 4.3 Framework Compatibility

| Framework | Compatibility | Notes |
|-----------|--------------|-------|
| Spring Boot 3.x | Tier 1 | Reflection/proxy config handled automatically |
| Quarkus | Tier 1 | AOT-friendly by design |
| Micronaut | Tier 1 | Compile-time DI, naturally suited to AOT |
| Vert.x | Tier 1 | No reflection dependency |
| Jakarta EE | Tier 2 | Some features require configuration |
| Hibernate | Tier 2 | Requires reflection config |
| MyBatis | Tier 2 | Requires reflection config |
| Lombok | Tier 1 | Compile-time annotation processing, fully compatible |

### 4.4 Reflection Auto-Detection

The biggest challenge with AOT compilation is reflection. Rava handles it automatically through multiple layers:

```
1. Static analysis:    scan source for Class.forName(), .getMethod(), etc.
2. Framework adapters: built-in reflection rules for Spring/Hibernate/Jackson
3. Annotation scan:    @Component, @Entity, @JsonProperty, etc. auto-registered
4. Runtime tracing:    rava run --trace-reflection records actual reflection calls
5. Manual override:    reflect-config.json as a last resort
```

### 4.5 Interoperability with Existing Projects

```bash
# Migrate from Maven
cd existing-maven-project
rava init --from-maven
# → reads pom.xml, generates rava.hcl
# → preserves src/main/java and src/test/java layout
# → optionally migrates to flat src/ and test/ layout

# Migrate from Gradle
rava init --from-gradle
# → reads build.gradle(.kts), generates rava.hcl

# Export pom.xml (reverse compatibility)
rava export maven
# → generates pom.xml for use in traditional Maven environments

# Mixed use: Rava manages deps, Maven/Gradle builds
rava export maven --sync
# → continuously syncs rava.hcl → pom.xml
```

---

## 5. CLI Reference

### 5.1 Command Overview

```
rava <command> [options] [args]

Project management:
  init [name]              initialize a new project
  run [file]               run Java source or project
  build                    compile project
  test [pattern]           run tests
  fmt [files...]           format code
  lint [files...]          lint code
  repl                     interactive REPL
  clean                    clean build artifacts

Dependency management:
  add <package>            add dependency
  remove <package>         remove dependency
  update [package]         update dependency
  deps tree                show dependency tree
  deps outdated            check for outdated deps
  deps audit               security vulnerability audit
  alias list               view short-name mappings

Publish:
  publish                  publish to registry
  export maven             export pom.xml

Tools:
  upgrade                  upgrade Rava itself
  doctor                   diagnose environment issues
  config                   manage global config
  completions <shell>      generate shell completion scripts
```

### 5.2 Global Options

| Option | Description |
|--------|-------------|
| `--verbose` / `-v` | Verbose output |
| `--quiet` / `-q` | Silent mode |
| `--color <auto\|always\|never>` | Color output |
| `--help` / `-h` | Help |
| `--version` / `-V` | Version |

### 5.3 Key Command Reference

**`rava run`:**

| Option | Description |
|--------|-------------|
| `--watch` / `-w` | Watch for file changes, auto-restart |
| `--jit` | Run in JIT mode (fall back to JVM) |
| `--release` | Run with release optimization level |
| `--env <KEY=VALUE>` | Set environment variables |
| `-- <args>` | Arguments passed to the program |

**`rava build`:**

| Option | Description |
|--------|-------------|
| `--target <native\|jar\|jlink\|docker>` | Compile target, default `native` |
| `--optimize <speed\|size\|debug>` | Optimization strategy, default `speed` |
| `--platform <target>` | Cross-compilation target platform |
| `--output` / `-o` | Output path |
| `--static` | Fully static linking (Linux musl) |

**`rava add`:**

| Option | Description |
|--------|-------------|
| `--dev` / `-D` | Add as dev dependency |
| `--exact` | Use exact version (no `^`) |
| `--offline` | Use local cache only |

---

## 6. Global Configuration

### 6.1 Config File Locations

```
~/.rava/
├── config.hcl              # global config
├── cache/                   # dependency cache
│   └── repository/          # Maven repository cache
├── toolchains/              # JDK toolchains (for JIT fallback)
└── aliases.hcl              # user-defined short names
```

### 6.2 Global Config (`~/.rava/config.hcl`)

```hcl
# Default Java version
default_java = "21"

# Default compile target
default_target = "native"

# Proxy config
proxy {
  http     = "http://proxy.company.internal:8080"
  https    = "http://proxy.company.internal:8080"
  no_proxy = ["localhost", "*.internal"]
}

# Mirror registry (for faster downloads)
mirror {
  maven_central = "https://maven.aliyun.com/repository/central"
}

# Telemetry (can be disabled)
telemetry = false
```

---

## 7. Non-Functional Requirements

### 7.1 Performance Targets

| Metric | Target | Comparison |
|--------|--------|------------|
| Single-file run startup | < 200 ms | javac + java: ~2 s |
| AOT-compiled startup | < 20 ms | JVM: ~2 s, GraalVM native: ~50 ms |
| Incremental build speed | < 1 s (single file change) | Maven: ~5 s |
| Full build (mid-size project) | < 30 s | Maven: ~60 s |
| Memory (AOT artifact) | < 50 MB (mid-size web app) | JVM: ~200 MB |
| Binary size | < 30 MB (mid-size web app) | GraalVM: ~60 MB |
| Dependency resolution | < 2 s (cached) | Maven: ~10 s |
| Rava binary size | < 50 MB | GraalVM: ~400 MB |

### 7.2 Reliability

| Metric | Target |
|--------|--------|
| Compilation correctness | Pass Java TCK core subset |
| Dependency resolution consistency | rava.lock guarantees 100% reproducible builds |
| Crash recovery | Interrupted builds do not corrupt cache |
| Error messages | Every error includes filename, line number, and fix suggestion |

### 7.3 Platform Support

| Platform | Support Level |
|----------|-------------|
| Linux x86_64 | Tier 1 (CI-tested) |
| Linux aarch64 | Tier 1 (CI-tested) |
| macOS x86_64 | Tier 1 (CI-tested) |
| macOS aarch64 (Apple Silicon) | Tier 1 (CI-tested) |
| Windows x86_64 | Tier 2 (community-tested) |

### 7.4 Installation

```bash
# macOS / Linux (recommended)
curl -fsSL https://rava.dev/install.sh | sh

# Homebrew
brew install a3s-lab/tap/rava

# Cargo (build from source)
cargo install rava

# Version management
rava upgrade              # upgrade to latest release
rava upgrade --canary     # upgrade to canary build
```

---

## 8. Comparison with Bun

| Dimension | Bun (TypeScript) | Rava (Java) |
|-----------|-----------------|-------------|
| Implementation language | Zig + C++ | Rust |
| Replaces | Node.js + npm + webpack | JDK + Maven/Gradle + GraalVM |
| Run | `bun run index.ts` | `rava run Main.java` |
| Build | `bun build` | `rava build` |
| Package management | `bun add express` | `rava add spring-boot-web` |
| Config | `package.json` | `rava.hcl` |
| Lockfile | `bun.lockb` | `rava.lock` |
| Test | `bun test` | `rava test` |
| REPL | `bun repl` | `rava repl` |
| Package registry | npm | Maven Central |
| Core advantage | Blazing-fast JS/TS runtime | Blazing-fast Java AOT compiler + runtime |

---

## 9. Glossary

| Term | Description |
|------|-------------|
| AOT | Ahead-of-Time compilation — compiling source to native machine code before execution |
| JIT | Just-in-Time compilation — compiling bytecode to machine code at runtime (traditional JVM approach) |
| RIR | Rava Intermediate Representation — Rava's internal IR |
| rava.hcl | Rava project config file, analogous to package.json / Cargo.toml |
| rava.lock | Dependency lockfile guaranteeing reproducible builds |
| Short name | Alias for a common Maven dependency (e.g. `guava` → `com.google.guava:guava`) |
| Reflection config | Reflection metadata for AOT compilation; Rava auto-detects with manual override available |
| TCK | Technology Compatibility Kit — Java compatibility test suite |
| LLVM | Compiler backend framework used for native machine code generation |
| Cranelift | Lightweight code-generation backend in the Rust ecosystem; faster compile times |
| MicroRT | Rava's embedded bytecode runtime — handles the 5% of code AOT cannot statically resolve |
| UnifiedHeap | Shared memory heap used by both AOT-compiled objects and MicroRT-interpreted objects |

---

# Part II — Technical Architecture

## 10. Approach: AOT + Embedded Bytecode Runtime

### 10.1 Executive Summary

**All dynamic Java features can be implemented — but not the GraalVM way.**

GraalVM's approach is "closed-world assumption + config fallback": every class must be known at compile time; anything unknown causes an error, and users must configure it manually. That is not solving the problem — it is avoiding it.

Rava's approach: **AOT compilation as the primary path + embedded lightweight bytecode runtime as the escape hatch**. Everything resolvable at compile time is AOT-compiled. Everything that cannot be resolved at compile time automatically falls back to the embedded runtime. Users never observe this transition.

```
GraalVM:  AOT compile → hit reflection → error → user writes reflect-config.json → recompile
Rava:     AOT compile → hit reflection → auto-mark → embedded runtime handles it → transparent to user
```

### 10.2 Why GraalVM's Approach Falls Short

GraalVM native-image is built on the [Closed-World Assumption](https://www.marcobehler.com/guides/graalvm-aot-jit):

> All classes, methods, and fields reachable by the program must be known at compile time. No class unseen at compile time may appear at runtime.

This assumption directly causes:

| Feature | GraalVM's handling | Problem |
|---------|-------------------|---------|
| Reflection | Requires [reflect-config.json](https://www.graalvm.org/22.1/reference-manual/native-image/Reflection/index.html) | Manual maintenance; framework upgrades can break it |
| Dynamic proxy | Requires [proxy-config.json](https://www.graalvm.org/latest/reference-manual/native-image/dynamic-features/DynamicProxy/) | Interface combinations must be declared upfront |
| Dynamic class loading | **Not supported** | `Class.forName()` can only find compile-time-known classes |
| Runtime bytecode generation | **Not supported** | [ByteBuddy/CGLIB require special adaptation](https://github.com/raphw/byte-buddy/issues/1588) |

**Why the config-file approach is a dead end:**

```
Typical reflection call chain in a Spring Boot project:

@RestController → Spring scan → reflection creates Bean
@Autowired      → reflection injects dependencies
@RequestBody    → Jackson reflection serialization/deserialization
@Transactional  → CGLIB dynamic proxy
JPA @Entity     → Hibernate reflection + dynamic proxy

A mid-size Spring Boot project's reflect-config.json can have 2,000+ entries.
Every dependency upgrade can invalidate the config.
This is not "limited support" — it is a maintenance nightmare.
```

**The Dart/Flutter lesson:**

[Dart removed dart:mirrors entirely](https://github.com/flutter/flutter/issues/1150) because reflection is incompatible with AOT + tree shaking. The Flutter ecosystem was forced to abandon dynamic capabilities and switch to compile-time code generation (`json_serializable`, `freezed`, etc.).

This is one "solution," but its cost is forcing the entire ecosystem to rewrite. Rava must not take this path — the core value of the Java ecosystem lies precisely in its dynamic capabilities. Removing reflection means removing Spring, Hibernate, and MyBatis.

---

## 11. First-Principles Analysis

### 11.1 What Do These Features Actually Do?

From the CPU's perspective, whether AOT or JIT, the end result is always machine code. The only difference is: **when is the machine code generated?**

| Feature | Essence | Required capability |
|---------|---------|-------------------|
| Reflection | Look up a class/method/field by string name at runtime and invoke it | Runtime metadata query + method dispatch |
| Dynamic proxy | Generate a new class at runtime that implements given interfaces and intercepts method calls | Runtime code generation + method interception |
| Dynamic class loading | Load and execute .class bytecode unknown at compile time | Runtime bytecode interpretation or compilation |

### 11.2 Three Levels of Complexity

```
Level 1 — Reflection (metadata query + call dispatch)
  → No runtime code generation needed
  → Only requires preserved metadata + function pointer table
  → Solvable with pure AOT

Level 2 — Dynamic proxy (generate a new class at runtime)
  → Requires runtime code generation
  → But generated code follows a fixed pattern: interface method → InvocationHandler.invoke()
  → Solvable with template AOT pre-generation + runtime assembly

Level 3 — Dynamic class loading (load arbitrary bytecode at runtime)
  → Requires runtime interpretation or compilation of arbitrary bytecode
  → Must embed a bytecode runtime
  → The hardest — and the one GraalVM abandoned entirely
```

---

## 12. Hybrid Runtime Architecture

### 12.1 Core Design: AOT + MicroRT

```
┌─────────────────────────────────────────────────────┐
│                  Rava Native Binary                   │
│                                                       │
│  ┌──────────────────────┐  ┌──────────────────────┐  │
│  │   AOT-compiled code   │  │ Embedded bytecode RT  │  │
│  │   (95%+ of code)      │  │   (Rava MicroRT)      │  │
│  │                       │  │                       │  │
│  │  • All statically     │  │  • Bytecode interpreter│  │
│  │    analyzable code    │  │  • Lightweight JIT     │  │
│  │  • Resolved reflection│  │    (optional)         │  │
│  │  • Pre-generated      │  │  • Class loader       │  │
│  │    proxy classes      │  │  • Reflection metadata │  │
│  │  • Direct machine     │  │    engine             │  │
│  │    code execution     │  │  • GC (shared)        │  │
│  └──────────┬────────────┘  └──────────┬────────────┘  │
│             │                          │               │
│             └─────────┬────────────────┘               │
│                       │                                │
│             ┌─────────▼──────────┐                     │
│             │   Unified Object    │                     │
│             │   Model             │                     │
│             │ (AOT objects and    │                     │
│             │  interpreter objects│                     │
│             │  share one heap)    │                     │
│             └─────────────────────┘                    │
└─────────────────────────────────────────────────────┘
```

**Key design decision: AOT code and interpreter code share a single object model and memory heap.**

This means:
- AOT-compiled methods can call objects running in the interpreter, and vice versa
- A `Method` found via reflection can point to either AOT code or bytecode in the interpreter
- Instances of dynamically loaded classes can be passed to AOT-compiled code

### 12.2 Rava MicroRT: Embedded Bytecode Runtime

MicroRT is not a full JVM. It is a lean runtime purpose-built as the escape hatch:

| Component | Description | Size estimate |
|-----------|-------------|--------------|
| Bytecode interpreter | Interprets Java bytecode (~200 instructions) | ~500 KB |
| Lightweight JIT | Compiles hot interpreter code (optional, uses Cranelift) | ~2 MB |
| Class loader | Loads bytecode from .class / .jar files | ~200 KB |
| Reflection metadata engine | Queries class/method/field metadata | ~100 KB |
| Bytecode verifier | Validates loaded bytecode for safety | ~150 KB |
| JNI environment layer | Provides `JNIEnv*` function table (~230 functions) and `JavaVM*` singleton for native library callbacks into Java | ~300 KB |
| **Total** | | **~3.3 MB** |

Final binary size: AOT code (~15 MB) + MicroRT (~3.3 MB) = **~18 MB** — still far smaller than a JVM (~200 MB).

### 12.3 Compile-Time Decision: AOT vs MicroRT

```
Rava compiler analysis pipeline:

1. Parse all source → AST → type check → semantic analysis

2. Reflection analysis pass:
   ├── Statically resolvable reflection calls → mark AOT (compile to function pointer calls)
   │   e.g. Class.forName("com.example.User")  ← string constant, resolvable at compile time
   │
   └── Dynamically unresolvable reflection calls → mark MicroRT
       e.g. Class.forName(config.get("className"))  ← only known at runtime

3. Proxy analysis pass:
   ├── Interface combination known at compile time → pre-generate proxy class, AOT compile
   └── Interface combination only known at runtime → MicroRT runtime generation

4. Class loading analysis pass:
   ├── Classes known at compile time → AOT
   └── Classes unknown at compile time (plugins, SPI) → MicroRT

5. Code generation:
   ├── AOT regions → LLVM/Cranelift → native machine code
   └── MicroRT regions → preserve bytecode + generate bridging code
```

---

## 13. Dynamic Features: Implementation

### 13.1 Reflection: Metadata Table + Dual-Path Dispatch

**Principle:** Reflection is fundamentally "find a function pointer by name and call it." After AOT compilation the function pointer is already known — we only need to retain a lookup table.

```
Compile-time metadata table (embedded in binary):

ClassMetadata {
  "com.example.User" → {
    fields: [
      { name: "id",   type: "long",   offset: 0,  getter: 0x7f001000 },
      { name: "name", type: "String", offset: 8,  getter: 0x7f001040 },
    ],
    methods: [
      { name: "getId",   signature: "()J",          ptr: 0x7f001000 },
      { name: "setName", signature: "(Ljava/lang/String;)V", ptr: 0x7f001080 },
    ],
    constructors: [
      { signature: "()V", ptr: 0x7f001100 },
    ]
  }
}
```

**Dual-path dispatch:**

```
Class.forName("com.example.User").getMethod("getId").invoke(obj)

Path A (AOT fast path):
  1. Query metadata table → find "com.example.User"
  2. Query method table  → find "getId" → function pointer 0x7f001000
  3. Call the function pointer directly (same speed as a normal method call)

Path B (MicroRT slow path):
  1. Query metadata table → not found (class unknown at compile time)
  2. Fall back to MicroRT → load .class file from classpath
  3. Parse bytecode → interpret or JIT compile
  4. Cache result → subsequent calls hit the cache
```

**Size impact:** The metadata table adds roughly 5–10% to binary size. Can be fully stripped with `rava build --strip-metadata` to trade reflection support for minimum size.

### 13.2 Dynamic Proxy: Template Pre-generation + Runtime Assembly

**Principle:** The code generated for a Java dynamic proxy follows a fixed pattern — every method is `handler.invoke(proxy, method, args)`. The only variation is the interface list and method signatures.

```java
// Java dynamic proxy in essence
Object proxy = Proxy.newProxyInstance(
    classLoader,
    new Class<?>[] { UserService.class, Cacheable.class },
    (proxy, method, args) -> {
        // interception logic
        return method.invoke(target, args);
    }
);

// The generated proxy class is essentially:
class $Proxy0 implements UserService, Cacheable {
    InvocationHandler handler;

    public User getUser(long id) {
        Method m = UserService.class.getMethod("getUser", long.class);
        return (User) handler.invoke(this, m, new Object[]{id});
    }
    // ... every interface method follows the same template
}
```

**Rava's three-layer strategy:**

```
Layer 1 — Compile-time pre-generation (covers 90%+ of cases)
  Compiler scans all Proxy.newProxyInstance() calls
  If the interface list is a compile-time constant → generate proxy class → AOT compile
  Spring @Transactional, MyBatis Mapper, etc. all fall here

Layer 2 — Template instantiation (covers ~9% of cases)
  Compiler generates a generic proxy template (AOT-compiled machine code)
  Runtime only needs to fill in: interface method table + InvocationHandler
  No new bytecode generation needed — only assembly of existing machine code fragments

Layer 3 — MicroRT fallback (covers ~1% of extreme cases)
  Runtime interface combination is completely unpredictable
  Fall back to MicroRT to generate bytecode → interpret
  Slow on first call; subsequent calls hit the cache
```

### 13.3 Dynamic Class Loading: Embedded Bytecode Runtime

**Principle:** Dynamic class loading is fundamentally "introducing code unknown at compile time." This is the root tension with AOT — but it is not an unsolvable one.

**Key insight: dynamically loaded classes do not need to be AOT-compiled. They can be interpreted.**

```
Scenario: SPI plugin loading

// Implementations are unknown at compile time
ServiceLoader<Plugin> plugins = ServiceLoader.load(Plugin.class);
for (Plugin p : plugins) {
    p.execute();  // calling code unknown at compile time
}

Rava's handling:

1. Plugin interface → AOT compile (known at compile time)
2. ServiceLoader.load() → scan META-INF/services/ at runtime
3. Discover com.third.MyPlugin → unknown at compile time
4. MicroRT loads MyPlugin.class → interprets bytecode
5. p.execute() → dispatched via interface; AOT code calls interpreter method
6. If MyPlugin.execute() becomes hot → MicroRT JIT-compiles it to machine code
```

**AOT ↔ MicroRT interop: the Unified Object Model**

```
┌─────────────────────────────────────────┐
│              Unified Object Header        │
│  ┌─────────┬──────────┬───────────────┐  │
│  │ Mark    │ Type ptr │ Origin tag    │  │
│  │ (GC)    │ (vtable) │ AOT/MicroRT   │  │
│  └─────────┴──────────┴───────────────┘  │
│                                          │
│  AOT object:                             │
│    Type ptr → AOT-compiled vtable        │
│               (array of function ptrs)   │
│                                          │
│  MicroRT object:                         │
│    Type ptr → Interpreter vtable         │
│               (bytecode method table)    │
│                                          │
│  Both object types allocated on the same │
│  heap and managed by the same GC         │
└─────────────────────────────────────────┘
```

When AOT code calls a method on a MicroRT object:
1. Read the type pointer from the object header
2. Detect it is a MicroRT vtable → jump to the interpreter entry point
3. Interpreter executes bytecode → returns result to AOT code

When MicroRT code calls a method on an AOT object:
1. Read the type pointer from the object header
2. Detect it is an AOT vtable → call the function pointer directly
3. Identical speed to a normal AOT call

---

## 14. JNI Subsystem

### 14.1 Two Directions, Two Levels of Complexity

JNI has two directions with very different complexity:

**Outbound (Java → C) — Phase 1**

Java declares `native` methods; C/C++ implements them:

```java
public class Database {
    static { System.loadLibrary("sqlite3"); }  // triggers dlopen
    public native long open(String path);      // AOT generates C call stub
    public native void exec(long db, String sql);
}
```

AOT compiler handling:
- `native` keyword → generate a C function call stub (a direct `call` instruction, no JNI overhead)
- `System.loadLibrary("sqlite3")` → runtime `dlopen` (Linux/macOS) or `LoadLibrary` (Windows)
- `System.load("/path/to/lib.so")` → load by explicit absolute path
- Static linking: merge `.a` archives at link time via `jni { static = ["sqlite3"] }` in `rava.hcl`

This is just ordinary C function calling — Phase 1 can implement it without MicroRT.

**Inbound (C → Java via JNIEnv\*) — Phase 3**

Native code calls back into Java via the JNI API:

```c
jclass    cls = (*env)->FindClass(env, "com/example/Callback");
jmethodID mid = (*env)->GetMethodID(env, cls, "onEvent", "(Ljava/lang/String;)V");
(*env)->CallVoidMethod(env, obj, mid, str);
```

This requires MicroRT to implement the full JNI function table.

### 14.2 MicroRT JNI Function Table (~230 Functions)

| Function group | MicroRT mapping |
|---------------|----------------|
| `FindClass`, `GetSuperclass` | ClassLoader (reflection metadata engine) |
| `GetMethodID`, `GetStaticMethodID` | Reflection metadata query |
| `Call*Method`, `CallStatic*Method` | Virtual dispatch (vtable/itable) |
| `NewObject`, `AllocObject` | UnifiedHeap allocation |
| `Get/SetField`, `Get/SetStaticField` | Field offset table |
| `NewStringUTF`, `GetStringUTFChars` | String interning |
| `New*Array`, `Get/Set*ArrayRegion` | Array operations |
| `NewGlobalRef`, `DeleteGlobalRef` | Reference counting |
| `ExceptionOccurred`, `ThrowNew` | Exception model |
| `AttachCurrentThread` | Thread registry |
| `GetPrimitiveArrayCritical` | Zero-copy array access |
| `JNI_OnLoad` / `JNI_OnUnload` | Library lifecycle hooks |

`JNIEnv*` is a per-thread pointer; `JavaVM*` is a global singleton. Both are provided by MicroRT.

### 14.3 JNI Type Mapping

| JNI type | MicroRT internal representation |
|----------|--------------------------------|
| `jobject` | `HeapRef` (UnifiedHeap reference) |
| `jstring` | `HeapRef` → Java `String` object |
| `jclass` | `ClassRef` (class metadata pointer) |
| `jarray` | `HeapRef` → Java array object |
| `jint`, `jlong`, `jdouble` | Rust native types (`i32`, `i64`, `f64`) |
| `jboolean` | `u8` (0 = false, 1 = true) |
| `jbyteArray` | `HeapRef` → `byte[]` (zero-copy eligible) |

### 14.4 Reference Management

JNI defines three reference types; MicroRT supports all three:

- **Local Ref**: valid within a single JNI call; auto-released when the function returns (Local Frame stack)
- **Global Ref**: created via `NewGlobalRef`; must be freed explicitly via `DeleteGlobalRef`
- **Weak Global Ref**: weak reference; GC may collect the referent; created via `NewWeakGlobalRef`

### 14.5 Library Lifecycle

```
System.loadLibrary("foo")
  → dlopen("libfoo.so")
  → look up symbol JNI_OnLoad
  → call JNI_OnLoad(JavaVM*, void*)   ← MicroRT provides JavaVM*
  → library init (register native methods, cache jclass/jmethodID)
  → return JNI version (e.g. JNI_VERSION_1_8)

dlclose("libfoo.so")  (on program exit)
  → call JNI_OnUnload(JavaVM*, void*)
  → library cleanup
```

### 14.6 GraalVM Comparison

| Capability | GraalVM native-image | Rava |
|-----------|---------------------|------|
| Outbound JNI (Java → C) | ✅ Supported (requires config) | ✅ Zero config, Phase 1 |
| `System.loadLibrary()` | ✅ Supported | ✅ Zero config |
| Inbound JNI (C → JNIEnv\*) | ⚠️ Limited, requires `@CEntryPoint` | ✅ Full function table, Phase 3 |
| `AttachCurrentThread` | ❌ Not supported | ✅ Thread registry, Phase 3 |
| `GetPrimitiveArrayCritical` | ✅ Supported | ✅ Zero-copy, Phase 3 |

---

## 15. Prior Art

The hybrid runtime approach is not new; it has well-established precedents:

### 15.1 GraalVM Truffle (Closest Precedent)

GraalVM's [Truffle framework](https://www.graalvm.org/jdk21/graalvm-as-a-platform/language-implementation-framework/HostOptimization/) is the production-grade incarnation of this idea:

- The Truffle interpreter itself is AOT-compiled into the native image
- The interpreter can dynamically interpret any guest-language code at runtime
- Hot guest code is JIT-compiled to machine code via Partial Evaluation
- Host (AOT) code and guest (interpreter) code share the same heap

Rava's MicroRT is essentially a Truffle-like embedded interpreter specialized for Java bytecode.

### 15.2 LuaJIT (Embedded Interpreter + JIT)

LuaJIT packages a Lua interpreter and JIT compiler into a ~500 KB single-file library. Any program that links against LuaJIT gains full Lua dynamic execution. Rava MicroRT does the same for Java bytecode.

### 15.3 Android ART (AOT + JIT Hybrid)

Android ART achieves:
- Frequently used code: AOT-compiled (at install time)
- Infrequently used or first-run code: JIT-executed
- Both share the same object model and GC

This proves that AOT + interpreter/JIT hybrid runtimes are production-viable.

### 15.4 .NET NativeAOT + Partial Interpreter

.NET NativeAOT faced the same reflection problem. .NET 9's solution:
- Statically resolvable reflection → AOT
- Unresolvable reflection → preserve metadata, handle at runtime via a built-in interpreter layer
- Unlike GraalVM, it does not error out

---

## 16. Trade-offs

This approach is not free. An honest accounting of costs:

### 16.1 Implementation Complexity

| Component | Complexity | Notes |
|-----------|-----------|-------|
| AOT compiler | High | Rava's core — required regardless |
| Metadata table generation | Medium | Additional compiler pass |
| MicroRT bytecode interpreter | High | ~100–200 K lines of Rust |
| AOT↔MicroRT interop layer | High | Unified object model is the hardest part |
| MicroRT JIT (optional) | Very high | Use Cranelift rather than building from scratch |
| Unified GC | High | GC must manage both object types simultaneously |

**This is a 2–3 year engineering project, not a 6-month deliverable.**

### 16.2 Performance Impact

| Scenario | Performance impact |
|----------|------------------|
| Pure AOT code (no reflection/proxy/dynamic loading) | Zero impact |
| Reflection call to an AOT-known class | Near-zero (metadata table lookup, faster than JVM reflection) |
| Reflection call to an unknown class | Slow on first call (MicroRT load); fast on cache hits thereafter |
| Dynamic proxy (compile-time known interfaces) | Zero impact (AOT pre-generated) |
| Dynamic proxy (runtime interfaces) | Small overhead on first call; normal thereafter |
| Dynamic class loading | Interpreted execution 2–5× slower than AOT; approaches AOT speed after JIT |

### 16.3 Binary Size

```
Final binary composition:
  AOT-compiled application code   ~15 MB
  AOT-compiled dependency code    ~10 MB
  MicroRT (interpreter)           ~3.3 MB
  Metadata table (for reflection) ~2 MB
  ──────────────────────────────────────
  Total                           ~30 MB   ← still far smaller than JVM 200 MB
```

### 16.4 Startup Time

```
Pure AOT code path:            ~10 ms   (no MicroRT initialization)
MicroRT present but not triggered: ~12 ms   (MicroRT init is lightweight)
Dynamic class loading triggered:   ~50 ms   (first bytecode load)
Traditional JVM:               ~2,000 ms
```

---

## 17. Phased Implementation Roadmap

### Phase 1 — Basic AOT (no MicroRT)

```
Goal:    make 80% of Java code AOT-compilable and runnable
Covers:  static code, Lambda, generics, stdlib, JNI Outbound
Excludes: reflection, dynamic proxy, dynamic class loading, JNI Inbound

Value delivered at this phase:
  - A friendlier toolchain than GraalVM native-image
  - HCL config, dependency management, rava run developer experience
  - Pure static code (algorithm libraries, CLI tools) runs perfectly
  - JNI Outbound: SQLite, OpenSSL, RocksDB, and any native library work out of the box
```

### Phase 2 — Reflection Support

```
Goal:    support reflection
Impl:    compile-time metadata table generation + dual-path dispatch
         (no MicroRT needed — reflection requires metadata, not an interpreter)

Frameworks unlocked: Jackson, Lombok (most cases)
Not yet: dynamic proxy, dynamic class loading, JNI Inbound
```

### Phase 3 — MicroRT v1 (Bytecode Interpreter)

```
Goal:    implement a Java bytecode interpreter
Impl:    Java bytecode interpreter (Rust implementation)
         Unified object model (AOT↔MicroRT interop)
         Full JNI function table (JNIEnv*, JavaVM*)

Unlocks: dynamic class loading, SPI, plugin systems, JNI Inbound
Frameworks: MyBatis, Hibernate (via interpreter), any JNI-heavy library
```

### Phase 4 — Dynamic Proxy AOT Promotion

```
Goal:    lift dynamic proxy from interpreter to AOT
Impl:    proxy template pre-generation + runtime assembly

Frameworks unlocked: Spring @Transactional (fully AOT), JDK Proxy (common combos AOT)
```

### Phase 5 — MicroRT v2 (Hot JIT)

```
Goal:    JIT-compile hot code paths inside MicroRT
Impl:    Cranelift as the JIT backend

Result:  dynamic class hot paths reach near-AOT performance
         Spring Boot + Hibernate runs fully, performance ~90% of JVM
```

---

## 18. Technical Challenges

An honest list of the three hardest problems:

### 18.1 Unified Object Model (Hardest)

AOT-compiled objects and MicroRT-interpreted objects must share the same memory representation and GC. This means Rava needs a custom GC capable of managing objects from both sources. Reference: Android ART. Estimated effort: 6–12 months.

### 18.2 Java Standard Library Coverage in the Bytecode Interpreter

The Java standard library contains thousands of classes. MicroRT cannot re-implement all of them. Solution: the vast majority of the standard library is already AOT-compiled into the binary; MicroRT only needs to be able to call those AOT-compiled stdlib methods (reverse interop). This is feasible but requires careful design.

### 18.3 Security

Dynamic class loading means arbitrary code can be loaded at runtime. A bytecode verifier is required to prevent malicious code. This is a mandatory component of MicroRT.

---

## 19. Feature Feasibility Summary

| Feature | Feasible? | Approach | Cost |
|---------|----------|----------|------|
| Reflection | ✅ Fully | AOT metadata table + dual-path dispatch | Phase 2 |
| Dynamic proxy | ✅ Fully | AOT pre-generation + MicroRT fallback | Phase 3–4 |
| Dynamic class loading | ✅ Fully | Embedded MicroRT bytecode runtime | Phase 3, most complex |
| JNI Outbound (Java → C) | ✅ Fully | AOT native method stubs + dlopen | Phase 1, low complexity |
| JNI Inbound (C → JNIEnv\*) | ✅ Fully | MicroRT JNI function table (~230 functions) | Phase 3, parallel with dynamic class loading |

**Rava's differentiation:** where GraalVM says "I can't do it — you configure it," Rava says "I handle it — you don't need to know." The cost is high engineering complexity and a long development timeline, but that is precisely Rava's core technical moat.

The Rava that achieves this will mean:
- Any Spring Boot / Hibernate / MyBatis / SQLite JNI project compiles to a native binary with zero code changes
- 10 ms startup, 20 MB memory, single-file deployment
- Something GraalVM cannot do — and the most urgent need in the entire Java ecosystem

---

*References:*
- [GraalVM Reachability Metadata](https://docs.oracle.com/en/graalvm/jdk/21/docs/reference-manual/native-image/metadata/)
- [GraalVM Dynamic Proxy](https://www.graalvm.org/latest/reference-manual/native-image/dynamic-features/DynamicProxy/)
- [GraalVM Truffle Host Optimization](https://www.graalvm.org/jdk21/graalvm-as-a-platform/language-implementation-framework/HostOptimization/)
- [ByteBuddy GraalVM Issue #1588](https://github.com/raphw/byte-buddy/issues/1588)
- [Flutter dart:mirrors Issue #1150](https://github.com/flutter/flutter/issues/1150)
- [OpenJDK JEP 8335368 — Ahead-of-Time Code Compilation](https://openjdk.org/jeps/8335368)
- [Java 25 AOT Cache Deep Dive](https://andrewbaker.ninja/2025/12/23/java-25-aot-cache-a-deep-dive-into-ahead-of-time-compilation-and-training/)

---

# Part III — Implementation Reference

This section documents the internal Rust implementation design for each component. It maps directly to the Rust crate structure and serves as the canonical design reference for contributors.

## Design Philosophy

**First-principles: minimal core + external extensions.**

Every component answers three questions:

1. Is this core or extension? Core components are capped at 7, non-replaceable.
2. Does every extension point have a trait definition and a working default implementation?
3. If this component is deleted, can the system still function? If yes — consider making it an extension.

Rava's core mission: **compile Java source to native binaries and bring the Java developer experience to the level of Bun/Deno.**

Any design that does not directly serve this mission does not belong in the core.

---

## 20. System Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                          rava CLI                                   │
│  run | build | init | add | test | fmt | lint | repl | publish     │
└────────────────────────┬───────────────────────────────────────────┘
                         │
          ┌──────────────▼──────────────┐
          │       Compiler Frontend      │
          │  Parser → AST → TypeChecker  │
          │  → SemanticAnalyzer          │
          └──────────────┬──────────────┘
                         │ RIR (Rava IR)
          ┌──────────────▼──────────────┐
          │         Analysis Passes      │
          │  ReflectionPass             │
          │  ProxyPass                  │
          │  ClassLoadPass              │
          └──────┬───────────┬──────────┘
                 │           │
    ┌────────────▼──┐   ┌────▼──────────────┐
    │  AOT Backend   │   │  MicroRT Marker   │
    │  Optimizer    │   │  (marks regions   │
    │  Cranelift/   │   │  needing the      │
    │  LLVM         │   │  embedded RT)     │
    └────────┬──────┘   └───────────────────┘
             │
┌────────────▼────────────────────────────────┐
│             Native Binary                    │
│  ┌────────────────┐  ┌─────────────────────┐│
│  │  AOT-compiled  │  │   Rava MicroRT       ││
│  │  code (~95%)   │  │  (~3 MB embedded RT) ││
│  │                │  │                      ││
│  │  • Static code │  │  • Bytecode interp   ││
│  │  • Resolved    │  │  • Cranelift JIT     ││
│  │    reflection  │  │  • Class loader      ││
│  │  • Pre-gen     │  │  • Reflection engine ││
│  │    proxy class │  │  • Bytecode verifier ││
│  │  • Metadata    │  │                      ││
│  └───────┬────────┘  └──────────┬───────────┘│
│          └──────────┬───────────┘             │
│               ┌─────▼──────┐                  │
│               │   Unified   │                  │
│               │ Object Model│                  │
│               │ Shared heap │                  │
│               │    + GC     │                  │
│               └────────────┘                  │
└─────────────────────────────────────────────┘

Runs separately (not embedded in binary):
┌─────────────────────────────────────────────┐
│           Package Manager                    │
│  Resolver → Maven Central → rava.lock        │
└─────────────────────────────────────────────┘
```

---

## 21. Core Components and Extension Points

### 21.1 Core Components (7, non-replaceable)

| # | Component | Responsibility | Why core |
|---|-----------|---------------|----------|
| 1 | **Frontend** | Java source → RIR | No compiler without it |
| 2 | **RIR** | Intermediate representation — shared language of the compiler | Decouples frontend from all backends |
| 3 | **AOT Backend** | RIR → native machine code | Core value: native binary |
| 4 | **MicroRT** | Escape hatch for reflection/proxy/dynamic loading | The fundamental difference from GraalVM |
| 5 | **UnifiedHeap** | AOT and MicroRT objects share one heap | Foundation for interoperability |
| 6 | **PackageManager** | Dependency resolution + rava.lock | Core to toolchain UX |
| 7 | **CLI** | User entry point, assembles all capabilities | Facade for zero-config experience |

### 21.2 Extension Points (trait-defined, all have default implementations)

| Trait | Default Implementation | Purpose |
|-------|----------------------|---------|
| `CodegenBackend` | `CraneliftBackend` | AOT code generation; replaceable with LLVM |
| `AotOptimizer` | Standard optimization pass chain | Escape analysis, inlining, DCE, constant folding |
| `BytecodeDispatcher` | `MatchDispatcher` | MicroRT bytecode dispatch strategy |
| `JitCompiler` | `CraneliftJit` | MicroRT hot-path JIT |
| `GcStrategy` | `G1StyleGc` | Garbage collection algorithm |
| `ClassResolver` | `MavenCentralResolver` | Maven dependency resolution |
| `LockfileFormat` | `HclLockfile` | rava.lock serialization format |
| `DiagnosticEmitter` | `TerminalEmitter` | Compilation error output |
| `JavaFormatter` | `GoogleJavaFormat` | `rava fmt` code formatter |
| `LintRules` | Built-in rule set | `rava lint` check rules |

---

## 22. Crate Structure

One crate, one responsibility:

```
rava/
├── cli/                 # CLI entry point, assembles all crates (bin crate)
├── frontend/            # Java lexing, parsing, type checking, semantic analysis
├── rir/                 # RIR data structure definitions (pure data, no logic)
├── aot/                 # AOT optimization passes + code generation coordination
├── codegen-cranelift/   # Cranelift backend (implements CodegenBackend trait)
├── codegen-llvm/        # LLVM backend (optional, feature flag)
├── micrort/             # MicroRT core: interpreter + class loading + verification + metadata
├── heap/                # Unified heap + GC (UnifiedHeap crate)
├── pkg/                 # Package manager: dependency resolution, Maven Central, rava.lock
├── hcl/                 # HCL parsing/generation for rava.hcl and rava.lock
└── common/              # Shared types: errors, diagnostics, Java type system
```

**Dependency rules (one-way, no cycles):**

```
cli
  → frontend, aot, micrort, pkg, hcl

frontend
  → rir, common

aot
  → rir, codegen-cranelift (default feature)

micrort
  → heap, common

heap
  → common (only)

pkg
  → hcl, common

all other crates → common
common → no internal dependencies
```

---

## 23. Frontend Pipeline (`rava-frontend`)

### 23.1 Pipeline Stages

```
Java source file
  │
  ▼ Lexer
  Token stream (keywords, identifiers, literals, operators)
  │
  ▼ Parser
  Concrete Syntax Tree (CST) → Abstract Syntax Tree (AST)
  AST preserves original span info (filename + line/col) for error reporting
  │
  ▼ TypeChecker
  Type-annotated AST
  Resolve names (imports, package paths)
  Infer `var` types
  Check type compatibility
  │
  ▼ SemanticAnalyzer
  Control flow analysis (dead code, reachability)
  Definite assignment checking
  Access modifier checking (public/private/protected)
  │
  ▼ Analysis Passes (MicroRT marking passes)
  ReflectionPass:  identify reflection calls; mark AOT-resolvable vs MicroRT
  ProxyPass:       identify Proxy.newProxyInstance; mark AOT-pre-generable vs MicroRT
  ClassLoadPass:   identify Class.forName and ServiceLoader; mark AOT vs MicroRT
  │
  ▼ RIR generation
  Rava Intermediate Representation (see §24)
```

### 23.2 Error Reporting Specification

Every diagnostic must include:

```rust
pub struct Diagnostic {
    pub level:   DiagnosticLevel,  // Error | Warning | Note
    pub code:    &'static str,     // e.g. "E0042"
    pub message: String,           // human-readable description of the problem
    pub span:    Span,             // filename + start/end position
    pub label:   Option<String>,   // annotation displayed at the span
    pub help:    Option<String>,   // fix suggestion (how to resolve)
    pub notes:   Vec<String>,      // additional context
}
```

Example output (Rust compiler style):

```
error[E0042]: cannot resolve reflection target at compile time
  --> src/com/example/Service.java:42:18
   |
42 |     Class.forName(config.get("class"))
   |                   ^^^^^^^^^^^^^^^^^^^ dynamic string, cannot resolve
   |
   = note: this call will be handled by Rava MicroRT at runtime
   = help: if performance is critical, use a compile-time constant string
```

---

## 24. Rava Intermediate Representation (RIR, `rava-rir`)

RIR is the shared language connecting the frontend to all backends. Design principle: **sufficiently lowered** — does not preserve Java's high-level abstractions, but remains semantically isomorphic to Java.

### 24.1 RIR Type System

```rust
pub enum RirType {
    // Primitive types
    I8, I16, I32, I64,
    F32, F64,
    Bool,
    Void,
    // Reference types
    Ref(ClassId),          // pointer to a heap object
    Array(Box<RirType>),   // array
    // Special types
    RawPtr,                // for MicroRT interop layer
}
```

### 24.2 RIR Instruction Set

```rust
pub enum RirInstr {
    // Control flow
    Branch { cond: Value, then_bb: BlockId, else_bb: BlockId },
    Jump(BlockId),
    Return(Option<Value>),
    Unreachable,

    // Calls
    Call          { func: FuncId,     args: Vec<Value>, ret: Option<Value> },
    CallVirtual   { receiver: Value, method: MethodId, args: Vec<Value>, ret: Option<Value> },
    CallInterface { receiver: Value, method: MethodId, args: Vec<Value>, ret: Option<Value> },

    // Object operations
    New        { class: ClassId,                            ret: Value  },
    GetField   { obj: Value, field: FieldId,                ret: Value  },
    SetField   { obj: Value, field: FieldId,  val: Value               },
    GetStatic  { field: FieldId,                            ret: Value  },
    SetStatic  { field: FieldId,              val: Value               },
    Instanceof { obj: Value, class: ClassId,                ret: Value  },
    Checkcast  { obj: Value, class: ClassId                            }, // throws ClassCastException

    // Array operations
    NewArray   { elem_type: RirType, len: Value,            ret: Value  },
    ArrayLoad  { arr: Value, idx: Value,                    ret: Value  },
    ArrayStore { arr: Value, idx: Value, val: Value                    },
    ArrayLen   { arr: Value,                                ret: Value  },

    // Arithmetic / bitwise / comparison (type determined by operands)
    BinOp   { op: BinOp,   lhs: Value, rhs: Value, ret: Value },
    UnaryOp { op: UnaryOp, operand: Value,          ret: Value },

    // Type conversion
    Convert { val: Value, from: RirType, to: RirType, ret: Value },

    // Exceptions
    Throw(Value),
    // Exception handling expressed via basic block landing_pad attribute

    // MicroRT interop (inserted by Analysis Passes)
    MicroRtReflect   { class_name: Value,                      ret: Value }, // dynamic Class.forName path
    MicroRtProxy     { interfaces: Vec<Value>, handler: Value, ret: Value },
    MicroRtClassLoad { class_name: Value,                      ret: Value },

    // Synchronization
    MonitorEnter(Value),
    MonitorExit(Value),
}
```

### 24.3 RIR Function Structure (SSA Form)

```rust
pub struct RirFunction {
    pub id:           FuncId,
    pub name:         String,
    pub params:       Vec<(Value, RirType)>,
    pub return_type:  RirType,
    pub basic_blocks: Vec<BasicBlock>,  // first BB is the entry block
    pub flags:        FuncFlags,        // is_clinit, is_constructor, is_synchronized
}

pub struct BasicBlock {
    pub id:          BlockId,
    pub params:      Vec<(Value, RirType)>,  // phi functions as BB params (MLIR style)
    pub instrs:      Vec<RirInstr>,
    pub terminator:  Terminator,
    pub landing_pad: Option<LandingPad>,     // exception handler
}
```

---

## 25. AOT Backend (`rava-aot` + `rava-codegen-cranelift`)

### 25.1 Optimization Pass Chain

```
RIR (from frontend)
  │
  ▼ EscapeAnalysisPass
    Analyze object escape — stack allocation vs heap allocation
    Objects that escape to other threads → forced heap allocation
  │
  ▼ InliningPass
    Inline small methods (< 32 bytecodes)
    Hot methods (profile-guided; heuristic-based on first compile)
  │
  ▼ DeadCodeEliminationPass
    Remove unreachable basic blocks
    Remove dead variables with no side effects
  │
  ▼ ConstantFoldingPass
    Evaluate constant expressions at compile time
  │
  ▼ MetadataTableGenPass
    Generate reflection metadata table (ClassMetadata) for all classes
    Generate fast paths for AOT-resolvable reflection calls
  │
  ▼ ProxyPregenPass
    Pre-generate proxy classes for compile-time-determinable interface combinations
  │
  ▼ MicroRtBridgePass
    Generate bridging code for MicroRtReflect/MicroRtProxy/MicroRtClassLoad instructions
  │
  ▼ CodegenBackend (trait)
    Default: CraneliftBackend
    Optional: LlvmBackend (feature flag — slower compile, better codegen)
  │
  ▼ Linker
    Statically link: stdlib + dependencies + MicroRT + metadata table
    Output: native binary
```

### 25.2 `CodegenBackend` Trait

```rust
pub trait CodegenBackend: Send {
    /// Compile a single RIR function to machine code.
    fn compile_function(&mut self, func: &RirFunction) -> Result<CompiledFunc>;

    /// Batch compile (allows cross-function optimization such as link-time inlining).
    fn compile_module(&mut self, module: &RirModule) -> Result<CompiledModule>;

    /// Return the list of target platforms this backend supports.
    fn supported_targets(&self) -> &[TargetTriple];
}

pub struct CompiledFunc {
    pub code:        Vec<u8>,           // machine code bytes
    pub relocations: Vec<Reloc>,        // symbol references needing linking
    pub gc_maps:     Vec<GcMap>,        // GC root info at each safepoint
    pub unwind_info: UnwindInfo,        // frame info for exception unwinding
}
```

### 25.3 Reflection Metadata Table (generated at compile time, embedded in binary)

```rust
/// Reflection metadata table embedded in the binary's read-only data segment.
pub struct ClassMetadata {
    pub name:         &'static str,
    pub fields:       &'static [FieldMetadata],
    pub methods:      &'static [MethodMetadata],
    pub constructors: &'static [ConstructorMetadata],
    pub superclass:   Option<ClassId>,
    pub interfaces:   &'static [ClassId],
}

pub struct MethodMetadata {
    pub name:       &'static str,
    pub descriptor: &'static str,       // JVM method descriptor, e.g. "(ILjava/lang/String;)V"
    pub ptr:        fn(),               // function pointer to AOT-compiled code
    pub flags:      MethodFlags,        // public/static/final/synchronized
}

pub struct FieldMetadata {
    pub name:      &'static str,
    pub type_name: &'static str,
    pub offset:    usize,               // byte offset within object
    pub getter:    fn(*const ()) -> u64, // generic getter (result reinterpret_cast)
    pub setter:    fn(*mut (), u64),     // generic setter
}
```

---

## 26. MicroRT (`rava-micrort`)

MicroRT is Rava's key differentiating component and the most complex part. It is not a full JVM — it is a **lean escape-hatch runtime** that handles only the code AOT cannot statically analyze.

### 26.1 Internal Component Map

```
MicroRT (~3 MB)
├── BytecodeInterpreter (~500 KB)
│   └── Core execution engine, Rust match dispatch
├── CraneliftJit (~2 MB, optional feature)
│   └── Hot bytecode → native machine code
├── ClassLoader (~200 KB)
│   └── Bootstrap → Platform → Application three-tier delegation
├── BytecodeVerifier (~150 KB)
│   └── StackMapTable verification, type safety checks
└── ReflectionEngine (~100 KB)
    └── Runtime metadata queries, augmenting the AOT metadata table
```

### 26.2 Bytecode Interpreter

**Dispatch strategy: Rust `match` (main loop)**

On modern CPUs (Haswell and later), Rust `match` is within 5–20% of C computed-goto performance. Given that:
- MicroRT handles only a small fraction of execution (< 5% of code paths) — absolute interpreter speed is not the primary goal
- `match` is safe Rust — no platform-specific unsafe inline assembly to maintain
- Hot code will be JIT-compiled by Cranelift, further reducing the interpreter's performance impact

**Key optimizations (in safe Rust):**

1. **Pointer-based PC**: `pc` is a raw pointer into the bytecode array; reading the next byte is a single dereference + pointer increment
2. **Unchecked array access**: The local variable array size is validated at method entry; the dispatch loop uses `get_unchecked` (one check + unsafe)
3. **Inline caches (IC)**: `invokevirtual` / `invokeinterface` maintain a 2–4 entry cache per call site

```rust
/// Core structures of the MicroRT main execution loop
pub struct Interpreter {
    /// Call frame stack (Vec rather than recursion — avoids C stack overflow)
    frames:       Vec<Frame>,
    /// Reference to the unified heap
    heap:         Arc<UnifiedHeap>,
    /// Class loader
    class_loader: ClassLoader,
}

pub struct Frame {
    /// Pointer into the bytecode array (pointer-based PC optimization)
    pc:              *const u8,
    /// Local variable array (4-byte slots; long/double occupy 2 slots)
    locals:          Vec<u32>,
    /// Operand stack (LIFO)
    operand_stack:   Vec<StackValue>,
    /// Current method
    method:          Arc<Method>,
    /// Exception handler table reference
    exception_table: &'static [ExceptionEntry],
    /// Inline caches (one entry per invoke call site)
    inline_caches:   Vec<InlineCache>,
}

#[derive(Clone, Copy)]
pub enum StackValue {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Ref(HeapRef),   // reference into the unified heap
    RetAddr(usize), // for jsr/ret (legacy; rare in modern bytecode)
}

pub struct InlineCache {
    pub state:   IcState,
    pub entries: [IcEntry; 4],  // up to 4 polymorphic entries
}

pub enum IcState { Uninitialized, Monomorphic, Polymorphic, Megamorphic }

pub struct IcEntry {
    pub klass:      KlassId, // expected object type
    pub method_ptr: fn(),    // direct call on cache hit
}
```

**Instruction dispatch loop (abbreviated):**

```rust
fn run(&mut self, frame: &mut Frame) -> Result<StackValue, Exception> {
    loop {
        // SAFETY: bytecode verifier guarantees pc is within valid range
        let opcode = unsafe { *frame.pc };
        frame.pc = unsafe { frame.pc.add(1) };

        match opcode {
            // Constant loads
            0x02 => frame.push(StackValue::Int(-1)),  // iconst_m1
            0x03 => frame.push(StackValue::Int(0)),   // iconst_0
            // ... remaining iconst_* omitted

            // Local variable load
            0x15 => {  // iload
                let idx = unsafe { *frame.pc } as usize;
                frame.pc = unsafe { frame.pc.add(1) };
                let val = unsafe { *frame.locals.get_unchecked(idx) };
                frame.push(StackValue::Int(val as i32));
            }

            // Arithmetic
            0x60 => {  // iadd
                let b = frame.pop_int();
                let a = frame.pop_int();
                frame.push(StackValue::Int(a.wrapping_add(b)));
            }

            // Virtual method call (with inline cache)
            0xb6 => {  // invokevirtual
                let method_ref = frame.read_u16();
                let receiver   = frame.peek_receiver();
                let klass      = self.heap.klass_of(receiver);
                self.invoke_virtual_with_ic(frame, klass, method_ref)?;
            }

            // Throw exception
            0xbf => {  // athrow
                let exc = frame.pop_ref();
                return Err(self.build_exception(exc));
            }

            // Unrecognized opcode → internal error
            unknown => {
                return Err(Exception::internal(
                    format!("unknown opcode: {:#04x}", unknown)
                ));
            }
        }
    }
}
```

### 26.3 Class Loader

Implements the JVM specification's three-tier delegation model:

```rust
pub struct ClassLoader {
    bootstrap: BootstrapLoader,    // loads stdlib (already AOT-compiled into the binary)
    platform:  PlatformLoader,     // loads JDK extension modules
    app:       ApplicationLoader,  // loads user classes and dependencies
    cache:     HashMap<String, Arc<Class>>,  // cache of loaded classes
}

impl ClassLoader {
    /// Load a class, following the delegation model (parent first, then self).
    pub fn load(&mut self, name: &str) -> Result<Arc<Class>> {
        // 1. Check cache
        if let Some(cls) = self.cache.get(name) {
            return Ok(Arc::clone(cls));
        }
        // 2. Delegation chain: bootstrap → platform → app
        let cls = if self.bootstrap.can_load(name) {
            // stdlib classes are in the AOT metadata table — create a lightweight wrapper
            self.bootstrap.load_from_aot_metadata(name)?
        } else if self.platform.can_load(name) {
            self.platform.load(name)?
        } else {
            // Load bytecode from classpath (.class / .jar)
            let bytecode = self.app.find_bytecode(name)?;
            self.define_class(name, &bytecode)?
        };
        self.cache.insert(name.to_string(), Arc::clone(&cls));
        Ok(cls)
    }

    /// Parse bytecode into a Class structure.
    fn define_class(&self, name: &str, bytecode: &[u8]) -> Result<Arc<Class>> {
        // 1. Parse the class file (magic, version, constant pool, methods, ...)
        let class_file = parse_class_file(bytecode)?;
        // 2. Bytecode verification (StackMapTable check)
        verify_bytecode(&class_file)?;
        // 3. Create the runtime Class object (allocated on the unified heap)
        let cls = Class::from_class_file(class_file, self)?;
        Ok(Arc::new(cls))
    }
}
```

### 26.4 Bytecode Verifier

StackMapTable verifier — prevents type-unsafe bytecode:

```rust
pub fn verify_bytecode(class_file: &ClassFile) -> Result<()> {
    for method in &class_file.methods {
        if let Some(code) = &method.code {
            // 1. Parse the StackMapTable attribute (required for Java 6+)
            let stack_map = parse_stack_map_table(code)?;
            // 2. Abstract interpretation: verify type consistency at each frame
            abstract_interpret(code, &stack_map)?;
        }
    }
    Ok(())
}

/// Abstract interpreter: simulate execution, tracking the types of stack and locals.
fn abstract_interpret(code: &CodeAttribute, stack_map: &StackMapTable) -> Result<()> {
    let mut state = VerificationState::new(code.max_stack, code.max_locals);
    for frame_entry in &stack_map.frames {
        // Verify that types described in the stack map match bytecode inference
        state.check_frame(frame_entry)?;
    }
    // Check that exception handler table ranges are valid
    for entry in &code.exception_table {
        check_exception_entry(entry, code.code.len())?;
    }
    Ok(())
}
```

### 26.5 Cranelift JIT (Hot-Path Promotion)

MicroRT has a built-in hotness counter. When a method has been interpreted past a threshold, it triggers Cranelift JIT compilation:

```rust
pub struct JitCompiler {
    /// Cranelift module managing all JIT-compiled functions
    module:    cranelift_jit::JITModule,
    /// Hotness counters (method → execution count)
    hotness:   HashMap<MethodId, u32>,
    /// JIT compilation threshold (number of interpreted invocations before JIT)
    threshold: u32,  // default: 1000
    /// Cache of JIT-compiled methods
    compiled:  HashMap<MethodId, JitCode>,
}

impl JitCompiler {
    /// Record a method call; trigger JIT if threshold is exceeded.
    pub fn record_call(&mut self, method: MethodId) -> Option<JitCode> {
        let count = self.hotness.entry(method).or_insert(0);
        *count += 1;
        if *count >= self.threshold {
            if let Some(code) = self.compiled.get(&method) {
                return Some(code.clone());
            }
            if let Ok(code) = self.jit_compile(method) {
                self.compiled.insert(method, code.clone());
                return Some(code);
            }
        }
        None
    }

    /// Compile Java bytecode to Cranelift CLIF IR, then to machine code.
    fn jit_compile(&mut self, method: MethodId) -> Result<JitCode> {
        let mut ctx      = cranelift::codegen::Context::new();
        let mut func_ctx = FunctionBuilderContext::new();
        let mut builder  = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

        // Translate Java bytecode to CLIF IR
        let translator = BytecodeToClif::new(&mut builder);
        translator.translate_method(method)?;
        builder.finalize();

        // Cranelift compiles to machine code (microsecond latency)
        let id = self.module.declare_function(...)?;
        self.module.define_function(id, &mut ctx)?;
        self.module.finalize_definitions()?;

        let ptr = self.module.get_finalized_function(id);
        Ok(JitCode { ptr, size: ctx.compiled_code().unwrap().code_info().total_size })
    }
}
```

---

## 27. Unified Object Model (`rava-heap`)

The hardest technical challenge in Rava: AOT-compiled objects and MicroRT-interpreted objects must share the same heap, use a unified object header format, and be managed by the same GC.

### 27.1 Object Header Layout

```
┌──────────────────────────────────────────────────────┐
│             Unified Object Header (16 bytes)          │
│                                                      │
│  ┌────────────────────────────────────────────────┐  │
│  │              Mark Word (8 bytes)               │  │
│  │  [63:62] lock state (00=unlocked 01=biased     │  │
│  │                       10=lightweight 11=heavy) │  │
│  │  [61]    GC mark bit (tri-color: white/grey/   │  │
│  │                        black)                  │  │
│  │  [60]    forwarding pointer flag (used when GC │  │
│  │                                  moves objects)│  │
│  │  [59]    origin tag: 0=AOT object 1=MicroRT    │  │
│  │  [58:32] identity hashcode (27 bits)            │  │
│  │  [31:0]  GC generation + other GC flags        │  │
│  └────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────┐  │
│  │              Klass Pointer (8 bytes)           │  │
│  │  Points to KlassDescriptor (type descriptor)   │  │
│  │  AOT object:     points to AOT KlassDesc       │  │
│  │  MicroRT object: points to MicroRT Class struct│  │
│  │  Both implement the same KlassDescriptor trait │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘

Field layout (immediately after the header):
┌─────────────────┐
│  Object header  │  16 bytes
├─────────────────┤
│  long / double  │  8-byte aligned
├─────────────────┤
│  int / float    │  4-byte aligned
├─────────────────┤
│  short / char   │  2-byte aligned
├─────────────────┤
│  byte / bool    │  1 byte
├─────────────────┤
│  ref fields     │  8 bytes (pointer)
├─────────────────┤
│  padding        │  align to 8-byte boundary
└─────────────────┘

Arrays have an additional 4-byte length field after the object header:
┌─────────────────┐
│  Object header  │  16 bytes
├─────────────────┤
│  length (4B)    │
├─────────────────┤
│  padding (4B)   │  alignment
├─────────────────┤
│  elem[0]        │
│  elem[1]        │
│  ...            │
└─────────────────┘
```

**The origin tag bit (bit 59 of the Mark Word) is the key to interoperability:**

```rust
impl HeapRef {
    /// When AOT code calls a method on an object, dispatch via the klass pointer.
    pub fn dispatch_virtual(&self, method_slot: usize) -> fn() {
        let klass = self.klass();
        // KlassDescriptor trait is implemented by both AotKlass and MicroRtKlass.
        // vtable() returns the method pointer array for this type.
        klass.vtable()[method_slot]
    }
}

pub trait KlassDescriptor: Send + Sync {
    /// Return the virtual method table.
    /// AOT objects:     array of direct function pointers
    /// MicroRT objects: array of interpreter stub functions
    fn vtable(&self) -> &[fn()];
    /// Instance size in bytes
    fn instance_size(&self) -> usize;
    /// Byte offsets within the object that contain references (used by GC)
    fn ref_offsets(&self) -> &[usize];
}
```

When AOT code calls a method on a MicroRT object:
1. Read the klass pointer from the object header
2. The klass pointer points to `MicroRtKlass`; its vtable holds **interpreter stub functions**
3. The stub takes control, entering the MicroRT interpreter to execute bytecode
4. After execution, the result is returned to the AOT code

When MicroRT code calls a method on an AOT object:
1. Read the klass pointer from the object header
2. The klass pointer points to `AotKlass`; its vtable holds **direct function pointers**
3. The MicroRT interpreter directly `call`s the function pointer
4. Speed is identical to a normal AOT-to-AOT call

### 27.2 GC Design: Concurrent Tri-color Mark + Generational Collection

```rust
pub struct UnifiedHeap {
    /// Young generation (new objects allocated here)
    young:      YoungGeneration,
    /// Old generation (long-lived objects promoted here)
    old:        OldGeneration,
    /// Card table (used by write barriers; 1 byte per 512 bytes of heap)
    card_table: CardTable,
    /// Safepoint coordinator
    safepoint:  SafepointCoordinator,
}

/// Card table write barrier (called after every write to a reference field)
#[inline(always)]
pub fn write_barrier(obj_addr: usize, _new_ref: HeapRef) {
    // Mark the card containing this object as dirty
    let card_idx = obj_addr >> 9;  // one card per 512 bytes
    // SAFETY: card_table is a fixed-size byte array; idx is bounded by heap limits
    unsafe { CARD_TABLE[card_idx] = DIRTY_CARD; }
}

impl UnifiedHeap {
    pub fn allocate(&mut self, size: usize, klass: KlassId) -> HeapRef {
        // Try fast-path allocation in TLAB (Thread-Local Allocation Buffer)
        if let Some(r) = self.current_thread_tlab().try_alloc(size) {
            return r;
        }
        // TLAB full → trigger Minor GC or request a new TLAB
        self.handle_tlab_overflow(size, klass)
    }

    /// Stop-the-world safepoint GC (Major GC)
    fn major_gc(&mut self) {
        // 1. Bring all threads to a safepoint
        self.safepoint.request_safepoint();
        self.safepoint.wait_all_threads();

        // 2. Scan all GC roots
        let mut roots = vec![];
        self.scan_thread_stacks(&mut roots);  // thread stacks (via GC stack maps)
        self.scan_static_fields(&mut roots);  // static fields
        self.scan_jni_handles(&mut roots);    // JNI global references

        // 3. Tri-color marking (concurrent or STW simplified)
        self.mark_from_roots(&roots);

        // 4. Sweep unmarked objects
        self.sweep();

        // 5. Resume all threads
        self.safepoint.release();
    }
}
```

**GC Stack Maps:**

The AOT compiler generates a stack map at every safepoint (after method calls, at loop back-edges), recording which stack slots and registers contain object references at that point:

```rust
pub struct GcMap {
    /// Instruction offset from function start
    pub pc_offset:        u32,
    /// Reference bitmap for stack slots (1 bit = 1 slot; 1 = contains a reference)
    pub stack_ref_bitmap: u64,
    /// Reference bitmap for registers
    pub reg_ref_bitmap:   u32,
}
```

The MicroRT interpreter's GC root scan is simpler: every `StackValue::Ref` in a frame's local variable array is a root.

---

## 28. Package Manager (`rava-pkg`)

### 28.1 Architecture

```
User command (rava add spring-boot-web)
  │
  ▼ ShortNameResolver
    spring-boot-web → org.springframework.boot:spring-boot-starter-web
    (looked up from built-in short-name registry + ~/.rava/aliases.hcl)
  │
  ▼ VersionResolver
    Resolve version ranges from rava.hcl (^3.2.0 → >=3.2.0, <4.0.0)
  │
  ▼ DependencyGraph
    Recursively resolve transitive dependencies (download POMs from Maven Central)
    Conflict resolution: for the same groupId:artifactId, pick the highest compatible version
  │
  ▼ LockfileGen
    Generate rava.lock (exact versions + SHA-256 hashes)
  │
  ▼ ArtifactDownloader
    Download .jar files in parallel to ~/.rava/cache/repository/
    Verify SHA-256
    Support mirror registries (regional acceleration)
```

### 28.2 Key Types

```rust
pub trait ClassResolver: Send + Sync {
    /// Resolve a dependency coordinate → download/locate the .jar file path.
    fn resolve(&self, dep: &Dependency) -> Result<ResolvedArtifact>;
    /// List transitive dependencies.
    fn transitive_deps(&self, dep: &Dependency) -> Result<Vec<Dependency>>;
}

/// Default implementation: resolve from Maven Central.
pub struct MavenCentralResolver {
    base_url:    String,
    cache_dir:   PathBuf,
    http_client: reqwest::Client,
}

/// rava.lock format
pub struct Lockfile {
    pub generated_at: DateTime<Utc>,
    pub packages:     Vec<LockedPackage>,
}

pub struct LockedPackage {
    pub group_id:     String,
    pub artifact_id:  String,
    pub version:      String,       // exact version
    pub sha256:       String,       // content hash for reproducible builds
    pub url:          String,       // download source URL
    pub dependencies: Vec<String>,  // transitive dependency list
}
```

---

## 29. CLI (`rava-cli`)

The CLI is Rava's facade. It parses user commands and composes the capabilities of all other crates. It contains no business logic itself.

### 29.1 Command Dispatch

```rust
// rava-cli/src/main.rs
fn main() {
    let cli    = Cli::parse();  // clap parsing
    let result = match cli.command {
        Command::Run(args)     => commands::run::execute(args),
        Command::Build(args)   => commands::build::execute(args),
        Command::Init(args)    => commands::init::execute(args),
        Command::Add(args)     => commands::add::execute(args),
        Command::Remove(args)  => commands::remove::execute(args),
        Command::Update(args)  => commands::update::execute(args),
        Command::Test(args)    => commands::test::execute(args),
        Command::Fmt(args)     => commands::fmt::execute(args),
        Command::Lint(args)    => commands::lint::execute(args),
        Command::Repl          => commands::repl::execute(),
        Command::Publish(args) => commands::publish::execute(args),
        Command::Deps(args)    => commands::deps::execute(args),
        Command::Export(args)  => commands::export::execute(args),
        Command::Doctor        => commands::doctor::execute(),
        Command::Upgrade(args) => commands::upgrade::execute(args),
    };
    if let Err(e) = result {
        eprintln!("{}", e.render());  // format via DiagnosticEmitter
        std::process::exit(1);
    }
}
```

### 29.2 `rava run` Execution Flow

```
rava run Main.java
  │
  ▼ Detect: file vs project directory
  Has rava.hcl → project mode (read config)
  No rava.hcl  → single-file mode
  │
  ▼ Check incremental cache (target/cache/)
  File mtime changed → recompile changed files
  No change          → execute cached binary directly
  │
  ▼ Frontend (parse + type check + semantic analysis)
  │
  ▼ AOT Backend (compile to native binary)
  Write to target/ (or temp directory)
  │
  ▼ Execute (exec replaces the current process)
  Pass user arguments
```

---

## 30. Phased Implementation: Crate Deliverables

Maps to the phase plan in §17; breaks down deliverables at crate granularity.

### Phase 1 (6–12 months): Basic Toolchain

**Deliverable:** `rava run` executes pure-static Java; `rava build` produces a native binary.

| Crate | Work |
|-------|------|
| `rava-common` | Base types, errors, diagnostics |
| `rava-hcl` | rava.hcl parsing |
| `rava-frontend` | Java 21 lexing/parsing, basic type checking (no reflection passes) |
| `rava-rir` | RIR data structures |
| `rava-aot` | Optimization passes (DCE, constant folding) |
| `rava-codegen-cranelift` | x86-64 and ARM64 code generation |
| `rava-pkg` | Maven Central dependency download, rava.lock generation |
| `rava-cli` | `run`, `build`, `init`, `add`, `remove` commands |

Not yet supported: reflection, dynamic proxy, dynamic class loading.

### Phase 2 (3–6 months): Reflection Support

**Deliverable:** Reflection metadata table + dual-path dispatch; Jackson/Lombok fully operational.

| Crate | Work |
|-------|------|
| `rava-frontend` | `ReflectionPass` (mark AOT-resolvable reflection calls) |
| `rava-aot` | `MetadataTableGenPass` (generate `ClassMetadata`, embed in binary) |
| `rava-heap` | Basic heap allocation (no GC, arena-only) |
| `rava-cli` | Add `test`, `fmt` commands |

### Phase 3 (6–12 months): MicroRT v1

**Deliverable:** MicroRT bytecode interpreter; Spring Boot runs at a basic level.

| Crate | Work |
|-------|------|
| `rava-micrort` | Bytecode interpreter + class loader + bytecode verifier + ReflectionEngine |
| `rava-heap` | Full GC (stop-the-world + card-table write barrier) |
| `rava-heap` + `rava-micrort` | Unified object model: AOT ↔ MicroRT interop layer |
| `rava-frontend` | `ProxyPass` + `ClassLoadPass` |

### Phase 4 (2–3 months): Dynamic Proxy AOT Promotion

**Deliverable:** Spring `@Transactional` / MyBatis Mapper fully AOT; zero interpreter overhead.

| Crate | Work |
|-------|------|
| `rava-aot` | `ProxyPregenPass` (pre-generate proxy classes) + `MicroRtBridgePass` |

### Phase 5 (6–12 months): MicroRT JIT

**Deliverable:** Cranelift JIT; interpreted hot code approaches AOT speed.

| Crate | Work |
|-------|------|
| `rava-micrort` | `CraneliftJit` component (hotness counter + `BytecodeToClif` translator) |
| `rava-heap` | Concurrent GC (reduce stop-the-world pauses) |

---

## 31. GraalVM vs Rava

| Dimension | GraalVM native-image | Rava |
|-----------|---------------------|------|
| Reflection | User writes reflect-config.json manually | AOT metadata table auto-generated, zero config |
| Dynamic proxy | User writes proxy-config.json manually | AOT pre-generation + MicroRT fallback |
| Dynamic class loading | **Not supported** | MicroRT bytecode interpreter |
| User experience | Config error → runtime crash | User does nothing — it just works |
| Architectural philosophy | Closed-World Assumption | AOT primary + open escape hatch |
| Spring Boot | Requires spring-aot plugin | Zero config, MicroRT handles it automatically |
| Toolchain | GraalVM separate install (~400 MB) | Rava single binary (< 50 MB) |
| Config format | Maven/Gradle (XML/Groovy) | HCL (human-readable) |

> Rava's core technical moat lies in MicroRT and the unified object model. Seamlessly fusing AOT-compiled code with an embedded bytecode runtime in a single binary — while maintaining 10 ms startup and 20 MB memory — is something GraalVM cannot do, and the most urgent need in the entire Java ecosystem.
