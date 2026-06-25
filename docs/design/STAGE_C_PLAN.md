# STAGE C PLAN

**Date:** 2026-06-24
**Status:** PLANNING — no code changes yet
**Depends on:** Stage A (complete), Stage B (complete)

---

## 1. What Stage C Is

Per `PURE_AJEEB_ROADMAP.md`, Stage C is **"LLVM Codegen in Ajeeb"** — the self-hosted compiler (`compiler.ajb`) generates LLVM IR text (`.ll` files), which `llc` compiles to assembly and `ld` links to a native binary.

```
Current:    compiler.ajb → C text → GCC → binary
Stage C:    compiler.ajb → LLVM IR text → llc → ld → binary
```

**Rationale:** The Rust compiler already generates LLVM IR as **text strings** (not via inkwell/llvm-sys). The Ajeeb compiler can do the same — pure string building. This eliminates the GCC dependency and enables LLVM optimizations.

---

## 2. Prerequisites Assessment

### What Exists

| Component | Status | Notes |
|-----------|--------|-------|
| C codegen (85% parity) | ✅ Working | `main.ajb` emits C source text |
| MIR (95% parity) | ✅ Working | 12+ opcodes, optimizations |
| HIR (53% parity) | ✅ Working | Core statements and expressions |
| Rust LLVM backend (reference) | ✅ Working | 2,325 LOC across 7 files |
| Self-hosting via C | ✅ Working | compiler.ajb → C → binary |
| `llc` / `ld` on system | ✅ Available | Standard LLVM toolchain |

### What's Missing for Stage C

| Component | LOC Needed | Complexity | Priority |
|-----------|-----------|------------|----------|
| LLVM IR emission core | 800-1,000 | High | P0 |
| Type mapping (Ajeeb→LLVM types) | 100-150 | Low | P0 |
| Control flow (br, phi, labels) | 200-300 | Medium | P0 |
| Runtime function declarations | 100-150 | Low | P0 |
| Global state (arena, buffers) | 100-150 | Low | P0 |
| String concat patterns (5 patterns) | 200-300 | High | P0 |
| Array/struct allocation | 200-300 | High | P1 |
| Optimization passes | 200-300 | Medium | P1 |
| **Total** | **1,900-2,650** | | |

**Reference:** Rust LLVM backend is 2,325 LOC. Expect similar.

---

## 3. Blockers for a Pure-Ajeeb Toolchain

The pure-Ajeeb goal is: **Ajeeb + GCC + LLC = full compiler** (no Rust, no Cargo).

### Critical Blockers (Must Resolve for Pure Ajeeb)

| # | Blocker | Impact | Effort to Resolve |
|---|---------|--------|-------------------|
| **B1** | `main.ajb` can't compile `compiler.ajb` yet | Self-hosting impossible | 455-660 LOC (6 features: import, class, self, new, true/false, const) |
| **B2** | No LLVM codegen in Ajeeb | Must use GCC (not pure) | 1,900-2,650 LOC |
| **B3** | Parth Ajeeb version has 7 commands vs Rust 35 | Package mgmt incomplete | Already mostly done in Stage B (Rust Parth works) |
| **B4** | No semantic analysis in Ajeeb compiler | No error detection | 1,600+ LOC |
| **B5** | No interpreter in Ajeeb compiler | Can't run code directly | 1,800+ LOC |

### Non-Critical (Can Ship Without)

| # | Issue | Impact | Can Defer? |
|---|-------|--------|-----------|
| C1 | `str_concat` with non-string args segfaults | Runtime crash | Yes — known workaround |
| C2 | Missing `__struct_*` runtime functions | Structs unusable in C mode | Yes — use LLVM backend |
| C3 | No module cache | Slow recompilation | Yes — recompile always works |
| C4 | No Windows bootstrap verification | Cross-platform gap | Yes — Linux first |
| C5 | No documentation suite | Adoption barrier | Yes — ship code first |

### Dependency Chain

```
B1 (self-hosting features)
  └─→ required before B2 matters
       └─→ B2 (LLVM codegen)
            └─→ enables pure Ajeeb toolchain
                 └─→ enables B5 (interpreter)
                      └─→ enables full ecosystem
```

**Key insight:** B1 (self-hosting features) is the real bottleneck, not B2 (LLVM). The C backend already works. LLVM is a performance upgrade, not a functionality requirement.

---

## 4. Goals, Milestones, Dependencies, and Effort

### M3.1: LLVM IR Emission Core
**Goal:** `compiler.ajb --llvm test.ajb` generates valid `.ll` file
**Depends on:** Stage A
**Effort:** 3-5 days (800-1,200 LOC)

| Task | LOC | Complexity |
|------|-----|-----------|
| LLVM IR preamble (target triple, runtime declarations) | 100-150 | Low |
| Function definitions (`define i64 @name(...)`) | 100-150 | Medium |
| Basic block creation and labeling | 50-80 | Low |
| Integer/string/boolean literal emission | 50-80 | Low |
| Binary operations → LLVM IR instructions | 80-100 | Medium |
| Comparison operations → `icmp` instructions | 30-50 | Low |
| Function calls → `call` instructions | 80-100 | Medium |
| Return statements → `ret` instructions | 20-30 | Low |
| Variable load/store → `load`/`store` | 100-150 | Medium |
| Global variable declarations | 50-80 | Low |

**Verification:**
- `compiler.ajb --llvm tests/test_simple.ajb` generates valid `.ll`
- `llc test_simple.ll -o test_simple.s && gcc test_simple.s -o test_simple` runs
- `./test_simple` outputs "Hello World"

### M3.2: Control Flow in LLVM
**Goal:** if/else, while, for, break/continue generate correct LLVM IR
**Depends on:** M3.1
**Effort:** 2-3 days (300-500 LOC)

| Task | LOC | Complexity |
|------|-----|-----------|
| Conditional branches (`br i1 %cond, label %then, label %else`) | 50-80 | Medium |
| Phi nodes for loop-carried values | 80-120 | High |
| Loop label tracking (break/continue targets) | 50-80 | Medium |
| Nested control flow | 50-80 | Medium |
| Short-circuit evaluation (`&&`, `||`) | 50-80 | High |

**Verification:**
- `compiler.ajb --llvm tests/test_if.ajb` → runs correctly
- `compiler.ajb --llvm tests/test_while.ajb` → runs correctly
- `compiler.ajb --llvm tests/test_for.ajb` → runs correctly

### M3.3: String Handling in LLVM
**Goal:** String literals, concatenation, and builtins work in LLVM mode
**Depends on:** M3.1
**Effort:** 2-3 days (300-500 LOC)

| Task | LOC | Complexity |
|------|-----|-----------|
| String literal globals (`@.str = constant [N x i8] c"...\00"`) | 50-80 | Low |
| `str_concat` call pattern (5 patterns from Rust) | 100-150 | High |
| `strlen`, `strcmp`, `substring`, `indexOf` calls | 50-80 | Medium |
| `println` multi-arg with auto-itoa conversion | 50-80 | Medium |
| String comparison (`==` content comparison, not pointer) | 30-50 | Medium |

**Verification:**
- `compiler.ajb --llvm tests/test_strings.ajb` → runs correctly
- `println("Hello ", 42)` doesn't segfault

### M3.4: Runtime Declarations and Global State
**Goal:** All 57+ runtime functions declared, arena/buffers initialized
**Depends on:** M3.1
**Effort:** 1-2 days (200-300 LOC)

| Task | LOC | Complexity |
|------|-----|-----------|
| 57 `declare` statements for runtime functions | 100-150 | Low |
| Arena allocator global (`@arena = global ptr null`) | 20-30 | Low |
| State buffer global (`@state = internal global [1024 x i64]`) | 20-30 | Low |
| Output buffer global (`@outbuf = internal global [65536 x i8]`) | 20-30 | Low |
| `@exit` declaration (missing in current LLVM) | 5-10 | Trivial |
| `main` function entry point with argc/argv | 20-30 | Low |

**Verification:**
- All 57 runtime functions resolve without linker errors
- `compiler.ajb --llvm tests/test_simple.ajb` links successfully

### M3.5: Struct and Array Support
**Goal:** Struct literals, field access, and arrays work in LLVM mode
**Depends on:** M3.2, M3.3
**Effort:** 2-3 days (300-500 LOC)

| Task | LOC | Complexity |
|------|-----|-----------|
| Struct type mapping (Ajeeb struct → LLVM `{ i64, i64, ... }`) | 50-80 | Medium |
| Struct literal construction (`{ .field = value }`) | 50-80 | Medium |
| Field access (`obj.field` → `extractvalue` or GEP) | 50-80 | Medium |
| Array allocation (dynamic via malloc or fixed) | 50-80 | High |
| Array indexing (`arr[i]` → GEP + load) | 50-80 | High |
| `__array_lit`, `__index`, `__index_assign` as LLVM intrinsics | 50-80 | High |

**Verification:**
- `compiler.ajb --llvm tests/struct_basic.ajb` → runs correctly
- `compiler.ajb --llvm tests/struct_literal.ajb` → runs correctly
- `compiler.ajb --llvm tests/test_array.ajb` → runs correctly

### M3.6: Optimization Passes
**Goal:** Dead code/block elimination and constant folding in LLVM mode
**Depends on:** M3.2
**Effort:** 1-2 days (200-300 LOC)

| Task | LOC | Complexity |
|------|-----|-----------|
| Dead block elimination (scan reachability) | 50-80 | Medium |
| Unreachable block elimination | 30-50 | Low |
| Dead code elimination (unused assignments) | 50-80 | Medium |
| Constant folding (compile-time evaluation) | 50-80 | Medium |

**Verification:**
- LLVM output doesn't contain unreachable blocks
- Constant expressions folded at compile time

### M3.7: Integration and Bootstrap
**Goal:** `compiler.ajb --llvm compiler.ajb` produces working self-compiled binary
**Depends on:** M3.1-M3.6
**Effort:** 2-3 days (integration + bug fixing)

| Task | LOC | Complexity |
|------|-----|-----------|
| `--llvm` flag in CLI | 10-20 | Low |
| MIR→LLVM mapping for all 52 functions in compiler.ajb | 0 (uses M3.1-M3.5) | — |
| Import resolution in LLVM mode | 50-80 | Medium |
| String literal handling for 100+ strings in compiler.ajb | 0 (uses M3.3) | — |
| Self-compilation test | 0 (test only) | — |
| Bootstrap SHA-256 verification | 0 (test only) | — |

**Verification:**
- `compiler.ajb --llvm compiler.ajb` → build/compiler_llvm
- `build/compiler_llvm --llvm tests/test_simple.ajb` → runs correctly
- Both C and LLVM backends produce correct output for all test files

---

## 5. Effort Summary

| Milestone | Days | LOC | Dependencies |
|-----------|------|-----|-------------|
| M3.1: LLVM IR Core | 3-5 | 800-1,200 | Stage A |
| M3.2: Control Flow | 2-3 | 300-500 | M3.1 |
| M3.3: String Handling | 2-3 | 300-500 | M3.1 |
| M3.4: Runtime Declarations | 1-2 | 200-300 | M3.1 |
| M3.5: Struct/Array Support | 2-3 | 300-500 | M3.2, M3.3 |
| M3.6: Optimization Passes | 1-2 | 200-300 | M3.2 |
| M3.7: Integration/Bootstrap | 2-3 | 100-200 | M3.1-M3.6 |
| **Total** | **13-21 days** | **2,200-3,500** | |

**Calendar estimate:** 3-4 weeks (single developer, full-time)

---

## 6. What's NOT in Stage C

| Feature | Why Deferred |
|---------|-------------|
| Interpreter mode | Stage D (separate effort, 2 weeks) |
| Semantic analysis | Not needed for codegen correctness |
| Generics/traits/enums/patterns | Not used by compiler.ajb itself |
| Module cache | Not needed for correctness |
| Package registry server | Infrastructure, not compiler |
| Windows support | Platform-specific, not core |

---

## 7. Recommended First Milestone

### **Start with M3.1: LLVM IR Emission Core**

**Why this first:**
1. **Highest leverage** — every subsequent milestone depends on it
2. **Validates the approach** — proves Ajeeb can generate LLVM IR text
3. **Incremental testability** — can verify with `llc` after each function added
4. **Reference exists** — Rust LLVM backend is 2,325 LOC to copy from
5. **No dependencies** — doesn't require semantic analysis, generics, or other missing features

**M3.1 deliverable:**
```
compiler.ajb --llvm tests/test_simple.ajb
  → generates test_simple.ll
  → llc test_simple.ll -o test_simple.s
  → gcc test_simple.s -o test_simple
  → ./test_simple
  → "Hello World"
```

**Success criteria for M3.1:**
- [ ] `--llvm` flag recognized by CLI
- [ ] LLVM IR preamble emitted (target triple, runtime declarations)
- [ ] `main()` function generated with argc/argv
- [ ] `println("Hello World")` generates correct LLVM IR
- [ ] String literal globals emitted correctly
- [ ] Generated `.ll` file passes `llc` compilation
- [ ] Linked binary runs and produces correct output

**Estimated effort:** 3-5 days, 800-1,200 LOC

---

## 8. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| LLVM IR syntax errors | High | Medium | Validate each function individually; use llc for instant feedback |
| String concat patterns complex | High | Medium | Copy 5 patterns from Rust backend directly |
| Phi nodes for loops | Medium | High | Start with simple loops; defer complex cases |
| Self-compilation fails | Medium | High | Test with simple programs first; bootstrap incrementally |
| Performance regression | Low | Low | LLVM optimized builds should be faster than GCC |

---

## 9. Success Criteria (Stage C Complete)

| Criterion | Status |
|-----------|--------|
| `compiler.ajb --llvm test.ajb` generates valid `.ll` | Required |
| `llc + gcc` produces working binary | Required |
| All existing C-backend tests pass via LLVM backend | Required |
| `compiler.ajb --llvm compiler.ajb` self-compiles | Required |
| Bootstrap SHA-256 verification passes | Required |
| LLVM output is optimized (no dead blocks/code) | Nice-to-have |
| LLVM backend handles all 52 functions in compiler.ajb | Required |
