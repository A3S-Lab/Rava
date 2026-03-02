/**
 * rava-rt: Minimal Java runtime for AOT-compiled binaries.
 *
 * Provides the C functions that Cranelift-generated code calls:
 *   - I/O: rava_println_*, rava_print_*
 *   - Strings: rava_str_concat, rava_int_to_str, rava_float_to_str
 *   - Objects: rava_obj_alloc, rava_obj_get_field, rava_obj_set_field
 *   - Arrays: rava_arr_alloc, rava_arr_load, rava_arr_store, rava_arr_len
 *   - Tags: rava_obj_set_tag, rava_obj_get_tag
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* ── String pool ─────────────────────────────────────────────────────────── */

#define STR_POOL_CAP 65536
static char  str_pool[STR_POOL_CAP];
static int   str_pool_pos = 0;

static const char* pool_str(const char* s) {
    int len = strlen(s) + 1;
    if (str_pool_pos + len > STR_POOL_CAP) return s; /* fallback */
    char* dst = str_pool + str_pool_pos;
    memcpy(dst, s, len);
    str_pool_pos += len;
    return dst;
}

/* ── I/O ─────────────────────────────────────────────────────────────────── */

void rava_println_int(int64_t v)   { printf("%lld\n", (long long)v); }
void rava_println_float(double v)  { printf("%g\n", v); }
void rava_println_str(int64_t ptr) { printf("%s\n", (const char*)(uintptr_t)ptr); }
void rava_println_bool(int64_t v)  { printf("%s\n", v ? "true" : "false"); }
void rava_println_void(void)       { printf("\n"); }
void rava_print_int(int64_t v)     { printf("%lld", (long long)v); }
void rava_print_str(int64_t ptr)   { printf("%s", (const char*)(uintptr_t)ptr); }

/* ── String operations ───────────────────────────────────────────────────── */

int64_t rava_str_concat(int64_t a, int64_t b) {
    const char* sa = (const char*)(uintptr_t)a;
    const char* sb = (const char*)(uintptr_t)b;
    size_t la = sa ? strlen(sa) : 0;
    size_t lb = sb ? strlen(sb) : 0;
    char* buf = (char*)malloc(la + lb + 1);
    if (sa) memcpy(buf, sa, la);
    if (sb) memcpy(buf + la, sb, lb);
    buf[la + lb] = '\0';
    return (int64_t)(uintptr_t)buf;
}

int64_t rava_int_to_str(int64_t v) {
    char* buf = (char*)malloc(32);
    snprintf(buf, 32, "%lld", (long long)v);
    return (int64_t)(uintptr_t)buf;
}

int64_t rava_float_to_str(double v) {
    char* buf = (char*)malloc(32);
    snprintf(buf, 32, "%g", v);
    return (int64_t)(uintptr_t)buf;
}

/* ── Object layout ───────────────────────────────────────────────────────── */
/*
 * Object header: [tag: i64][num_fields: i64][fields: i64 * num_fields]
 * Total size: (2 + num_fields) * 8 bytes
 */

#define OBJ_TAG_OFFSET    0
#define OBJ_NFIELDS_OFFSET 8
#define OBJ_FIELDS_OFFSET  16

int64_t rava_obj_alloc(int64_t num_fields) {
    size_t size = (size_t)(2 + num_fields) * 8;
    int64_t* obj = (int64_t*)calloc(1, size);
    obj[0] = 0;           /* tag */
    obj[1] = num_fields;  /* num_fields */
    return (int64_t)(uintptr_t)obj;
}

void rava_obj_set_tag(int64_t ptr, int64_t tag) {
    int64_t* obj = (int64_t*)(uintptr_t)ptr;
    obj[0] = tag;
}

int64_t rava_obj_get_tag(int64_t ptr) {
    if (!ptr) return 0;
    int64_t* obj = (int64_t*)(uintptr_t)ptr;
    return obj[0];
}

void rava_obj_set_field(int64_t ptr, int64_t slot, int64_t val) {
    if (!ptr) return;
    int64_t* obj = (int64_t*)(uintptr_t)ptr;
    int64_t nfields = obj[1];
    if (slot >= 0 && slot < nfields) {
        obj[2 + slot] = val;
    }
}

int64_t rava_obj_get_field(int64_t ptr, int64_t slot) {
    if (!ptr) return 0;
    int64_t* obj = (int64_t*)(uintptr_t)ptr;
    int64_t nfields = obj[1];
    if (slot >= 0 && slot < nfields) {
        return obj[2 + slot];
    }
    return 0;
}

/* ── Array layout ────────────────────────────────────────────────────────── */
/*
 * Array header: [len: i64][elements: i64 * len]
 */

int64_t rava_arr_alloc(int64_t len) {
    if (len < 0) len = 0;
    int64_t* arr = (int64_t*)calloc(1, (size_t)(1 + len) * 8);
    arr[0] = len;
    return (int64_t)(uintptr_t)arr;
}

int64_t rava_arr_len(int64_t ptr) {
    if (!ptr) return 0;
    int64_t* arr = (int64_t*)(uintptr_t)ptr;
    return arr[0];
}

int64_t rava_arr_load(int64_t ptr, int64_t idx) {
    if (!ptr) return 0;
    int64_t* arr = (int64_t*)(uintptr_t)ptr;
    int64_t len = arr[0];
    if (idx < 0 || idx >= len) return 0;
    return arr[1 + idx];
}

void rava_arr_store(int64_t ptr, int64_t idx, int64_t val) {
    if (!ptr) return;
    int64_t* arr = (int64_t*)(uintptr_t)ptr;
    int64_t len = arr[0];
    if (idx >= 0 && idx < len) {
        arr[1 + idx] = val;
    }
}

/* ── Bool / extra string conversions ────────────────────────────────────── */

int64_t rava_bool_to_str(int64_t v) {
    return (int64_t)(uintptr_t)(v ? "true" : "false");
}

/* ── Array iterator ──────────────────────────────────────────────────────── */
/*
 * Iterator layout: [arr_ptr: i64][current_index: i64]
 */

int64_t rava_iter_new(int64_t arr_ptr) {
    int64_t* iter = (int64_t*)malloc(2 * sizeof(int64_t));
    iter[0] = arr_ptr;
    iter[1] = 0;
    return (int64_t)(uintptr_t)iter;
}

int64_t rava_iter_has_next(int64_t iter_ptr) {
    if (!iter_ptr) return 0;
    int64_t* iter = (int64_t*)(uintptr_t)iter_ptr;
    int64_t len = rava_arr_len(iter[0]);
    return iter[1] < len ? 1 : 0;
}

int64_t rava_iter_next(int64_t iter_ptr) {
    if (!iter_ptr) return 0;
    int64_t* iter = (int64_t*)(uintptr_t)iter_ptr;
    int64_t val = rava_arr_load(iter[0], iter[1]);
    iter[1]++;
    return val;
}

/* ── Math functions ──────────────────────────────────────────────────────── */

#include <math.h>

int64_t rava_math_sqrt(int64_t bits) {
    double v;
    memcpy(&v, &bits, 8);
    double r = sqrt(v);
    int64_t out;
    memcpy(&out, &r, 8);
    return out;
}

int64_t rava_math_pow(int64_t base_bits, int64_t exp_bits) {
    double b, e;
    memcpy(&b, &base_bits, 8);
    memcpy(&e, &exp_bits, 8);
    double r = pow(b, e);
    int64_t out;
    memcpy(&out, &r, 8);
    return out;
}

int64_t rava_math_abs_int(int64_t v)   { return v < 0 ? -v : v; }
int64_t rava_math_min_int(int64_t a, int64_t b) { return a < b ? a : b; }
int64_t rava_math_max_int(int64_t a, int64_t b) { return a > b ? a : b; }

/* ── String comparison ───────────────────────────────────────────────────── */

int64_t rava_str_equals(int64_t a, int64_t b) {
    const char* sa = a ? (const char*)(uintptr_t)a : "";
    const char* sb = b ? (const char*)(uintptr_t)b : "";
    return strcmp(sa, sb) == 0 ? 1 : 0;
}

int64_t rava_str_compare_to(int64_t a, int64_t b) {
    const char* sa = a ? (const char*)(uintptr_t)a : "";
    const char* sb = b ? (const char*)(uintptr_t)b : "";
    return (int64_t)strcmp(sa, sb);
}
