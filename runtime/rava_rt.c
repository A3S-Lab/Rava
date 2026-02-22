/* rava_rt.c — Minimal C runtime for Rava AOT-compiled binaries.
 *
 * Provides: System.out.println, System.out.print, string operations.
 * Linked into every native binary produced by `rava build`.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>

/* Forward declaration — the AOT compiler always exports rava_entry as the entry point */
extern void rava_entry(long args);

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

/* ── Object operations ───────────────────────────────────────────────────── */

/*
 * Object layout: [num_fields: long][field_0: long][field_1: long]...
 * All fields are stored as long (I64). Pointers, ints, floats-as-bits.
 */

/* Allocate an object with `num_fields` slots, all zeroed. */
long rava_obj_alloc(long num_fields) {
    size_t size = (1 + num_fields) * sizeof(long);
    long* obj = (long*)calloc(1, size);
    if (!obj) return 0;
    obj[0] = 0; /* class tag — set later via rava_obj_set_tag */
    return (long)obj;
}

/* Set the class tag (stored in header slot 0). */
void rava_obj_set_tag(long obj_ptr, long tag) {
    if (obj_ptr == 0) return;
    long* obj = (long*)obj_ptr;
    obj[0] = tag;
}

/* Get the class tag (stored in header slot 0). */
long rava_obj_get_tag(long obj_ptr) {
    if (obj_ptr == 0) return 0;
    long* obj = (long*)obj_ptr;
    return obj[0];
}

/* Get field at slot index (0-based). */
long rava_obj_get_field(long obj_ptr, long slot) {
    if (obj_ptr == 0) return 0;
    long* obj = (long*)obj_ptr;
    return obj[1 + slot]; /* skip header */
}

/* Set field at slot index (0-based). */
void rava_obj_set_field(long obj_ptr, long slot, long value) {
    if (obj_ptr == 0) return;
    long* obj = (long*)obj_ptr;
    obj[1 + slot] = value;
}

/* ── Array operations ────────────────────────────────────────────────────── */

/*
 * Array layout: [length: long][elem_0: long][elem_1: long]...
 * All elements are stored as long (I64).
 */

/* Allocate an array of `length` elements, all zeroed. */
long rava_arr_alloc(long length) {
    if (length < 0) return 0;
    size_t size = (1 + length) * sizeof(long);
    long* arr = (long*)calloc(1, size);
    if (!arr) return 0;
    arr[0] = length;
    return (long)arr;
}

/* Load element at index. */
long rava_arr_load(long arr_ptr, long index) {
    if (arr_ptr == 0) return 0;
    long* arr = (long*)arr_ptr;
    if (index < 0 || index >= arr[0]) {
        fprintf(stderr, "ArrayIndexOutOfBoundsException: %ld\n", index);
        exit(1);
    }
    return arr[1 + index];
}

/* Store element at index. */
void rava_arr_store(long arr_ptr, long index, long value) {
    if (arr_ptr == 0) return;
    long* arr = (long*)arr_ptr;
    if (index < 0 || index >= arr[0]) {
        fprintf(stderr, "ArrayIndexOutOfBoundsException: %ld\n", index);
        exit(1);
    }
    arr[1 + index] = value;
}

/* Get array length. */
long rava_arr_len(long arr_ptr) {
    if (arr_ptr == 0) return 0;
    long* arr = (long*)arr_ptr;
    return arr[0];
}

/* ── System operations ───────────────────────────────────────────────────── */

void rava_system_exit(long code) {
    exit((int)code);
}

long rava_system_currenttimemillis(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (long)tv.tv_sec * 1000 + tv.tv_usec / 1000;
}

long rava_system_nanotime(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (long)tv.tv_sec * 1000000000L + (long)tv.tv_usec * 1000;
}

/* ── Math operations ─────────────────────────────────────────────────────── */

/* Returns a pseudo-random double between 0.0 and 1.0 as I64 bits. */
long rava_math_random(void) {
    static int seeded = 0;
    if (!seeded) {
        struct timeval tv;
        gettimeofday(&tv, NULL);
        srand((unsigned)(tv.tv_sec ^ tv.tv_usec));
        seeded = 1;
    }
    double r = (double)rand() / (double)RAND_MAX;
    long bits;
    memcpy(&bits, &r, sizeof(bits));
    return bits;
}

/* ── Entry point ──────────────────────────────────────────────────────────── */

int main(int argc, char** argv) {
    rava_entry(0);
    return 0;
}
