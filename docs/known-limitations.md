# Known Limitations

This document tracks known limitations and edge cases in the Rava compiler and runtime.

## While Loop with Assignment-in-Condition Pattern

**Status:** Known limitation (as of 2026-02-27)

**Affected Pattern:**
```java
while ((variable = expression) != value) {
    // body that modifies variable
}
```

**Example:**
```java
int idx = 0;
while ((idx = text.indexOf(pattern, idx)) != -1) {
    count++;
    idx++;
}
```

**Issue:**
The current SSA lowering with `__copy__` mechanism cannot correctly handle loops where:
1. The condition contains an assignment that creates a new SSA variable
2. The loop body further modifies the same variable

**Root Cause:**
- The condition reads from `idx_0` and assigns to `idx_1`
- The body reads `idx_1` and creates `idx_2`
- The `__copy__` mechanism propagates `idx_2` back to `idx_1`
- But the next iteration's condition still reads from `idx_0` (which never gets updated)
- This creates an infinite loop

**Workaround:**
Rewrite the pattern to separate the assignment from the condition:

```java
// Instead of:
while ((idx = text.indexOf(pattern, idx)) != -1) {
    count++;
    idx++;
}

// Use:
idx = text.indexOf(pattern, idx);
while (idx != -1) {
    count++;
    idx++;
    idx = text.indexOf(pattern, idx);
}
```

**Proper Fix:**
Implement PHI nodes at loop headers to properly merge values from different control flow predecessors. This requires:
1. Adding `Phi` instruction to RIR
2. Modifying the lowerer to emit PHI nodes at loop headers
3. Updating the interpreter to handle PHI node evaluation
4. Ensuring all loop constructs (while, do-while, for) use PHI nodes

**Affected Tests:**
- `string_manipulation_advanced` (countOccurrences method)
- `index_of_with_from_index` (assignment-in-condition pattern)

**Impact:**
Low - this pattern is relatively rare in typical Java code. Most loops use simple conditions without embedded assignments.
