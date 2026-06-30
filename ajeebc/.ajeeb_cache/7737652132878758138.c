#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

char __ajeeb_buf[4194304];
char __ajeeb_outbuf[65536];

static inline intptr_t getStateBuf(void) { return (intptr_t)__ajeeb_buf; }
static inline intptr_t getOutbuf(void) { __ajeeb_outbuf[0] = '\0'; return (intptr_t)__ajeeb_outbuf; }
static inline intptr_t getInt(intptr_t buf, intptr_t off) { return *(int64_t*)((char*)buf + off); }
static inline intptr_t setInt(intptr_t buf, intptr_t off, intptr_t val) { *(int64_t*)((char*)buf + off) = val; return val; }
static inline intptr_t charCode(intptr_t s, intptr_t i) { return (intptr_t)(unsigned char)((const char*)s)[i]; }
static inline intptr_t strSet(intptr_t s, intptr_t i, intptr_t c) { ((char*)s)[i] = (char)c; ((char*)s)[i+1] = '\0'; return c; }
static inline intptr_t allocBuf(intptr_t size) { return (intptr_t)calloc((size_t)size + 1, 1); }

intptr_t ajeeb_print(intptr_t);
intptr_t ajeeb_println(intptr_t);
intptr_t ajeeb_read_file(intptr_t);
intptr_t ajeeb_write_file(intptr_t, intptr_t);
intptr_t ajeeb_write_append(intptr_t, intptr_t);
intptr_t ajeeb_write_byte(intptr_t, intptr_t);
intptr_t ajeeb_exec(intptr_t);
intptr_t ajeeb_mkdir(intptr_t);
intptr_t ajeeb_read_arg(intptr_t);
intptr_t ajeeb_now_ms(void);

#include <stdarg.h>
static inline intptr_t __array_lit(intptr_t count, ...) {
    intptr_t* arr = (intptr_t*)calloc((size_t)(count + 2) * 8, 1);
    va_list ap; va_start(ap, count);
    intptr_t i; for (i = 0; i < count; i++) { arr[i + 2] = va_arg(ap, intptr_t); }
    va_end(ap); return (intptr_t)arr;
}
intptr_t array_to_string(intptr_t, intptr_t);

intptr_t __ipow(intptr_t base, intptr_t exp) { intptr_t r = 1; while (exp > 0) { r = r * base; exp = exp - 1; } return r; }

intptr_t greet(intptr_t);

intptr_t greet(intptr_t p0) {
    intptr_t t0, t1, t2, t3;

    goto block_0;
block_0:    t0 = p0;
    t2 = (intptr_t)"Hello ";
    t3 = str_concat(t2, t0);
    t1 = println(t3);

}

int main(void) {
    intptr_t t0, t1, t2;

    goto block_0;
block_0:    t1 = (intptr_t)"World";
    t0 = greet(t1);
    t2 = 0;
    return t2;

}

