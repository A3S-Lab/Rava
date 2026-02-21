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
        printf("%s\n", (const char*)str_ptr);
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
        printf("%s", (const char*)str_ptr);
    }
}

/* ── String operations ────────────────────────────────────────────────────── */

/* Concatenate two strings. Returns a heap-allocated null-terminated string. */
long rava_str_concat(long a, long b) {
    const char* sa = (a == 0) ? "null" : (const char*)a;
    const char* sb = (b == 0) ? "null" : (const char*)b;
    size_t la = strlen(sa);
    size_t lb = strlen(sb);
    char* result = (char*)malloc(la + lb + 1);
    if (!result) return 0;
    memcpy(result, sa, la);
    memcpy(result + la, sb, lb);
    result[la + lb] = '\0';
    return (long)result;
}

/* Convert an integer to a heap-allocated string. */
long rava_int_to_str(long value) {
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%ld", value);
    char* result = (char*)malloc(len + 1);
    if (!result) return 0;
    memcpy(result, buf, len + 1);
    return (long)result;
}

/* Convert a float to a heap-allocated string. */
long rava_float_to_str(double value) {
    char buf[64];
    int len;
    if (value == (long)value && value < 1e15 && value > -1e15) {
        len = snprintf(buf, sizeof(buf), "%.1f", value);
    } else {
        len = snprintf(buf, sizeof(buf), "%g", value);
    }
    char* result = (char*)malloc(len + 1);
    if (!result) return 0;
    memcpy(result, buf, len + 1);
    return (long)result;
}

/* ── Entry point ──────────────────────────────────────────────────────────── */

int main(int argc, char** argv) {
    Main_main(0);
    return 0;
}
