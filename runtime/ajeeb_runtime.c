#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

extern char __ajeeb_buf[16384];
extern char __ajeeb_outbuf[65536];

static char* saved_argv[256];
static int saved_argc = 0;

// Cached file handles for writeAppend/writeByte — avoid fopen/fclose per call
#define FILE_CACHE_SIZE 256
static struct { const char* path; FILE* fp; } file_cache[FILE_CACHE_SIZE];
static int file_cache_count = 0;

static void flush_cached_files(void) {
    for (int i = 0; i < file_cache_count; i++) {
        fflush(file_cache[i].fp);
        fclose(file_cache[i].fp);
        free((void*)file_cache[i].path);
    }
    file_cache_count = 0;
}

static FILE* get_cached_file(const char* path) {
    for (int i = 0; i < file_cache_count; i++) {
        if (strcmp(file_cache[i].path, path) == 0)
            return file_cache[i].fp;
    }
    if (file_cache_count >= FILE_CACHE_SIZE) return NULL;
    FILE* fp = fopen(path, "ab");
    if (!fp) return NULL;
    file_cache[file_cache_count].path = strdup(path);
    file_cache[file_cache_count].fp = fp;
    file_cache_count++;
    return fp;
}
static int args_init = 0;

static void init_args(void) {
    if (args_init) return;
    args_init = 1;
    atexit(flush_cached_files);
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
    FILE* f = get_cached_file(fname);
    if (!f) return;
    fwrite(data, 1, strlen(data), f);
    fflush(f);
}

void writeByte(intptr_t path, intptr_t byte) {
    const char* fname = (const char*)path;
    FILE* f = get_cached_file(fname);
    if (!f) return;
    fputc((char)byte, f);
    fflush(f);
}

intptr_t println(intptr_t s) {
    puts((const char*)s);
    return 0;
}

// Unique string allocation — no circular pool, no aliasing.
// Memory is allocated via malloc and reclaimed by OS at process exit.
// This eliminates all string corruption from buffer reuse.
static char* alloc_str(const char* src, size_t len) {
    if (len == 0) return strdup("");
    char* out = (char*)malloc(len + 1);
    if (!out) return strdup("");
    memcpy(out, src, len);
    out[len] = '\0';
    return out;
}

intptr_t itoa(intptr_t n) {
    char tmp[32];
    int len = snprintf(tmp, sizeof(tmp), "%ld", (long)n);
    return (intptr_t)alloc_str(tmp, len);
}

intptr_t strcmp_ajeeb(intptr_t a, intptr_t b) {
    return (intptr_t)strcmp((const char*)a, (const char*)b);
}

intptr_t str_concat(intptr_t a, intptr_t b) {
    const char* sa = (const char*)a;
    const char* sb = (const char*)b;
    size_t la = strlen(sa), lb = strlen(sb);
    size_t total = la + lb;
    char* out = (char*)malloc(total + 1);
    if (!out) return (intptr_t)"";
    memcpy(out, sa, la);
    memcpy(out + la, sb, lb);
    out[total] = '\0';
    return (intptr_t)out;
}

intptr_t substring(intptr_t s, intptr_t start, intptr_t end) {
    const char* src = (const char*)s;
    size_t slen = strlen(src);
    size_t st = (size_t)start;
    size_t en = (size_t)end;
    if (st > slen) st = slen;
    if (en > slen) en = slen;
    if (en < st) en = st;
    return (intptr_t)alloc_str(src + st, en - st);
}

intptr_t indexOf(intptr_t s, intptr_t search) {
    const char* str = (const char*)s;
    const char* sub = (const char*)search;
    const char* found = strstr(str, sub);
    if (found) return (intptr_t)(found - str);
    return (intptr_t)-1;
}

intptr_t contains(intptr_t s, intptr_t search) {
    const char* str = (const char*)s;
    const char* sub = (const char*)search;
    return (intptr_t)(strstr(str, sub) != NULL ? 1 : 0);
}

intptr_t toUpperCase(intptr_t s) {
    const char* src = (const char*)s;
    size_t slen = strlen(src);
    char* out = (char*)malloc(slen + 1);
    if (!out) return (intptr_t)"";
    for (size_t i = 0; i < slen; i++) {
        char c = src[i];
        out[i] = (c >= 'a' && c <= 'z') ? c - 32 : c;
    }
    out[slen] = '\0';
    return (intptr_t)out;
}

intptr_t toLowerCase(intptr_t s) {
    const char* src = (const char*)s;
    size_t slen = strlen(src);
    char* out = (char*)malloc(slen + 1);
    if (!out) return (intptr_t)"";
    for (size_t i = 0; i < slen; i++) {
        char c = src[i];
        out[i] = (c >= 'A' && c <= 'Z') ? c + 32 : c;
    }
    out[slen] = '\0';
    return (intptr_t)out;
}

intptr_t trim(intptr_t s) {
    const char* src = (const char*)s;
    while (*src == ' ' || *src == '\t' || *src == '\n' || *src == '\r') src++;
    if (*src == '\0') return (intptr_t)"";
    const char* end = src + strlen(src) - 1;
    while (end > src && (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r')) end--;
    return (intptr_t)alloc_str(src, end - src + 1);
}

intptr_t startsWith(intptr_t s, intptr_t prefix) {
    const char* str = (const char*)s;
    const char* pre = (const char*)prefix;
    size_t n = strlen(pre);
    return (intptr_t)(strncmp(str, pre, n) == 0 ? 1 : 0);
}

intptr_t endsWith(intptr_t s, intptr_t suffix) {
    const char* str = (const char*)s;
    const char* suf = (const char*)suffix;
    size_t slen = strlen(str);
    size_t suflen = strlen(suf);
    if (suflen > slen) return 0;
    return (intptr_t)(strncmp(str + slen - suflen, suf, suflen) == 0 ? 1 : 0);
}

intptr_t replace(intptr_t s, intptr_t from, intptr_t to) {
    const char* src = (const char*)s;
    const char* f = (const char*)from;
    const char* t = (const char*)to;
    size_t flen = strlen(f);
    size_t tlen = strlen(t);
    size_t capacity = strlen(src) * 2 + 1;
    char* out = (char*)malloc(capacity);
    if (!out) return (intptr_t)"";
    size_t out_pos = 0;
    while (*src && out_pos < capacity - 1) {
        if (strncmp(src, f, flen) == 0) {
            size_t copy = tlen;
            if (out_pos + copy >= capacity - 1) copy = capacity - 1 - out_pos;
            memcpy(out + out_pos, t, copy);
            out_pos += copy;
            src += flen;
        } else {
            out[out_pos++] = *src++;
        }
    }
    out[out_pos] = '\0';
    return (intptr_t)out;
}
