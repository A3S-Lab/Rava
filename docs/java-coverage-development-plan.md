# Java Coverage Development Plan

This plan tracks the remaining work required to move from high practical coverage to near-complete Java compatibility.

## Current Status

- Parser and checker coverage is high for common Java syntax.
- Generic parsing and overload resolution have been significantly improved.
- Workspace tests currently pass (`cargo test --workspace`).
- Full JLS-level parity is not yet reached.

## Priority Roadmap

### P0: Runtime Semantics Completion

- [ ] Implement missing interpreter semantics paths in MicroRT.
- [ ] Complete verifier behavior for stricter Java correctness checks.
- [ ] Expand reflection/runtime metadata behavior to match expected Java usage.

### P1: Type System and Resolution Parity

- [ ] Continue refining generic type inference in complex nested and overloaded scenarios.
- [ ] Extend overload resolution toward closer JLS method selection parity.
- [ ] Improve bound checking and edge-case diagnostics for generic constraints.

### P2: Syntax and Frontend Completeness

- [ ] Perform chapter-by-chapter JLS parser audit and fill syntax edge gaps.
- [ ] Complete annotation semantics pipeline beyond declaration parsing.
- [ ] Expand module system semantic checks for `module-info` directives.

### P3: Standard Library and Behavioral Compatibility

- [ ] Increase long-tail JDK API compatibility coverage in runtime behavior.
- [ ] Add behavior-focused regression tests for subtle API/exception differences.
- [ ] Validate edge-case parity with representative Java reference outputs.

## Validation Strategy

- [ ] Keep all existing tests green after each feature increment.
- [ ] Add targeted frontend unit tests for every parser/checker rule change.
- [ ] Add focused MicroRT e2e tests for every runtime semantic addition.
- [ ] Run `cargo test --workspace` as a required gate for each milestone.

## Execution Order

1. Finish runtime semantics gaps (P0).
2. Tighten type-system and overload parity (P1).
3. Complete syntax/module/annotation edge coverage (P2).
4. Improve long-tail library behavior compatibility (P3).
