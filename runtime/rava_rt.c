/* rava_rt.c — Minimal C runtime for Rava AOT-compiled binaries.
 *
 * Provides: System.out.println, System.out.print, string operations.
 * Linked into every native binary produced by `rava build`.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Forward declaration — the AOT compiler exports Main_main (mangled from Main.main) */
extern void Main_main(long args);

/* ── System.out.println variants ──────────────────────────────────────────── */

void rava_println_int(long value) {
    printf("%ld\n", value);
}

void rava_println_float(double value) {
    if (value == (long)value && value < 1e15 && value > -1e15) {
        printf("%.1f\n", value);
    } else {
        printf("%g\n", value);
    }
}

void rava_println_str(long str_ptr) {
    if (str_ptr == 0) {
        printf("null\n");
    } else {
        printf("%ld\n", str_ptr);
    }
}

void rava_println_bool(long value) {
    printf("%s\n", value ? "true" : "false");
}

void rava_println_void(void) {
    printf("\n");
}

/* ── System.out.print variants ────────────────────────────────────────────── */

void rava_print_int(long value) {
    printf("%ld", value);
}

void rava_print_str(long str_ptr) {
    if (str_ptr == 0) {
        printf("null");
    } else {
        printf("%ld", str_ptr);
    }
}

/* ── String operations (Phase 1 stubs) ────────────────────────────────────── */

long rava_str_concat(long a, long b) {
    return 0;
}

long rava_int_to_str(long value) {
    return 0;
}

long rava_float_to_str(double value) {
    return 0;
}

/* ── Entry point ──────────────────────────────────────────────────────────── */

int main(int argc, char** argv) {
    Main_main(0);
    return 0;
}
