# Assignment-in-Condition Loop Limitation - Investigation Summary

**Date:** 2026-02-27
**Status:** Fixed (2026-02-28)

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

Implemented a loop-carried value propagation fix in lowering:
1. Snapshot variable map before condition lowering (`pre_cond_vars`)
2. Keep existing post-condition snapshot (`pre_body_vars`)
3. On loop back-edge, propagate updated values to both snapshots

This ensures variables read in condition-before-assignment and variables used in body both
observe the latest loop-carried value on the next iteration.

## Validation

- Re-enabled and passed the two previously ignored e2e tests:
  - `string_manipulation_advanced`
  - `index_of_with_from_index`
- Current status: full e2e pass for this suite section (393/393)

## Future Work

The current fix resolves the reported limitation without introducing new IR instructions.
If the loop lowering model is redesigned in the future, explicit block-parameter/PHI-style
representation is still a viable architectural cleanup direction.
