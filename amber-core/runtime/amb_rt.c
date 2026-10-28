/* amber-core/runtime/amb_rt.c
 * Runtime support for the Amberlink LLVM AOT backend.
 *
 * Generated code manipulates tagged values through pointers (never by value),
 * so there is no struct-passing ABI to get wrong between hand-written LLVM IR
 * and C. Semantics deliberately mirror amber-vm (avm.cpp): same type rules,
 * same error messages, same print formatting.
 *
 * Build (done for you by `ambc file.amb --native out`):
 *   clang -O2 program.ll amb_rt.c -o program
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Tags match the VM's ValueType enum order. */
enum { T_INT = 0, T_FLOAT = 1, T_BOOL = 2, T_CHAR = 3, T_OBJ = 4, T_STR = 5, T_NULL = 6 };

typedef struct {
    int32_t tag;
    int32_t bits; /* int32 / float bits / bool / char / string-table index */
} AmbValue;

static char **g_strs = NULL;
static int32_t g_nstrs = 0;
static int32_t g_cap = 0;

static void die(const char *msg) {
    fflush(stdout);
    fprintf(stderr, "Runtime error: %s\n", msg);
    exit(1);
}

static float as_f(const AmbValue *v) {
    float f;
    memcpy(&f, &v->bits, sizeof f);
    return f;
}

static void set_int(AmbValue *o, int32_t i) { o->tag = T_INT; o->bits = i; }
static void set_bool(AmbValue *o, int b) { o->tag = T_BOOL; o->bits = b ? 1 : 0; }
static void set_float(AmbValue *o, float f) {
    o->tag = T_FLOAT;
    memcpy(&o->bits, &f, sizeof f);
}

void amb_init(const char *const *consts, int32_t n) {
    g_cap = n < 16 ? 16 : n * 2;
    g_strs = (char **)malloc(sizeof(char *) * (size_t)g_cap);
    if (!g_strs) die("out of memory");
    for (int32_t i = 0; i < n; ++i) g_strs[i] = (char *)consts[i];
    g_nstrs = n;
}

static int32_t add_string(char *s) {
    if (g_nstrs == g_cap) {
        g_cap *= 2;
        g_strs = (char **)realloc(g_strs, sizeof(char *) * (size_t)g_cap);
        if (!g_strs) die("out of memory");
    }
    g_strs[g_nstrs] = s;
    return g_nstrs++;
}

static const char *str_of(const AmbValue *v) {
    if (v->bits < 0 || v->bits >= g_nstrs) return "<Invalid String Index>";
    return g_strs[v->bits];
}

void amb_add(AmbValue *out, const AmbValue *pa, const AmbValue *pb) {
    AmbValue a = *pa, b = *pb;
    if (a.tag == T_INT && b.tag == T_INT) set_int(out, (int32_t)((uint32_t)a.bits + (uint32_t)b.bits));
    else if (a.tag == T_FLOAT && b.tag == T_FLOAT) set_float(out, as_f(&a) + as_f(&b));
    else if (a.tag == T_STR && b.tag == T_STR) {
        const char *x = str_of(&a), *y = str_of(&b);
        size_t lx = strlen(x), ly = strlen(y);
        char *s = (char *)malloc(lx + ly + 1);
        if (!s) die("out of memory");
        memcpy(s, x, lx);
        memcpy(s + lx, y, ly + 1);
        out->tag = T_STR;
        out->bits = add_string(s);
    } else die("Type mismatch: Cannot add incompatible types.");
}

void amb_sub(AmbValue *out, const AmbValue *pa, const AmbValue *pb) {
    AmbValue a = *pa, b = *pb;
    if (a.tag == T_INT && b.tag == T_INT) set_int(out, (int32_t)((uint32_t)a.bits - (uint32_t)b.bits));
    else if (a.tag == T_FLOAT && b.tag == T_FLOAT) set_float(out, as_f(&a) - as_f(&b));
    else die("Type mismatch: Cannot subtract incompatible types.");
}

void amb_mul(AmbValue *out, const AmbValue *pa, const AmbValue *pb) {
    AmbValue a = *pa, b = *pb;
    if (a.tag == T_INT && b.tag == T_INT) set_int(out, (int32_t)((uint32_t)a.bits * (uint32_t)b.bits));
    else if (a.tag == T_FLOAT && b.tag == T_FLOAT) set_float(out, as_f(&a) * as_f(&b));
    else die("Type mismatch: Cannot multiply incompatible types.");
}

void amb_div(AmbValue *out, const AmbValue *pa, const AmbValue *pb) {
    AmbValue a = *pa, b = *pb;
    if (a.tag == T_INT && b.tag == T_INT) {
        if (b.bits == 0) die("Division by zero.");
        if (a.bits == INT32_MIN && b.bits == -1) set_int(out, INT32_MIN);
        else set_int(out, a.bits / b.bits);
    } else if (a.tag == T_FLOAT && b.tag == T_FLOAT) {
        if (as_f(&b) == 0.0f) die("Division by zero.");
        set_float(out, as_f(&a) / as_f(&b));
    } else die("Type mismatch: Cannot divide incompatible types.");
}

/* op: 0 <, 1 >, 2 ==, 3 !=, 4 <=, 5 >= */
void amb_cmp(AmbValue *out, const AmbValue *pa, const AmbValue *pb, int32_t op) {
    AmbValue a = *pa, b = *pb;
    int r;
    if (a.tag == T_INT && b.tag == T_INT) {
        int32_t x = a.bits, y = b.bits;
        r = op == 0 ? x < y : op == 1 ? x > y : op == 2 ? x == y : op == 3 ? x != y : op == 4 ? x <= y : x >= y;
    } else if (a.tag == T_FLOAT && b.tag == T_FLOAT) {
        float x = as_f(&a), y = as_f(&b);
        r = op == 0 ? x < y : op == 1 ? x > y : op == 2 ? x == y : op == 3 ? x != y : op == 4 ? x <= y : x >= y;
    } else if (a.tag == T_CHAR && b.tag == T_CHAR) {
        char x = (char)a.bits, y = (char)b.bits;
        r = op == 0 ? x < y : op == 1 ? x > y : op == 2 ? x == y : op == 3 ? x != y : op == 4 ? x <= y : x >= y;
    } else if (a.tag == T_BOOL && b.tag == T_BOOL && (op == 2 || op == 3)) {
        r = op == 2 ? (a.bits != 0) == (b.bits != 0) : (a.bits != 0) != (b.bits != 0);
    } else if (a.tag == T_STR && b.tag == T_STR && (op == 2 || op == 3)) {
        int eq = strcmp(str_of(&a), str_of(&b)) == 0;
        r = op == 2 ? eq : !eq;
    } else {
        die("Type mismatch: Cannot compare incompatible types.");
        r = 0;
    }
    set_bool(out, r);
}

/* Returns 1 when a JUMP_IF_FALSE should be taken. */
int32_t amb_is_false(const AmbValue *v) {
    if (v->tag == T_BOOL) return v->bits == 0;
    if (v->tag == T_INT) return v->bits == 0;
    die("JUMP_IF_FALSE requires a boolean or integer condition.");
    return 0;
}

void amb_check_null(const AmbValue *v) {
    if (v->tag == T_NULL) die("Null pointer error: value is null.");
}

void amb_print(const AmbValue *v) {
    switch (v->tag) {
        case T_INT: printf("%d\n", v->bits); break;
        case T_FLOAT: printf("%g\n", (double)as_f(v)); break;
        case T_BOOL: printf("%s\n", v->bits ? "true" : "false"); break;
        case T_CHAR: printf("'%c'\n", (char)v->bits); break;
        case T_OBJ: printf("ObjRef(%d)\n", v->bits); break;
        case T_STR: printf("%s\n", str_of(v)); break;
        default: printf("null\n"); break;
    }
}
