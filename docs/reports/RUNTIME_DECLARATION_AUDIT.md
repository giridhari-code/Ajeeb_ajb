# RUNTIME_DECLARATION_AUDIT.md

**Date:** 2026-06-22
**Claim Under Audit:** "Bootstrap is blocked by 16 missing runtime function declarations in compiler.ajb"
**Verdict:** **INCORRECT — the claim overstates the gap.**

---

## 1. Evidence: What compiler.ajb Emits as C Header

**compiler.ajb lines 76-102** — the self-hosted compiler's own C header (used when compiling `.ajb` files via the interpreter):

```c
intptr_t getInt(intptr_t buf, intptr_t off);         // 76
void setInt(intptr_t buf, intptr_t off, intptr_t v); // 77
intptr_t charCode(intptr_t s, intptr_t i);           // 78
intptr_t len(intptr_t s);                            // 79
intptr_t readArg(intptr_t n);                        // 80
intptr_t readFile(intptr_t path);                     // 81
void writeFile(intptr_t path, intptr_t content);     // 82
void writeAppend(intptr_t path, intptr_t content);   // 83
void writeByte(intptr_t path, intptr_t byte);        // 84
intptr_t getOutbuf();                                // 85
void strSet(intptr_t s, intptr_t i, intptr_t c);    // 86
intptr_t itoa(intptr_t n);                           // 87
intptr_t println(intptr_t s);                        // 88
intptr_t getStateBuf();                              // 89
intptr_t strcmp_ajeeb(intptr_t a, intptr_t b);       // 90
intptr_t str_concat(intptr_t a, intptr_t b);         // 91
intptr_t substring(intptr_t s, intptr_t start, intptr_t end); // 92
intptr_t indexOf(intptr_t s, intptr_t search);       // 93  ⚠️ WRONG: runtime has 3 args
intptr_t contains(intptr_t s, intptr_t search);      // 94
intptr_t toUpperCase(intptr_t s);                    // 95
intptr_t toLowerCase(intptr_t s);                    // 96
intptr_t trim(intptr_t s);                           // 97
intptr_t replace(intptr_t s, intptr_t from, intptr_t to); // 98
intptr_t startsWith(intptr_t s, intptr_t prefix);    // 99
intptr_t endsWith(intptr_t s, intptr_t suffix);      // 100
intptr_t parseExpr(intptr_t src, intptr_t buf, intptr_t out); // 101 ⚠️ NOT a runtime function
intptr_t parseStmt(intptr_t src, intptr_t buf, intptr_t out); // 102 ⚠️ NOT a runtime function
```

**Total: 29 declarations** (27 runtime + 2 forward decls of compiler-defined functions)

---

## 2. Evidence: What main.ajb emitCHeader() Emits

**main.ajb lines 1280-1311** — the NEW C codegen's header:

```c
intptr_t getStateBuf(void);                          // 1280
intptr_t getOutbuf(void);                            // 1281
intptr_t getInt(intptr_t, intptr_t);                 // 1282
void setInt(intptr_t, intptr_t, intptr_t);           // 1283
intptr_t len(intptr_t);                              // 1284
void strSet(intptr_t, intptr_t, intptr_t);           // 1285
intptr_t charCode(intptr_t, intptr_t);               // 1286
intptr_t str_concat(intptr_t, intptr_t);             // 1287
intptr_t substring(intptr_t, intptr_t, intptr_t);    // 1288
intptr_t indexOf(intptr_t, intptr_t);                // 1289 ⚠️ WRONG: runtime has 3 args
intptr_t contains(intptr_t, intptr_t);               // 1290
intptr_t itoa(intptr_t);                             // 1291
intptr_t println(intptr_t);                          // 1292
intptr_t readArg(intptr_t);                          // 1293
intptr_t readFile(intptr_t);                         // 1294
void writeFile(intptr_t, intptr_t);                  // 1295
void writeAppend(intptr_t, intptr_t);                // 1296
void writeByte(intptr_t, intptr_t);                  // 1297
intptr_t strcmp_ajeeb(intptr_t, intptr_t);           // 1298
intptr_t trim(intptr_t);                             // 1299
intptr_t toUpperCase(intptr_t);                      // 1300
intptr_t toLowerCase(intptr_t);                      // 1301
intptr_t startsWith(intptr_t, intptr_t);             // 1302
intptr_t endsWith(intptr_t, intptr_t);               // 1303
intptr_t replace(intptr_t, intptr_t, intptr_t);      // 1304
intptr_t array_to_string(intptr_t, intptr_t);        // 1305
intptr_t arr_len(intptr_t);                          // 1306
intptr_t __array_lit(intptr_t, ...);                 // 1307
intptr_t exec(intptr_t);                             // 1308
intptr_t mkdir(intptr_t);                            // 1309
intptr_t print(intptr_t);                            // 1310
intptr_t allocBuf(intptr_t);                         // 1311
```

**Total: 32 declarations** (all runtime functions)

---

## 3. Side-by-Side Comparison

| # | Function | compiler.ajb C header | main.ajb emitCHeader | Runtime impl | Status |
|---|----------|----------------------|---------------------|--------------|--------|
| 1 | `getStateBuf` | ✅ line 89 | ✅ line 1280 | ✅ line 636 | MATCH |
| 2 | `getOutbuf` | ✅ line 85 | ✅ line 1281 | ✅ line 640 | MATCH |
| 3 | `getInt` | ✅ line 76 | ✅ line 1282 | ✅ line 561 | MATCH |
| 4 | `setInt` | ✅ line 77 | ✅ line 1283 | ✅ line 565 | MATCH |
| 5 | `len` | ✅ line 79 | ✅ line 1284 | ✅ line 608 | MATCH |
| 6 | `strSet` | ✅ line 86 | ✅ line 1285 | ✅ line 631 | MATCH |
| 7 | `charCode` | ✅ line 78 | ✅ line 1286 | ✅ line 582 | MATCH |
| 8 | `str_concat` | ✅ line 91 | ✅ line 1287 | ✅ line 781 | MATCH |
| 9 | `substring` | ✅ line 92 | ✅ line 1288 | ✅ line 801 | MATCH |
| 10 | `indexOf` | ✅ line 93 (2 args) | ✅ line 1289 (2 args) | ✅ line 823 (**3 args**) | **SIGNATURE MISMATCH** |
| 11 | `contains` | ✅ line 94 | ✅ line 1290 | ✅ line 834 | MATCH |
| 12 | `itoa` | ✅ line 87 | ✅ line 1291 | ✅ line 759 | MATCH |
| 13 | `println` | ✅ line 88 | ✅ line 1292 | ✅ line 727 | MATCH |
| 14 | `readArg` | ✅ line 80 | ✅ line 1293 | ✅ line 655 | MATCH |
| 15 | `readFile` | ✅ line 81 | ✅ line 1294 | ✅ line 680 | MATCH |
| 16 | `writeFile` | ✅ line 82 | ✅ line 1295 | ✅ line 701 | MATCH |
| 17 | `writeAppend` | ✅ line 83 | ✅ line 1296 | ✅ line 710 | MATCH |
| 18 | `writeByte` | ✅ line 84 | ✅ line 1297 | ✅ line 719 | MATCH |
| 19 | `strcmp_ajeeb` | ✅ line 90 | ✅ line 1298 | ✅ line 765 | MATCH |
| 20 | `trim` | ✅ line 97 | ✅ line 1299 | ✅ line 888 | MATCH |
| 21 | `toUpperCase` | ✅ line 95 | ✅ line 1300 | ✅ line 853 | MATCH |
| 22 | `toLowerCase` | ✅ line 96 | ✅ line 1301 | ✅ line 871 | MATCH |
| 23 | `startsWith` | ✅ line 99 | ✅ line 1302 | ✅ line 899 | MATCH |
| 24 | `endsWith` | ✅ line 100 | ✅ line 1303 | ✅ line 911 | MATCH |
| 25 | `replace` | ✅ line 98 | ✅ line 1304 | ✅ line 948 | MATCH |
| 26 | `arr_len` | ❌ MISSING | ✅ line 1306 | ✅ line 613 | **MISSING from compiler.ajb** |
| 27 | `__array_lit` | ❌ MISSING | ✅ line 1307 | ✅ line 619 | **MISSING from compiler.ajb** |
| 28 | `exec` | ❌ MISSING | ✅ line 1308 | ✅ line 688 | **MISSING from compiler.ajb** |
| 29 | `mkdir` | ❌ MISSING | ✅ line 1309 | ✅ line 692 | **MISSING from compiler.ajb** |
| 30 | `print` | ❌ MISSING | ✅ line 1310 | ✅ line 733 | **MISSING from compiler.ajb** |
| 31 | `allocBuf` | ❌ MISSING | ✅ line 1311 | ✅ line 569 | **MISSING from compiler.ajb** |
| 32 | `array_to_string` | ❌ MISSING | ✅ line 1305 | ✅ line 1420 | **MISSING from compiler.ajb** |
| — | `parseExpr` | ✅ line 101 | ❌ NOT declared | ❌ NOT in runtime | Forward decl (generated code defines it) |
| — | `parseStmt` | ✅ line 102 | ❌ NOT declared | ❌ NOT in runtime | Forward decl (generated code defines it) |

---

## 4. Summary of Actual Gaps

### compiler.ajb C header is MISSING 7 functions that main.ajb declares:

| # | Function | Signature in Runtime | Used by compiler.ajb itself? | Needed for bootstrap? |
|---|----------|---------------------|---------------------------|---------------------|
| 1 | `arr_len` | `intptr_t arr_len(intptr_t)` | No | Only if compiled code uses arrays |
| 2 | `__array_lit` | `intptr_t __array_lit(intptr_t, ...)` | No | Only if compiled code uses array literals |
| 3 | `exec` | `int64_t exec(int64_t)` | **YES** — lines 1439, 1440, 1611 | **YES — compiler calls exec()** |
| 4 | `mkdir` | `int64_t mkdir(int64_t)` | **YES** — line 1523 | **YES — compiler calls mkdir()** |
| 5 | `print` | `intptr_t print(intptr_t)` | No | Only if compiled code uses print() |
| 6 | `allocBuf` | `intptr_t allocBuf(intptr_t)` | No | Only if compiled code uses allocBuf() |
| 7 | `array_to_string` | `intptr_t array_to_string(intptr_t, int64_t)` | No | Only if compiled code prints arrays |

### CRITICAL FINDING: `exec` and `mkdir` ARE called by compiler.ajb itself

compiler.ajb calls these functions at runtime (when it executes), not in generated C code:
- `exec("which gcc > /dev/null 2>&1")` — line 1439
- `exec("which llc > /dev/null 2>&1")` — line 1440
- `mkdir("build")` — line 1523
- `exec(gccCmd)` — line 1611

**These are NOT blocked by missing C header declarations** — the compiler calls them directly as Ajeeb functions. The runtime provides them. The C header is only for generated code.

### Signature mismatch: `indexOf`

| Location | Declaration |
|----------|-------------|
| Runtime (`ajeeb_runtime.c:823`) | `intptr_t indexOf(intptr_t s, intptr_t search, intptr_t start)` — **3 args** |
| compiler.ajb C header (line 93) | `intptr_t indexOf(intptr_t s, intptr_t search)` — **2 args** |
| main.ajb C header (line 1289) | `intptr_t indexOf(intptr_t, intptr_t)` — **2 args** |

This is a **signature mismatch**, not a missing declaration. C allows calling with wrong arg count (undefined behavior, but usually works on x86-64 due to register-based calling convention).

### Bogus declarations in compiler.ajb C header

| Line | Declaration | Problem |
|------|-------------|---------|
| 101 | `intptr_t parseExpr(...)` | Not a runtime function — forward decl of a function defined in generated C |
| 102 | `intptr_t parseStmt(...)` | Not a runtime function — forward decl of a function defined in generated C |

These are harmless forward declarations for functions the generated C defines.

---

## 5. Verdict: Is Bootstrap Blocked?

**No. Bootstrap is NOT blocked by 16 missing declarations.**

### What IS true:
1. compiler.ajb's C header is missing 7 functions that main.ajb declares (arr_len, __array_lit, exec, mkdir, print, allocBuf, array_to_string)
2. Of these, only `exec` and `mkdir` are called by compiler.ajb itself — but these work fine because the runtime provides them
3. The other 5 (arr_len, __array_lit, print, allocBuf, array_to_string) are only needed if the *compiled* code uses arrays — not for bootstrap
4. `indexOf` has a 2-vs-3 arg signature mismatch (harmless on x86-64)

### What is NOT true:
- There are NOT 16 missing declarations — there are 7 (or 6 if you exclude `array_to_string`)
- None of these block bootstrap — `exec`/`mkdir` are runtime calls, not C header dependencies
- The 5 "missing" array/print functions are only needed for programs that use those features

### What WOULD block bootstrap:
If the generated C code calls a function not declared in the C header AND not defined in the runtime. But the generated C code from compiler.ajb only calls functions that ARE declared (getInt, setInt, len, strSet, charCode, str_concat, substring, indexOf, contains, println, strcmp_ajeeb, itoa, writeFile, writeAppend, writeByte, readFile, toUpperCase, toLowerCase, trim, startsWith, endsWith, replace).

---

## 6. Corrected Declarations That Should Be Added

If you want compiler.ajb's C header to match main.ajb's, add these 7 lines after line 100 (after `endsWith`):

```
emitStr(out, "intptr_t array_to_string(intptr_t, intptr_t);\n");
emitStr(out, "intptr_t arr_len(intptr_t);\n");
emitStr(out, "intptr_t __array_lit(intptr_t, ...);\n");
emitStr(out, "intptr_t exec(intptr_t);\n");
emitStr(out, "intptr_t mkdir(intptr_t);\n");
emitStr(out, "intptr_t print(intptr_t);\n");
emitStr(out, "intptr_t allocBuf(intptr_t);\n");
```

And fix the `indexOf` signature on line 93 from 2 args to 3:
```
emitStr(out, "intptr_t indexOf(intptr_t, intptr_t, intptr_t);\n");
```

**These are not blocking bootstrap.** They are completeness fixes for programs that use arrays, exec, mkdir, print, or allocBuf.
