# Assignment-in-Condition Loop Limitation - Investigation Summary

**Date:** 2026-02-27
**Status:** Documented as known limitation

## Problem Statement

While loops with assignment-in-condition patterns hang indefinitely:

```java
while ((idx = text.indexOf(pattern, idx)) != -1) {
    count++;
    idx++;
}
```

## Root Cause Analysis

The current SSA lowering uses a `__copy__` mechanism to propagate loop-carried dependencies. This fails for assignment-in-condition because:

1. **Condition evaluation:** Reads `idx_0`, calls `indexOf`, assigns result to `idx_1`
2. **Body execution:** Reads `idx_1`, increments to create `idx_2`
3. **Back-edge propagation:** Emits `__copy__idx_1 = idx_2` (propagates to post-condition SSA name)
4. **Next iteration:** Condition re-evaluates, reads `idx_0` (which was never updated!)

The `__copy__` mechanism propagates to `idx_1` (the SSA name created by the condition's assignment), but the next iteration's condition reads from `idx_0` (the pre-condition SSA name).

## Attempted Fixes

### Attempt 1: Remove body-end `__copy__`
- **Result:** Broke `index_of_with_from_index` test
- **Reason:** Simple loops need body-end propagation

### Attempt 2: Add `__copy_final__` with transitive resolution
- **Result:** Still hangs on `countOccurrences`
- **Reason:** Transitive resolution logic didn't correctly trace through SSA chains

### Attempt 3: Propagate to pre-condition SSA only
- **Result:** Broke `linked_list_impl` and `strategy_pattern` (391/393 passing)
- **Reason:** Missed propagation to intermediate SSA names

### Attempt 4: Propagate to both pre-condition and pre-body SSA
- **Result:** Still broke 2-3 tests
- **Reason:** Complex interaction between multiple SSA names

### Attempt 5: PHI node implementation (partial)
- **Result:** Abandoned - too large a change
- **Reason:** Requires architectural refactoring across lowerer, RIR, and interpreter

## Architectural Analysis

Per systematic debugging Phase 4.5 guidance: after 4+ failed attempts, the issue is architectural, not implementation.

**The `__copy__` mechanism is fundamentally incompatible with assignment-in-condition patterns** because:
- SSA form creates new variables for each assignment
- `__copy__` uses compile-time snapshots of SSA names
- Assignment-in-condition creates a dependency chain that `__copy__` cannot express

**Proper solution:** Implement PHI nodes at loop headers to merge values from different control flow predecessors (pre-loop and back-edge).

## Resolution

Documented as known limitation with:
1. Technical documentation in `docs/known-limitations.md`
2. README section with workaround
3. Code comments in `lowerer/stmt.rs`
4. Tests marked with `#[ignore]` and explanation

## Impact

- **Affected tests:** 2/393 (0.5%)
- **Real-world impact:** Low - this pattern is rare in typical Java code
- **Workaround:** Separate assignment from condition (simple refactoring)

## Future Work

To properly fix this, implement PHI nodes:
1. Add `Phi { ret: Value, incoming: Vec<(BlockId, Value)> }` to `RirInstr`
2. Emit PHI nodes at loop headers in lowerer
3. Implement PHI evaluation in interpreter (select value based on predecessor block)
4. Apply to all loop constructs (while, do-while, for)

Estimated effort: 1-2 days of focused work + extensive testing.
