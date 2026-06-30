// ═══════════════════════════════════════════════════════════════
//  Ajeeb Runtime — Minimal C Bridge (syscalls only)
//  Phase 1: syscall_bridge.c — only OS-level operations
//  Everything else is implemented in runtime.ajb (pure Ajeeb)
// ═══════════════════════════════════════════════════════════════

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <unistd.h>
#include <fcntl.h>

// ── Arena Allocator ──────────────────────────────────────────
// Simple bump-pointer arena for string allocation.
#define ARENA_SIZE (4 * 1024 * 1024)  // 4 MB

static char* arena_base = NULL;
static size_t arena_offset = 0;
static size_t arena_capacity = 0;

static void arena_init(void) {
    if (!arena_base) {
        arena_capacity = ARENA_SIZE;
        arena_base = (char*)calloc(arena_capacity, 1);
        arena_offset = 0;
    }
}

static intptr_t ajeeb_alloc(intptr_t size) {
    arena_init();
    if (size <= 0) return (intptr_t)"";
    size_t aligned = ((size_t)size + 7) & ~7;
    if (arena_offset + aligned > arena_capacity) {
        // Grow arena
        while (arena_offset + aligned > arena_capacity) arena_capacity *= 2;
        char* new_base = (char*)realloc(arena_base, arena_capacity);
        if (!new_base) return (intptr_t)"";
        arena_base = new_base;
        memset(arena_base + arena_offset, 0, arena_capacity - arena_offset);
    }
    char* ptr = arena_base + arena_offset;
    arena_offset += aligned;
    memset(ptr, 0, (size_t)size);
    return (intptr_t)ptr;
}

// ── Stdout ───────────────────────────────────────────────────
intptr_t ajeeb_print(intptr_t s) {
    if (s) printf("%s", (const char*)s);
    return 0;
}

intptr_t ajeeb_println(intptr_t s) {
    if (s) printf("%s\n", (const char*)s);
    return 0;
}

// ── File I/O ─────────────────────────────────────────────────
intptr_t ajeeb_read_file(intptr_t path) {
    if (!path) return (intptr_t)"";
    const char* fname = (const char*)path;
    FILE* f = fopen(fname, "rb");
    if (!f) return (intptr_t)"";
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    if (sz < 0) { fclose(f); return (intptr_t)""; }
    rewind(f);
    char* content = (char*)ajeeb_alloc((intptr_t)(sz + 1));
    if (!content) { fclose(f); return (intptr_t)""; }
    size_t nread = fread(content, 1, (size_t)sz, f);
    content[nread] = '\0';
    fclose(f);
    return (intptr_t)content;
}

void ajeeb_write_file(intptr_t path, intptr_t content) {
    if (!path) return;
    const char* fname = (const char*)path;
    const char* data = (const char*)content;
    if (!data) data = "";
    FILE* f = fopen(fname, "wb");
    if (!f) return;
    fwrite(data, 1, strlen(data), f);
    fclose(f);
}

void ajeeb_write_append(intptr_t path, intptr_t content) {
    if (!path) return;
    const char* fname = (const char*)path;
    const char* data = (const char*)content;
    if (!data) data = "";
    FILE* f = fopen(fname, "ab");
    if (!f) return;
    fwrite(data, 1, strlen(data), f);
    fclose(f);
}

void ajeeb_write_byte(intptr_t path, intptr_t byte) {
    if (!path) return;
    const char* fname = (const char*)path;
    FILE* f = fopen(fname, "ab");
    if (!f) return;
    fputc((char)byte, f);
    fclose(f);
}

// ── System ───────────────────────────────────────────────────
intptr_t ajeeb_exec(intptr_t cmd) {
    if (!cmd) return -1;
    return system((const char*)cmd);
}

intptr_t ajeeb_mkdir(intptr_t path) {
    if (!path || !*(const char*)path) return -1;
    char cmd[8192];
    snprintf(cmd, sizeof(cmd), "mkdir -p '%s'", (const char*)path);
    return system(cmd);
}

// ── CLI Arguments ───────────────────────────────────────────
static int saved_argc = 0;
static char** saved_argv = NULL;
static int args_init = 0;

static void init_args(void) {
    if (args_init) return;
    args_init = 1;
    // Read from /proc/self/cmdline on Linux
    FILE* f = fopen("/proc/self/cmdline", "rb");
    if (!f) { saved_argv = NULL; saved_argc = 0; return; }
    char buf[4096];
    int n = fread(buf, 1, sizeof(buf) - 1, f);
    fclose(f);
    if (n <= 0) return;
    buf[n] = '\0';
    // Count args
    int count = 0;
    for (int i = 0; i < n; i++) if (buf[i] == '\0') count++;
    saved_argv = (char**)calloc((size_t)(count + 1), sizeof(char*));
    int start = 0;
    int idx = 0;
    for (int i = 0; i <= n; i++) {
        if (i == n || buf[i] == '\0') {
            int len = i - start;
            saved_argv[idx] = (char*)ajeeb_alloc(len + 1);
            if (saved_argv[idx]) {
                memcpy(saved_argv[idx], buf + start, (size_t)len);
                saved_argv[idx][len] = '\0';
            }
            idx++;
            start = i + 1;
        }
    }
    saved_argc = count;
}

intptr_t ajeeb_read_arg(intptr_t i) {
    init_args();
    if (i >= 0 && i < saved_argc && saved_argv) {
        return (intptr_t)saved_argv[(int)i];
    }
    return (intptr_t)"";
}

// ── Array Ops ───────────────────────────────────────────────
#include <stdarg.h>

intptr_t ajeeb_array_lit(intptr_t count, ...) {
    if (count < 0 || count > 1000000) return 0;
    va_list args;
    va_start(args, count);
    intptr_t* arr = (intptr_t*)calloc((size_t)(count + 1), sizeof(intptr_t));
    if (!arr) { va_end(args); return 0; }
    arr[0] = count;
    for (intptr_t i = 0; i < count; i++) {
        arr[i + 1] = va_arg(args, intptr_t);
    }
    va_end(args);
    return (intptr_t)arr;
}

intptr_t ajeeb_index(intptr_t arr, intptr_t idx) {
    if (arr == 0) return 0;
    int64_t* p = (int64_t*)arr;
    int64_t len = p[0];
    if (idx < 0 || idx >= len) return 0;
    return p[idx + 1];
}

intptr_t ajeeb_index_assign(intptr_t arr, intptr_t idx, intptr_t val) {
    if (arr == 0) return 0;
    int64_t* p = (int64_t*)arr;
    int64_t len = p[0];
    if (idx < 0 || idx >= len) return val;
    p[idx + 1] = val;
    return val;
}

intptr_t ajeeb_arr_len(intptr_t arr) {
    if (arr == 0) return 0;
    return ((int64_t*)arr)[0];
}

intptr_t ajeeb_array_to_string(intptr_t ptr, intptr_t len) {
    if (!ptr) { return (intptr_t)"[]"; }
    // Simple array to string
    char buf[4096];
    int pos = 0;
    buf[pos] = '['; pos++;
    for (intptr_t i = 0; i < len && i < 500; i++) {
        if (i > 0) { buf[pos] = ','; pos++; buf[pos] = ' '; pos++; }
        int written = snprintf(buf + pos, sizeof(buf) - (size_t)pos - 1, "%ld", (long)((int64_t*)ptr)[i]);
        if (written > 0) pos += written;
    }
    buf[pos] = ']'; pos++;
    buf[pos] = '\0';
    return (intptr_t)ajeeb_alloc(pos + 1);
}

// ── Time ─────────────────────────────────────────────────────
intptr_t ajeeb_now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (intptr_t)(ts.tv_sec * 1000LL + ts.tv_nsec / 1000000);
}

// ── Leak check (called at exit) ─────────────────────────────
void ajeeb_leak_check(void) {
    // In this minimal bridge, we don't track allocations.
    // Arena is freed at OS level on process exit.
}
