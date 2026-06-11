#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

extern char __ajeeb_buf[16384];
extern char __ajeeb_outbuf[65536];

static char* saved_argv[256];
static int saved_argc = 0;
static int args_init = 0;

static void init_args(void) {
    if (args_init) return;
    args_init = 1;
    FILE* f = fopen("/proc/self/cmdline", "rb");
    if (!f) return;
    char buf[4096];
    int n = fread(buf, 1, sizeof(buf) - 1, f);
    fclose(f);
    if (n <= 0) return;
    buf[n] = '\0';
    int start = 0;
    for (int i = 0; i <= n; i++) {
        if (i == n || buf[i] == '\0') {
            if (saved_argc < 256) {
                int len = i - start;
                saved_argv[saved_argc] = (char*)malloc(len + 1);
                if (saved_argv[saved_argc]) {
                    memcpy(saved_argv[saved_argc], buf + start, len);
                    saved_argv[saved_argc][len] = '\0';
                }
                saved_argc++;
            }
            start = i + 1;
        }
    }
}

intptr_t getInt(intptr_t buf, intptr_t off) {
    return *(int64_t*)((char*)buf + off);
}

void setInt(intptr_t buf, intptr_t off, intptr_t v) {
    *(int64_t*)((char*)buf + off) = v;
}

intptr_t charCode(intptr_t s, intptr_t i) {
    return (unsigned char)((char*)s)[i];
}

intptr_t len(intptr_t s) {
    return (intptr_t)strlen((char*)s);
}

void strSet(intptr_t s, intptr_t i, intptr_t c) {
    ((char*)s)[i] = (char)c;
}

intptr_t getStateBuf() {
    return (intptr_t)__ajeeb_buf;
}

intptr_t getOutbuf() {
    return (intptr_t)__ajeeb_outbuf;
}

intptr_t readArg(intptr_t n) {
    init_args();
    if (n >= 0 && n < saved_argc) {
        return (intptr_t)saved_argv[n];
    }
    return (intptr_t)"";
}

intptr_t readFile(intptr_t path) {
    const char* fname = (const char*)path;
    FILE* f = fopen(fname, "rb");
    if (!f) return (intptr_t)"";
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    rewind(f);
    char* content = (char*)malloc(sz + 1);
    if (!content) { fclose(f); return (intptr_t)""; }
    fread(content, 1, sz, f);
    content[sz] = '\0';
    fclose(f);
    return (intptr_t)content;
}

void writeFile(intptr_t path, intptr_t content) {
    const char* fname = (const char*)path;
    const char* data = (const char*)content;
    FILE* f = fopen(fname, "wb");
    if (!f) return;
    fwrite(data, 1, strlen(data), f);
    fclose(f);
}

void writeAppend(intptr_t path, intptr_t content) {
    const char* fname = (const char*)path;
    const char* data = (const char*)content;
    FILE* f = fopen(fname, "ab");
    if (!f) return;
    fwrite(data, 1, strlen(data), f);
    fclose(f);
}

void writeByte(intptr_t path, intptr_t byte) {
    const char* fname = (const char*)path;
    FILE* f = fopen(fname, "ab");
    if (!f) return;
    fputc((char)byte, f);
    fclose(f);
}

intptr_t println(intptr_t s) {
    puts((const char*)s);
    return 0;
}

static char itoa_buf[32];

intptr_t itoa(intptr_t n) {
    snprintf(itoa_buf, sizeof(itoa_buf), "%ld", (long)n);
    return (intptr_t)itoa_buf;
}

intptr_t strcmp_ajeeb(intptr_t a, intptr_t b) {
    return (intptr_t)strcmp((const char*)a, (const char*)b);
}

#define STR_POOL_SLOTS 256
#define STR_POOL_SIZE 4096

static char str_pool[STR_POOL_SLOTS][STR_POOL_SIZE];
static int str_pool_idx = 0;

intptr_t str_concat(intptr_t a, intptr_t b) {
    const char* sa = (const char*)a;
    const char* sb = (const char*)b;
    size_t la = strlen(sa), lb = strlen(sb);
    int slot = str_pool_idx;
    str_pool_idx = (str_pool_idx + 1) % STR_POOL_SLOTS;
    char* out = str_pool[slot];
    size_t total = la + lb;
    if (total >= STR_POOL_SIZE - 1) total = STR_POOL_SIZE - 1;
    size_t copy_a = la < total ? la : total;
    memcpy(out, sa, copy_a);
    size_t copy_b = total - copy_a;
    if (copy_b > 0) memcpy(out + copy_a, sb, copy_b);
    out[total] = '\0';
    return (intptr_t)out;
}
