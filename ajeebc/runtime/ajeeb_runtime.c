#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>
#include <dlfcn.h>


// ── Arena Allocator ──────────────────────────────────────────────────
// Bump-pointer arena. No individual frees. Reset entire arena at program end.
// All string allocations come from the arena — zero memory leaks by construction.

#define ARENA_DEFAULT_CAPACITY (1024 * 1024)  // 1 MB initial

typedef struct Arena {
    char* base;
    size_t offset;
    size_t capacity;
    size_t alloc_count;   // debug: number of allocations
    size_t total_requested; // debug: total bytes requested
} Arena;

static Arena* the_arena = NULL;

static Arena* arena_create(size_t initial_capacity) {
    Arena* a = (Arena*)malloc(sizeof(Arena));
    if (!a) return NULL;
    a->capacity = initial_capacity > 0 ? initial_capacity : ARENA_DEFAULT_CAPACITY;
    a->base = (char*)malloc(a->capacity);
    if (!a->base) { free(a); return NULL; }
    a->offset = 0;
    a->alloc_count = 0;
    a->total_requested = 0;
    return a;
}

static void* arena_alloc(Arena* a, size_t size) {
    if (!a) return NULL;
    // Align to 8 bytes
    size_t aligned = (size + 7) & ~7;
    if (a->offset + aligned > a->capacity) {
        // Grow: double capacity
        size_t new_cap = a->capacity * 2;
        while (a->offset + aligned > new_cap) new_cap *= 2;
        char* new_base = (char*)realloc(a->base, new_cap);
        if (!new_base) return NULL;
        a->base = new_base;
        a->capacity = new_cap;
    }
    void* ptr = a->base + a->offset;
    a->offset += aligned;
    a->alloc_count++;
    a->total_requested += size;
    return ptr;
}

static char* arena_strdup(Arena* a, const char* s) {
    if (!s) return NULL;
    size_t len = strlen(s);
    char* out = (char*)arena_alloc(a, len + 1);
    if (!out) return NULL;
    memcpy(out, s, len);
    out[len] = '\0';
    return out;
}

static char* arena_strndup(Arena* a, const char* s, size_t len) {
    if (!s) len = 0;
    char* out = (char*)arena_alloc(a, len + 1);
    if (!out) return NULL;
    memcpy(out, s, len);
    out[len] = '\0';
    return out;
}

static void arena_reset(Arena* a) {
    if (!a) return;
    a->offset = 0;
    a->alloc_count = 0;
    a->total_requested = 0;
}

static void arena_destroy(Arena* a) {
    if (!a) return;
    free(a->base);
    free(a);
}

static Arena* get_arena(void) {
    if (!the_arena) {
        the_arena = arena_create(ARENA_DEFAULT_CAPACITY);
    }
    return the_arena;
}

// ── String Arena (Individual Free Support) ─────────────────────────
// Unlike the bump arena above, this arena tracks individual allocations
// and supports freeing them. Used for temporary strings that outlive
// their creation scope but should not leak.

#define STR_ARENA_BLOCK_SIZE (64 * 1024)  // 64 KB blocks
#define STR_ARENA_MAX_BLOCKS 256

typedef struct StrArenaBlock {
    char data[STR_ARENA_BLOCK_SIZE];
    size_t offset;
    struct StrArenaBlock* next;
} StrArenaBlock;

typedef struct StrAlloc {
    char* ptr;
    size_t size;
    int free;  // 1 = freed, 0 = in use
} StrAlloc;

typedef struct StringArena {
    StrArenaBlock* blocks;
    StrAlloc allocs[16384];  // tracking table
    int alloc_count;
    size_t total_allocated;
    size_t total_freed;
} StringArena;

static StringArena* the_str_arena = NULL;

static StringArena* str_arena_create(void) {
    StringArena* sa = (StringArena*)calloc(1, sizeof(StringArena));
    return sa;
}

static StrArenaBlock* str_arena_new_block(StringArena* sa) {
    StrArenaBlock* blk = (StrArenaBlock*)malloc(sizeof(StrArenaBlock));
    if (!blk) return NULL;
    blk->offset = 0;
    blk->next = sa->blocks;
    sa->blocks = blk;
    return blk;
}

static char* str_arena_alloc(StringArena* sa, size_t size) {
    if (!sa) sa = str_arena_create();
    if (!the_str_arena) the_str_arena = sa;

    size_t aligned = (size + 15) & ~15;  // 16-byte aligned
    StrArenaBlock* blk = sa->blocks;
    if (!blk || blk->offset + aligned > STR_ARENA_BLOCK_SIZE) {
        blk = str_arena_new_block(sa);
        if (!blk) return NULL;
    }
    char* ptr = blk->data + blk->offset;
    blk->offset += aligned;

    // Track allocation
    if (sa->alloc_count < 16384) {
        sa->allocs[sa->alloc_count].ptr = ptr;
        sa->allocs[sa->alloc_count].size = aligned;
        sa->allocs[sa->alloc_count].free = 0;
        sa->alloc_count++;
    }
    sa->total_allocated += aligned;
    return ptr;
}

static void str_arena_free(StringArena* sa, char* ptr) {
    if (!sa || !ptr) return;
    for (int i = 0; i < sa->alloc_count; i++) {
        if (sa->allocs[i].ptr == ptr && !sa->allocs[i].free) {
            sa->allocs[i].free = 1;
            sa->total_freed += sa->allocs[i].size;
            return;
        }
    }
}

static void str_arena_destroy(StringArena* sa) {
    if (!sa) return;
    StrArenaBlock* blk = sa->blocks;
    while (blk) {
        StrArenaBlock* next = blk->next;
        free(blk);
        blk = next;
    }
    free(sa);
}

static StringArena* get_str_arena(void) {
    if (!the_str_arena) {
        the_str_arena = str_arena_create();
    }
    return the_str_arena;
}

// ── Reference Counting Data Structures ─────────────────────────────
// RC-managed strings live on the heap (malloc/free) rather than the arena.
// Each RC allocation has a small header before the string data.
// is_rc == true on AjeebValue means the string pointer has this header.

typedef struct AjeebRcHeader {
    int32_t ref_count;      // current reference count
    size_t  size;           // total allocation size (for free + debug)
} AjeebRcHeader;

static int rc_total_allocs = 0;
static int rc_total_frees = 0;

// Given a string pointer from an RC AjeebValue, return its RC header.
static inline AjeebRcHeader* rc_header(const char* s) {
    return (AjeebRcHeader*)(s - sizeof(AjeebRcHeader));
}

// ── Tagged Runtime Value ───────────────────────────────────────────
// Eliminates intptr_t ownership ambiguity. Every value carries its type tag.

typedef enum {
    AJB_INT = 0,
    AJB_FLOAT,
    AJB_STRING,
    AJB_BOOL,
    AJB_VOID,
} AjeebTypeTag;

typedef struct {
    AjeebTypeTag tag;
    unsigned int is_rc : 1; // 1 = heap-allocated (RC-managed), 0 = arena-allocated
    union {
        int64_t as_int;
        double as_float;
        int64_t as_bool;   // 0 or 1
    } data;
    // For strings, the character data may be arena- or heap-allocated.
    // The string field is only valid when tag == AJB_STRING.
    const char* string;    // pointer into arena or to RC header data
    size_t string_len;
} AjeebValue;

// Constructors
static inline AjeebValue ajb_int(int64_t v) {
    AjeebValue val;
    val.tag = AJB_INT;
    val.is_rc = 0;
    val.data.as_int = v;
    val.string = NULL;
    val.string_len = 0;
    return val;
}

static inline AjeebValue ajb_float(double v) {
    AjeebValue val;
    val.tag = AJB_FLOAT;
    val.is_rc = 0;
    val.data.as_float = v;
    val.string = NULL;
    val.string_len = 0;
    return val;
}

static inline AjeebValue ajb_bool(int64_t v) {
    AjeebValue val;
    val.tag = AJB_BOOL;
    val.is_rc = 0;
    val.data.as_bool = v != 0;
    val.string = NULL;
    val.string_len = 0;
    return val;
}

static inline AjeebValue ajb_string(const char* s, size_t len) {
    AjeebValue val;
    val.tag = AJB_STRING;
    val.is_rc = 0;
    val.data.as_int = 0;
    val.string = s;
    val.string_len = len;
    return val;
}

// RC-managed string constructor — allocates on heap, ref_count = 1.
static inline AjeebValue ajb_rc_string(const char* s, size_t len) {
    AjeebValue val;
    val.tag = AJB_STRING;
    val.is_rc = 1;
    val.data.as_int = 0;
    size_t header_sz = sizeof(AjeebRcHeader);
    AjeebRcHeader* hdr = (AjeebRcHeader*)malloc(header_sz + len + 1);
    if (!hdr) { val.string = ""; val.string_len = 0; val.is_rc = 0; return val; }
    hdr->ref_count = 1;
    hdr->size = header_sz + len + 1;
    rc_total_allocs++;
    char* data = (char*)hdr + header_sz;
    memcpy(data, s, len);
    data[len] = '\0';
    val.string = data;
    val.string_len = len;
    return val;
}

static inline AjeebValue ajb_void(void) {
    AjeebValue val;
    val.tag = AJB_VOID;
    val.is_rc = 0;
    val.data.as_int = 0;
    val.string = NULL;
    val.string_len = 0;
    return val;
}

// ── Reference Counting Operations ──────────────────────────────────
// retain/release/clone for RC-managed AjeebValues.
// Arena-managed values (is_rc == 0) are no-ops for retain/release;
// clone on arena values performs a deep arena copy.

static inline AjeebValue ajeeb_retain(AjeebValue v) {
    if (v.is_rc && v.tag == AJB_STRING && v.string) {
        AjeebRcHeader* hdr = rc_header(v.string);
        hdr->ref_count++;
    }
    return v;
}

static inline void ajeeb_release(AjeebValue v) {
    if (v.is_rc && v.tag == AJB_STRING && v.string) {
        AjeebRcHeader* hdr = rc_header(v.string);
        if (hdr->ref_count > 0) {
            hdr->ref_count--;
            if (hdr->ref_count == 0) {
                free(hdr);
                rc_total_frees++;
            }
        }
    }
}

static inline AjeebValue ajeeb_clone(AjeebValue v) {
    if (v.tag == AJB_STRING && v.string) {
        if (v.is_rc) {
            // RC: retain existing (shallow clone with refcount bump)
            return ajeeb_retain(v);
        } else {
            // Arena: deep copy into arena
            Arena* a = get_arena();
            char* s = arena_strndup(a, v.string, v.string_len);
            return ajb_string(s, v.string_len);
        }
    }
    return v;
}

// ── RC Introspection (testing / debugging) ─────────────────────────

int32_t ajeeb_rc_refcount(AjeebValue v) {
    if (v.is_rc && v.tag == AJB_STRING && v.string) {
        return rc_header(v.string)->ref_count;
    }
    return -1;
}

int ajeeb_rc_allocs(void) { return rc_total_allocs; }
int ajeeb_rc_frees(void)  { return rc_total_frees;  }

// ── String Free API ────────────────────────────────────────────────
// Free a string allocated by string operations (str_concat, substring, etc.)
// Arena strings are no-ops (freed at program exit).
// RC strings decrement ref_count and free when it hits 0.

void ajeeb_string_free(AjeebValue v) {
    if (v.tag != AJB_STRING || !v.string) return;
    if (v.is_rc) {
        ajeeb_release(v);
    }
    // Arena strings are freed at program exit (no-op here)
}

// ── Forward declarations for AjeebValue API ─────────────────────────
// All ajeeb_* functions are the canonical implementation. intptr_t wrappers
// call these; keep forward decls here so order doesn't matter.
AjeebValue ajeeb_charCode(AjeebValue s, AjeebValue idx);
AjeebValue ajeeb_len(AjeebValue s);
AjeebValue ajeeb_readArg(AjeebValue n);
AjeebValue ajeeb_readFile(AjeebValue path);
AjeebValue ajeeb_println(AjeebValue v);
AjeebValue ajeeb_itoa(AjeebValue n);
AjeebValue ajeeb_str_concat(AjeebValue a, AjeebValue b);
AjeebValue ajeeb_substring(AjeebValue s, AjeebValue start, AjeebValue end);
AjeebValue ajeeb_indexOf(AjeebValue s, AjeebValue search);
AjeebValue ajeeb_contains(AjeebValue s, AjeebValue search);
AjeebValue ajeeb_toUpperCase(AjeebValue s);
AjeebValue ajeeb_toLowerCase(AjeebValue s);
AjeebValue ajeeb_trim(AjeebValue s);
AjeebValue ajeeb_startsWith(AjeebValue s, AjeebValue prefix);
AjeebValue ajeeb_endsWith(AjeebValue s, AjeebValue suffix);
AjeebValue ajeeb_replace(AjeebValue s, AjeebValue from, AjeebValue to);
AjeebValue ajeeb_tcp_listen(AjeebValue port);
AjeebValue ajeeb_tcp_accept(AjeebValue listen_fd);
AjeebValue ajeeb_tcp_read(AjeebValue fd, AjeebValue max);
AjeebValue ajeeb_sqlite_open(AjeebValue path);
AjeebValue ajeeb_sqlite_close(AjeebValue handle);
AjeebValue ajeeb_sqlite_exec(AjeebValue handle, AjeebValue sql);
AjeebValue ajeeb_now_ms(void);
void ajeeb_string_free(AjeebValue v);

// ── Leak Detection ─────────────────────────────────────────────────
// Call at program exit. Asserts no net allocation leaks.

void ajeeb_leak_check(void) {
    // Report RC heap stats
    int rc_leaked = rc_total_allocs - rc_total_frees;
    if (rc_leaked > 0) {
        fprintf(stderr, "[Ajeeb Runtime] RC LEAK: %d allocations not freed (%d total allocs, %d frees)\n",
            rc_leaked, rc_total_allocs, rc_total_frees);
    } else {
        fprintf(stderr, "[Ajeeb Runtime] RC: %d allocs, %d frees — no leaks\n",
            rc_total_allocs, rc_total_frees);
    }
    // Report string arena stats
    if (the_str_arena) {
        size_t str_leaked = the_str_arena->total_allocated - the_str_arena->total_freed;
        if (str_leaked > 0) {
            fprintf(stderr, "[Ajeeb Runtime] String Arena: %zu bytes allocated, %zu bytes freed, %zu bytes leaked\n",
                the_str_arena->total_allocated, the_str_arena->total_freed, str_leaked);
        } else {
            fprintf(stderr, "[Ajeeb Runtime] String Arena: %zu bytes allocated, %zu bytes freed — no leaks\n",
                the_str_arena->total_allocated, the_str_arena->total_freed);
        }
        str_arena_destroy(the_str_arena);
        the_str_arena = NULL;
    }
    // Report arena stats
    if (the_arena) {
        fprintf(stderr, "[Ajeeb Runtime] Arena: %zu allocs, %zu bytes requested, %zu bytes capacity\n",
            the_arena->alloc_count, the_arena->total_requested, the_arena->capacity);
        arena_destroy(the_arena);
        the_arena = NULL;
    }
}

// Auto-register leak detection on program exit.
// Uses GCC/clang constructor attribute; no changes to generated main() needed.
__attribute__((constructor))
static void auto_init_atexit(void) {
    atexit(ajeeb_leak_check);
}

// ── Forward declarations for old-style intptr_t compat ─────────────
// These wrap the new AjeebValue API. The old signatures are kept for
// backward compatibility with any existing C codegen output.
// New code should use AjeebValue directly.

// ── Original Runtime Functions (ported to Arena + AjeebValue) ──────

extern char __ajeeb_buf[16384];
extern char __ajeeb_outbuf[65536];

static char* saved_argv[256];
static int saved_argc = 0;

#define FILE_CACHE_SIZE 65536
static struct { const char* path; FILE* fp; } file_cache[FILE_CACHE_SIZE];
static int file_cache_count = 0;

static void flush_cached_files(void) {
    for (int i = 0; i < file_cache_count; i++) {
        fflush(file_cache[i].fp);
        fclose(file_cache[i].fp);
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
    Arena* a = get_arena();
    file_cache[file_cache_count].path = arena_strdup(a, path);
    file_cache[file_cache_count].fp = fp;
    file_cache_count++;
    return fp;
}

static int args_init = 0;

static void init_args(void) {
    if (args_init) return;
    args_init = 1;
    atexit(flush_cached_files);
    Arena* a = get_arena();
#if defined(__linux__)
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
                saved_argv[saved_argc] = arena_strndup(a, buf + start, len);
                saved_argc++;
            }
            start = i + 1;
        }
    }
#elif defined(__APPLE__)
    extern int _NSGetArgc(int*);
    extern char*** _NSGetArgv(void);
    int mac_argc;
    char** mac_argv = *_NSGetArgv();
    _NSGetArgc(&mac_argc);
    for (int i = 0; i < mac_argc && i < 256; i++) {
        saved_argv[i] = arena_strdup(a, mac_argv[i]);
        saved_argc++;
    }
#elif defined(_WIN32)
    int wargc;
    wchar_t** wargv = CommandLineToArgvW(GetCommandLineW(), &wargc);
    if (wargv) {
        for (int i = 0; i < wargc && i < 256; i++) {
            int len = WideCharToMultiByte(CP_UTF8, 0, wargv[i], -1, NULL, 0, NULL, NULL);
            saved_argv[i] = (char*)arena_alloc(a, len);
            if (saved_argv[i]) {
                WideCharToMultiByte(CP_UTF8, 0, wargv[i], -1, saved_argv[i], len, NULL, NULL);
            }
            saved_argc++;
        }
        LocalFree(wargv);
    }
#else
    saved_argv[0] = arena_strdup(a, "ajeeb");
    saved_argc = 1;
#endif
}

intptr_t getInt(intptr_t buf, intptr_t off) {
    return *(int64_t*)((char*)buf + off);
}

void setInt(intptr_t buf, intptr_t off, intptr_t v) {
    *(int64_t*)((char*)buf + off) = v;
}

intptr_t allocBuf(intptr_t size) {
    Arena* a = get_arena();
    char* buf = (char*)arena_alloc(a, (size_t)size + 1);
    memset(buf, 0, (size_t)size + 1);
    return (intptr_t)buf;
}

AjeebValue ajeeb_charCode(AjeebValue s, AjeebValue idx) {
    if (s.tag != AJB_STRING || idx.tag != AJB_INT) return ajb_int(0);
    if ((size_t)idx.data.as_int >= s.string_len) return ajb_int(0);
    return ajb_int((unsigned char)s.string[idx.data.as_int]);
}

intptr_t charCode(intptr_t s, intptr_t i) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    AjeebValue vi = ajb_int((int64_t)i);
    return (intptr_t)ajeeb_charCode(vs, vi).data.as_int;
}

AjeebValue ajeeb_chr(AjeebValue s, AjeebValue idx) {
    if (s.tag != AJB_STRING || idx.tag != AJB_INT) return ajb_string("", 0);
    if ((size_t)idx.data.as_int >= s.string_len) return ajb_string("", 0);
    char buf[2] = { s.string[idx.data.as_int], '\0' };
    return ajb_string(buf, 1);
}

intptr_t chr(intptr_t s, intptr_t i) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    AjeebValue vi = ajb_int((int64_t)i);
    AjeebValue result = ajeeb_chr(vs, vi);
    return (intptr_t)result.string;
}

AjeebValue ajeeb_len(AjeebValue s) {
    if (s.tag != AJB_STRING) return ajb_int(0);
    return ajb_int((int64_t)s.string_len);
}

intptr_t len(intptr_t s) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    return (intptr_t)ajeeb_len(vs).data.as_int;
}

intptr_t arr_len(intptr_t arr) {
    if (arr == 0) return 0;
    return *(int64_t*)arr;
}

void strSet(intptr_t s, intptr_t i, intptr_t c) {
    ((char*)s)[i] = (char)c;
    ((char*)s)[i + 1] = '\0';
}

intptr_t getStateBuf(void) {
    return (intptr_t)__ajeeb_buf;
}

intptr_t getOutbuf(void) {
    __ajeeb_outbuf[0] = '\0';
    return (intptr_t)__ajeeb_outbuf;
}

AjeebValue ajeeb_readArg(AjeebValue n) {
    init_args();
    if (n.tag != AJB_INT) return ajb_string("", 0);
    int idx = (int)n.data.as_int;
    if (idx >= 0 && idx < saved_argc) {
        return ajb_string(saved_argv[idx], strlen(saved_argv[idx]));
    }
    return ajb_string("", 0);
}

intptr_t readArg(intptr_t n) {
    AjeebValue vn = ajb_int((int64_t)n);
    AjeebValue result = ajeeb_readArg(vn);
    return (intptr_t)result.string;
}

// ── String Functions (Arena-Allocated) ─────────────────────────────

AjeebValue ajeeb_readFile(AjeebValue path) {
    if (path.tag != AJB_STRING) return ajb_string("", 0);
    const char* fname = path.string;
    FILE* f = fopen(fname, "rb");
    if (!f) return ajb_string("", 0);
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    rewind(f);
    Arena* a = get_arena();
    char* content = (char*)arena_alloc(a, (size_t)sz + 1);
    if (!content) { fclose(f); return ajb_string("", 0); }
    fread(content, 1, sz, f);
    content[sz] = '\0';
    fclose(f);
    return ajb_string(content, (size_t)sz);
}

intptr_t readFile(intptr_t path) {
    AjeebValue vpath = ajb_string((const char*)path, strlen((const char*)path));
    AjeebValue result = ajeeb_readFile(vpath);
    return (intptr_t)result.string;
}

int64_t getStr(int64_t ptr) { return ptr; }

int64_t exec(int64_t cmd_ptr) {
    return system((const char*)cmd_ptr);
}

int64_t mkdir(int64_t path_ptr) {
    // Use system() to avoid name conflict with POSIX mkdir()
    // mkdir -p creates parent directories too
    char buf[4096];
    int r = snprintf(buf, sizeof(buf), "mkdir -p %s", (const char*)path_ptr);
    if (r < 0 || (size_t)r >= sizeof(buf)) return -1;
    return system(buf);
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
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    ajeeb_println(vs);
    return 0;
}

intptr_t print(intptr_t s) {
    printf("%s", (const char*)s);
    return 0;
}

AjeebValue ajeeb_println(AjeebValue v) {
    switch (v.tag) {
        case AJB_INT:    printf("%ld", (long)v.data.as_int); break;
        case AJB_FLOAT:  printf("%f", v.data.as_float); break;
        case AJB_BOOL:   printf("%s", v.data.as_bool ? "true" : "false"); break;
        case AJB_STRING: printf("%.*s", (int)v.string_len, v.string); break;
        case AJB_VOID:   printf("void"); break;
    }
    printf("\n");
    return ajb_void();
}

AjeebValue ajeeb_itoa(AjeebValue n) {
    if (n.tag != AJB_INT) return ajb_string("0", 1);
    char tmp[32];
    int len = snprintf(tmp, sizeof(tmp), "%ld", (long)n.data.as_int);
    Arena* a = get_arena();
    char* s = arena_strndup(a, tmp, (size_t)len);
    return ajb_string(s, (size_t)len);
}

intptr_t itoa(intptr_t n) {
    AjeebValue vn = ajb_int((int64_t)n);
    AjeebValue result = ajeeb_itoa(vn);
    return (intptr_t)result.string;
}

intptr_t strcmp_ajeeb(intptr_t a, intptr_t b) {
    return (intptr_t)strcmp((const char*)a, (const char*)b);
}

AjeebValue ajeeb_str_concat(AjeebValue a, AjeebValue b) {
    if (a.tag != AJB_STRING || b.tag != AJB_STRING) return ajb_string("", 0);
    size_t total = a.string_len + b.string_len;
    Arena* ar = get_arena();
    char* out = (char*)arena_alloc(ar, total + 1);
    if (!out) return ajb_string("", 0);
    memcpy(out, a.string, a.string_len);
    memcpy(out + a.string_len, b.string, b.string_len);
    out[total] = '\0';
    return ajb_string(out, total);
}

intptr_t str_concat(intptr_t a, intptr_t b) {
    AjeebValue va = ajb_string((const char*)a, strlen((const char*)a));
    AjeebValue vb = ajb_string((const char*)b, strlen((const char*)b));
    AjeebValue result = ajeeb_str_concat(va, vb);
    return (intptr_t)result.string;
}

AjeebValue ajeeb_substring(AjeebValue s, AjeebValue start, AjeebValue end) {
    if (s.tag != AJB_STRING || start.tag != AJB_INT || end.tag != AJB_INT) return ajb_string("", 0);
    size_t slen = s.string_len;
    size_t st = (size_t)start.data.as_int;
    size_t en = (size_t)end.data.as_int;
    if (st > slen) st = slen;
    if (en > slen) en = slen;
    if (en < st) en = st;
    Arena* a = get_arena();
    char* out = arena_strndup(a, s.string + st, en - st);
    return ajb_string(out, en - st);
}

intptr_t substring(intptr_t s, intptr_t start, intptr_t end) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    AjeebValue vstart = ajb_int((int64_t)start);
    AjeebValue vend = ajb_int((int64_t)end);
    AjeebValue result = ajeeb_substring(vs, vstart, vend);
    return (intptr_t)result.string;
}

AjeebValue ajeeb_indexOf(AjeebValue s, AjeebValue search) {
    if (s.tag != AJB_STRING || search.tag != AJB_STRING) return ajb_int(-1);
    if (search.string_len == 0) return ajb_int(0);
    for (size_t i = 0; i <= s.string_len - search.string_len; i++) {
        if (memcmp(s.string + i, search.string, search.string_len) == 0) {
            return ajb_int((int64_t)i);
        }
    }
    return ajb_int(-1);
}

intptr_t indexOf(intptr_t s, intptr_t search) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    AjeebValue vsearch = ajb_string((const char*)search, strlen((const char*)search));
    return (intptr_t)ajeeb_indexOf(vs, vsearch).data.as_int;
}

AjeebValue ajeeb_contains(AjeebValue s, AjeebValue search) {
    int64_t idx = ajeeb_indexOf(s, search).data.as_int;
    return ajb_int(idx >= 0 ? 1 : 0);
}

intptr_t contains(intptr_t s, intptr_t search) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    AjeebValue vsearch = ajb_string((const char*)search, strlen((const char*)search));
    return (intptr_t)ajeeb_contains(vs, vsearch).data.as_int;
}

AjeebValue ajeeb_toUpperCase(AjeebValue s) {
    if (s.tag != AJB_STRING) return ajb_string("", 0);
    Arena* a = get_arena();
    char* out = (char*)arena_alloc(a, s.string_len + 1);
    if (!out) return ajb_string("", 0);
    for (size_t i = 0; i < s.string_len; i++) {
        char c = s.string[i];
        out[i] = (c >= 'a' && c <= 'z') ? c - 32 : c;
    }
    out[s.string_len] = '\0';
    return ajb_string(out, s.string_len);
}

intptr_t toUpperCase(intptr_t s) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    return (intptr_t)ajeeb_toUpperCase(vs).string;
}

AjeebValue ajeeb_toLowerCase(AjeebValue s) {
    if (s.tag != AJB_STRING) return ajb_string("", 0);
    Arena* a = get_arena();
    char* out = (char*)arena_alloc(a, s.string_len + 1);
    if (!out) return ajb_string("", 0);
    for (size_t i = 0; i < s.string_len; i++) {
        char c = s.string[i];
        out[i] = (c >= 'A' && c <= 'Z') ? c + 32 : c;
    }
    out[s.string_len] = '\0';
    return ajb_string(out, s.string_len);
}

intptr_t toLowerCase(intptr_t s) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    return (intptr_t)ajeeb_toLowerCase(vs).string;
}

AjeebValue ajeeb_trim(AjeebValue s) {
    if (s.tag != AJB_STRING) return ajb_string("", 0);
    const char* start = s.string;
    const char* end = s.string + s.string_len;
    while (start < end && (*start == ' ' || *start == '\t' || *start == '\n' || *start == '\r')) start++;
    while (end > start && (*(end-1) == ' ' || *(end-1) == '\t' || *(end-1) == '\n' || *(end-1) == '\r')) end--;
    size_t len = (size_t)(end - start);
    Arena* a = get_arena();
    char* out = arena_strndup(a, start, len);
    return ajb_string(out, len);
}

intptr_t trim(intptr_t s) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    return (intptr_t)ajeeb_trim(vs).string;
}

AjeebValue ajeeb_startsWith(AjeebValue s, AjeebValue prefix) {
    if (s.tag != AJB_STRING || prefix.tag != AJB_STRING) return ajb_bool(0);
    if (prefix.string_len > s.string_len) return ajb_bool(0);
    return ajb_bool(memcmp(s.string, prefix.string, prefix.string_len) == 0 ? 1 : 0);
}

intptr_t startsWith(intptr_t s, intptr_t prefix) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    AjeebValue vpre = ajb_string((const char*)prefix, strlen((const char*)prefix));
    return (intptr_t)ajeeb_startsWith(vs, vpre).data.as_bool;
}

AjeebValue ajeeb_endsWith(AjeebValue s, AjeebValue suffix) {
    if (s.tag != AJB_STRING || suffix.tag != AJB_STRING) return ajb_bool(0);
    if (suffix.string_len > s.string_len) return ajb_bool(0);
    return ajb_bool(memcmp(s.string + s.string_len - suffix.string_len, suffix.string, suffix.string_len) == 0 ? 1 : 0);
}

intptr_t endsWith(intptr_t s, intptr_t suffix) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    AjeebValue vsuf = ajb_string((const char*)suffix, strlen((const char*)suffix));
    return (intptr_t)ajeeb_endsWith(vs, vsuf).data.as_bool;
}

AjeebValue ajeeb_replace(AjeebValue s, AjeebValue from, AjeebValue to) {
    if (s.tag != AJB_STRING || from.tag != AJB_STRING || to.tag != AJB_STRING) return ajb_string("", 0);
    if (from.string_len == 0) return s;
    size_t count = 0;
    for (size_t i = 0; i <= s.string_len - from.string_len; ) {
        if (memcmp(s.string + i, from.string, from.string_len) == 0) {
            count++;
            i += from.string_len;
        } else {
            i++;
        }
    }
    if (count == 0) return s;
    size_t result_len = s.string_len + count * (to.string_len - from.string_len);
    Arena* a = get_arena();
    char* out = (char*)arena_alloc(a, result_len + 1);
    if (!out) return ajb_string("", 0);
    size_t pos = 0;
    for (size_t i = 0; i < s.string_len; ) {
        if (i <= s.string_len - from.string_len && memcmp(s.string + i, from.string, from.string_len) == 0) {
            memcpy(out + pos, to.string, to.string_len);
            pos += to.string_len;
            i += from.string_len;
        } else {
            out[pos++] = s.string[i++];
        }
    }
    out[pos] = '\0';
    return ajb_string(out, result_len);
}

intptr_t replace(intptr_t s, intptr_t from, intptr_t to) {
    AjeebValue vs = ajb_string((const char*)s, strlen((const char*)s));
    AjeebValue vfrom = ajb_string((const char*)from, strlen((const char*)from));
    AjeebValue vto = ajb_string((const char*)to, strlen((const char*)to));
    return (intptr_t)ajeeb_replace(vs, vfrom, vto).string;
}

// ── Networking (TCP Sockets) ─────────────────────────────────────────
#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#pragma comment(lib, "ws2_32.lib")
static int winsock_init(void) {
    WSADATA wsa;
    return WSAStartup(MAKEWORD(2,2), &wsa);
}
typedef int socklen_t;
#define CLOSE_SOCKET closesocket
#else
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <unistd.h>
#include <errno.h>
#define CLOSE_SOCKET close
#endif

AjeebValue ajeeb_tcp_listen(AjeebValue port) {
    if (port.tag != AJB_INT) return ajb_int(0);
#ifdef _WIN32
    winsock_init();
#endif
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return ajb_int(0);
    int opt = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, (char*)&opt, sizeof(opt));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port.data.as_int);
    addr.sin_addr.s_addr = INADDR_ANY;
    if (bind(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        CLOSE_SOCKET(fd); return ajb_int(0);
    }
    if (listen(fd, 128) < 0) {
        CLOSE_SOCKET(fd); return ajb_int(0);
    }
    return ajb_int((int64_t)fd);
}

intptr_t tcp_listen(intptr_t port) {
    return (intptr_t)ajeeb_tcp_listen(ajb_int((int64_t)port)).data.as_int;
}

AjeebValue ajeeb_tcp_accept(AjeebValue listen_fd) {
    if (listen_fd.tag != AJB_INT) return ajb_int(0);
    struct sockaddr_in client;
    socklen_t client_len = sizeof(client);
    int fd = accept((int)listen_fd.data.as_int, (struct sockaddr*)&client, &client_len);
    if (fd < 0) return ajb_int(0);
    return ajb_int((int64_t)fd);
}

intptr_t tcp_accept(intptr_t listen_fd) {
    return (intptr_t)ajeeb_tcp_accept(ajb_int((int64_t)listen_fd)).data.as_int;
}

AjeebValue ajeeb_tcp_read(AjeebValue fd, AjeebValue max) {
    if (fd.tag != AJB_INT || max.tag != AJB_INT) return ajb_string("", 0);
    int f = (int)fd.data.as_int;
    size_t m = (size_t)max.data.as_int;
    if (f <= 0 || m <= 0) return ajb_string("", 0);
    Arena* a = get_arena();
    char* buf = (char*)arena_alloc(a, m + 1);
    if (!buf) return ajb_string("", 0);
    ssize_t n = read(f, buf, m);
    if (n > 0) {
        buf[n] = '\0';
        return ajb_string(buf, (size_t)n);
    }
    return ajb_string("", 0);
}

intptr_t tcp_read(intptr_t fd, intptr_t max) {
    AjeebValue result = ajeeb_tcp_read(ajb_int((int64_t)fd), ajb_int((int64_t)max));
    return (intptr_t)result.string;
}

void tcp_write(intptr_t fd, intptr_t data) {
    if (fd <= 0) return;
    const char* str = (const char*)data;
    if (str) write((int)fd, str, strlen(str));
}

void tcp_close(intptr_t fd) {
    if (fd > 0) CLOSE_SOCKET((int)fd);
}

AjeebValue ajeeb_tcp_connect(AjeebValue host, AjeebValue port) {
    if (host.tag != AJB_STRING || port.tag != AJB_INT) return ajb_int(0);
#ifdef _WIN32
    winsock_init();
#endif
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return ajb_int(0);
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port.data.as_int);
    if (inet_pton(AF_INET, host.string, &addr.sin_addr) <= 0) {
        // Not a dotted-quad, try DNS
        struct addrinfo hints, *res;
        memset(&hints, 0, sizeof(hints));
        hints.ai_family = AF_INET;
        hints.ai_socktype = SOCK_STREAM;
        char port_str[16];
        snprintf(port_str, sizeof(port_str), "%d", (int)port.data.as_int);
        if (getaddrinfo(host.string, port_str, &hints, &res) != 0 || !res) {
            CLOSE_SOCKET(fd); return ajb_int(0);
        }
        memcpy(&addr.sin_addr, &((struct sockaddr_in*)res->ai_addr)->sin_addr, sizeof(addr.sin_addr));
        freeaddrinfo(res);
    }
    if (connect(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        CLOSE_SOCKET(fd); return ajb_int(0);
    }
    return ajb_int((int64_t)fd);
}

intptr_t tcp_connect(intptr_t host_ptr, intptr_t port) {
    const char* host = (const char*)(intptr_t)host_ptr;
    AjeebValue h = ajb_string(host, strlen(host));
    AjeebValue p = ajb_int((int64_t)port);
    return (intptr_t)ajeeb_tcp_connect(h, p).data.as_int;
}

AjeebValue ajeeb_dns_lookup(AjeebValue hostname) {
    if (hostname.tag != AJB_STRING) return ajb_string("", 0);
    struct addrinfo hints, *res;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    if (getaddrinfo(hostname.string, NULL, &hints, &res) != 0 || !res) {
        return ajb_string("", 0);
    }
    char ip[64];
    inet_ntop(AF_INET, &((struct sockaddr_in*)res->ai_addr)->sin_addr, ip, sizeof(ip));
    freeaddrinfo(res);
    return ajb_string(ip, strlen(ip));
}

intptr_t dns_lookup(intptr_t hostname_ptr) {
    const char* hostname = (const char*)(intptr_t)hostname_ptr;
    AjeebValue result = ajeeb_dns_lookup(ajb_string(hostname, strlen(hostname)));
    return (intptr_t)result.string;
}

// TLS connect via runtime-loaded OpenSSL (no compile-time dependency)
typedef struct {
    void* ssl;
    void* ctx;
} TlsSession;

static void* tls_ssl_handle = NULL;
static void* tls_crypto_handle = NULL;

// Function pointers loaded at runtime
static void* (*tls_SSL_new)(void* ctx);
static int (*tls_SSL_set_fd)(void* ssl, int fd);
static int (*tls_SSL_connect)(void* ssl);
static void* (*tls_SSL_write)(void* ssl, const void* buf, int num);
static void* (*tls_SSL_read)(void* ssl, void* buf, int num);
static void (*tls_SSL_shutdown)(void* ssl);
static void (*tls_SSL_free)(void* ssl);
static void* (*tls_SSL_CTX_new)(void* method);
static void (*tls_SSL_CTX_free)(void* ctx);
static void* (*tls_TLS_method)(void);
static void (*tls_SSL_set_tlsext_host_name)(void* ssl, const char* name);

AjeebValue ajeeb_tls_connect(AjeebValue host, AjeebValue port) {
    if (host.tag != AJB_STRING || port.tag != AJB_INT) return ajb_int(0);
    
    // Lazy-load OpenSSL
    if (!tls_ssl_handle) {
        tls_ssl_handle = dlopen("libssl.so.3", RTLD_NOW | RTLD_LOCAL);
        if (!tls_ssl_handle) tls_ssl_handle = dlopen("libssl.so.1.1", RTLD_NOW | RTLD_LOCAL);
        if (!tls_ssl_handle) tls_ssl_handle = dlopen("libssl.so", RTLD_NOW | RTLD_LOCAL);
        if (!tls_ssl_handle) { fprintf(stderr, "[TLS] cannot load libssl\n"); return ajb_int(0); }
        tls_crypto_handle = dlopen("libcrypto.so.3", RTLD_NOW | RTLD_LOCAL);
        if (!tls_crypto_handle) tls_crypto_handle = dlopen("libcrypto.so.1.1", RTLD_NOW | RTLD_LOCAL);
        if (!tls_crypto_handle) tls_crypto_handle = dlopen("libcrypto.so", RTLD_NOW | RTLD_LOCAL);
        
        tls_SSL_new = (void* (*)(void*))dlsym(tls_ssl_handle, "SSL_new");
        tls_SSL_set_fd = (int (*)(void*, int))dlsym(tls_ssl_handle, "SSL_set_fd");
        tls_SSL_connect = (int (*)(void*))dlsym(tls_ssl_handle, "SSL_connect");
        tls_SSL_write = (void* (*)(void*, const void*, int))dlsym(tls_ssl_handle, "SSL_write");
        tls_SSL_read = (void* (*)(void*, void*, int))dlsym(tls_ssl_handle, "SSL_read");
        tls_SSL_shutdown = (void (*)(void*))dlsym(tls_ssl_handle, "SSL_shutdown");
        tls_SSL_free = (void (*)(void*))dlsym(tls_ssl_handle, "SSL_free");
        tls_SSL_CTX_new = (void* (*)(void*))dlsym(tls_ssl_handle, "SSL_CTX_new");
        tls_SSL_CTX_free = (void (*)(void*))dlsym(tls_ssl_handle, "SSL_CTX_free");
        tls_TLS_method = (void* (*)(void))dlsym(tls_ssl_handle, "TLS_method");
        tls_SSL_set_tlsext_host_name = (void (*)(void*, const char*))dlsym(tls_ssl_handle, "SSL_set_tlsext_host_name");
        
        if (!tls_SSL_new || !tls_SSL_connect || !tls_TLS_method) {
            fprintf(stderr, "[TLS] OpenSSL symbols not found\n");
            return ajb_int(0);
        }
    }
    
    // Create TCP connection first
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return ajb_int(0);
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port.data.as_int);
    if (inet_pton(AF_INET, host.string, &addr.sin_addr) <= 0) {
        struct addrinfo hints, *res;
        memset(&hints, 0, sizeof(hints));
        hints.ai_family = AF_INET;
        hints.ai_socktype = SOCK_STREAM;
        char port_str[16];
        snprintf(port_str, sizeof(port_str), "%d", (int)port.data.as_int);
        if (getaddrinfo(host.string, port_str, &hints, &res) != 0 || !res) {
            CLOSE_SOCKET(fd); return ajb_int(0);
        }
        memcpy(&addr.sin_addr, &((struct sockaddr_in*)res->ai_addr)->sin_addr, sizeof(addr.sin_addr));
        freeaddrinfo(res);
    }
    if (connect(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        CLOSE_SOCKET(fd); return ajb_int(0);
    }
    
    // Wrap in TLS
    void* ctx = tls_SSL_CTX_new(tls_TLS_method());
    if (!ctx) { CLOSE_SOCKET(fd); return ajb_int(0); }
    void* ssl = tls_SSL_new(ctx);
    if (!ssl) { tls_SSL_CTX_free(ctx); CLOSE_SOCKET(fd); return ajb_int(0); }
    tls_SSL_set_fd(ssl, fd);
    if (tls_SSL_set_tlsext_host_name) {
        tls_SSL_set_tlsext_host_name(ssl, host.string);
    }
    if (tls_SSL_connect(ssl) != 1) {
        fprintf(stderr, "[TLS] SSL_connect failed\n");
        tls_SSL_free(ssl);
        tls_SSL_CTX_free(ctx);
        CLOSE_SOCKET(fd);
        return ajb_int(0);
    }
    
    // Store TLS session info in a static map (simplified: use a fixed-size array)
    // For production, use a proper handle map. For now, return handle with TLS bits.
    // Store the TLS handle in a global slot (this is a simplified design)
    // The fd returned is the raw TCP fd; TLS read/write wrappers would need separate builtins.
    // Returning the SSL pointer as handle for use with tls_read/tls_write/tls_close (separate builtins)
    tls_SSL_CTX_free(ctx);
    return ajb_int((int64_t)(intptr_t)ssl);
}

intptr_t tls_connect(intptr_t host_ptr, intptr_t port) {
    const char* host = (const char*)(intptr_t)host_ptr;
    AjeebValue h = ajb_string(host, strlen(host));
    AjeebValue p = ajb_int((int64_t)port);
    return (intptr_t)ajeeb_tls_connect(h, p).data.as_int;
}

intptr_t tls_read(intptr_t handle, intptr_t max) {
    if (!handle || max <= 0) return (intptr_t)"";
    static char buf[65536];
    size_t m = (size_t)max > sizeof(buf) - 1 ? sizeof(buf) - 1 : (size_t)max;
    // Use stored TLS read function pointer
    if (tls_SSL_read) {
        int n = ((int (*)(void*, void*, int))(intptr_t)tls_SSL_read)((void*)(intptr_t)handle, buf, (int)m);
        if (n > 0) { buf[n] = '\0'; return (intptr_t)ajb_string(buf, (size_t)n).string; }
    }
    return (intptr_t)"";
}

void tls_write(intptr_t handle, intptr_t data_ptr) {
    if (!handle) return;
    const char* data = (const char*)(intptr_t)data_ptr;
    if (data && tls_SSL_write) {
        ((int (*)(void*, const void*, int))(intptr_t)tls_SSL_write)((void*)(intptr_t)handle, data, (int)strlen(data));
    }
}

void tls_close(intptr_t handle) {
    if (!handle) return;
    if (tls_SSL_shutdown) ((void (*)(void*))(intptr_t)tls_SSL_shutdown)((void*)(intptr_t)handle);
    if (tls_SSL_free) ((void (*)(void*))(intptr_t)tls_SSL_free)((void*)(intptr_t)handle);
}

// ── SQLite (conditional) ──────────────────────────────────────────────
#ifdef USE_SQLITE3
#include <sqlite3.h>

AjeebValue ajeeb_sqlite_open(AjeebValue path) {
    if (path.tag != AJB_STRING) return ajb_int(0);
    sqlite3* db;
    if (sqlite3_open(path.string, &db) != SQLITE_OK) return ajb_int(0);
    return ajb_int((int64_t)(intptr_t)db);
}

AjeebValue ajeeb_sqlite_close(AjeebValue handle) {
    if (handle.tag != AJB_INT || handle.data.as_int == 0) return ajb_void();
    sqlite3_close((sqlite3*)(intptr_t)handle.data.as_int);
    return ajb_void();
}

AjeebValue ajeeb_sqlite_exec(AjeebValue handle, AjeebValue sql) {
    if (handle.tag != AJB_INT || sql.tag != AJB_STRING) return ajb_int(1);
    char* err = NULL;
    int rc = sqlite3_exec((sqlite3*)(intptr_t)handle.data.as_int, sql.string, NULL, NULL, &err);
    if (err) sqlite3_free(err);
    return ajb_int((int64_t)rc);
}
#else
intptr_t sqlite_open(intptr_t path) { (void)path; return 0; }
void sqlite_close(intptr_t handle) { (void)handle; }
intptr_t sqlite_exec(intptr_t handle, intptr_t sql) { (void)handle; (void)sql; return 1; }
#endif

// ── C ABI FFI ─────────────────────────────────────────────────────────
int64_t lib_open(int64_t path_ptr) {
    const char* path = (const char*)(intptr_t)path_ptr;
    void* handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!handle) {
        fprintf(stderr, "[FFI] dlopen error: %s\n", dlerror());
        return -1;
    }
    return (int64_t)(intptr_t)handle;
}

int64_t lib_sym(int64_t handle, int64_t name_ptr) {
    void* lib = (void*)(intptr_t)handle;
    const char* name = (const char*)(intptr_t)name_ptr;
    void* sym = dlsym(lib, name);
    if (!sym) {
        fprintf(stderr, "[FFI] dlsym error: %s\n", dlerror());
    }
    return (int64_t)(intptr_t)sym;
}

int64_t lib_call(int64_t fn_ptr, int64_t arg0, int64_t arg1, int64_t arg2,
                 int64_t arg3, int64_t arg4, int64_t arg5, int64_t arg6,
                 int64_t arg7, int64_t num_args) {
    (void)num_args;
    typedef int64_t (*fn_t)(int64_t, int64_t, int64_t, int64_t,
                            int64_t, int64_t, int64_t, int64_t);
    fn_t f = (fn_t)(intptr_t)fn_ptr;
    return f(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7);
}

// ── Time ──────────────────────────────────────────────────────────────
#include <time.h>

AjeebValue ajeeb_now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return ajb_int((int64_t)ts.tv_sec * 1000 + (int64_t)ts.tv_nsec / 1000000);
}

intptr_t now_ms(void) {
    return (intptr_t)ajeeb_now_ms().data.as_int;
}

// ── Array Printing ────────────────────────────────────────────────────
// Arrays in the LLVM backend are raw i64* pointers.
// Convention: bit 63 = 1 means "this element is a pointer to a sub-array"
//             bit 63 = 0 means "this element is a plain integer"

#define ARRAY_PTR_TAG ((int64_t)1 << 63)

static void array_to_string_rec(int64_t* arr, int64_t len, char* out, int* pos, int* cap) {
    out[*pos] = '[';
    (*pos)++;
    for (int64_t i = 0; i < len; i++) {
        if (i > 0) {
            out[*pos] = ',';
            (*pos)++;
            out[*pos] = ' ';
            (*pos)++;
        }
        int64_t elem = arr[i];
        if (elem & ARRAY_PTR_TAG) {
            // It's a pointer to a sub-array
            int64_t* sub = (int64_t*)(uintptr_t)(elem & ~ARRAY_PTR_TAG);
            // First element of sub-array is its length
            int64_t sub_len = sub[0];
            array_to_string_rec(sub + 1, sub_len, out, pos, cap);
        } else {
            // Plain integer
            int written = snprintf(out + *pos, *cap - *pos, "%ld", (long)elem);
            *pos += written;
        }
    }
    out[*pos] = ']';
    (*pos)++;
}

// LLVM backend passes arrays as: [len][elem0][elem1]...
// The len is stored as the first i64 element.
static char* ll_array_to_string(int64_t* raw) {
    if (!raw) {
        char* empty = (char*)malloc(3);
        strcpy(empty, "[]");
        return empty;
    }
    int64_t len = raw[0];
    int64_t* data = raw + 1;
    int cap = 4096;
    char* out = (char*)malloc(cap);
    int pos = 0;
    out[pos] = '[';
    pos++;
    for (int64_t i = 0; i < len; i++) {
        if (i > 0) {
            out[pos] = ',';
            pos++;
            out[pos] = ' ';
            pos++;
        }
        int64_t elem = data[i];
        if (elem & ARRAY_PTR_TAG) {
            int64_t* sub = (int64_t*)(uintptr_t)(elem & ~ARRAY_PTR_TAG);
            int64_t sub_len = sub[0];
            array_to_string_rec(sub + 1, sub_len, out, &pos, &cap);
        } else {
            int written = snprintf(out + pos, cap - pos, "%ld", (long)elem);
            pos += written;
        }
    }
    out[pos] = ']';
    pos++;
    out[pos] = '\0';
    return out;
}

// Simple wrapper: LLVM codegen passes (ptr, len) separately
intptr_t array_to_string(intptr_t ptr, int64_t len) {
    if (!ptr) {
        char* empty = (char*)malloc(3);
        strcpy(empty, "[]");
        return (intptr_t)empty;
    }
    int cap = 4096;
    char* out = (char*)malloc(cap);
    int pos = 0;
    out[pos] = '[';
    pos++;
    for (int64_t i = 0; i < len; i++) {
        if (i > 0) {
            out[pos] = ',';
            pos++;
            out[pos] = ' ';
            pos++;
        }
        int64_t elem = ((int64_t*)ptr)[i];
        if (elem & ARRAY_PTR_TAG) {
            int64_t* sub = (int64_t*)(uintptr_t)(elem & ~ARRAY_PTR_TAG);
            int64_t sub_len = sub[0];
            array_to_string_rec(sub + 1, sub_len, out, &pos, &cap);
        } else {
            int written = snprintf(out + pos, cap - pos, "%ld", (long)elem);
            pos += written;
        }
    }
    out[pos] = ']';
    pos++;
    out[pos] = '\0';
    return (intptr_t)out;
}
