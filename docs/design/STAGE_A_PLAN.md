# Stage A Plan: Feature Parity Execution

**Date:** 2026-06-22
**Status:** READY FOR APPROVAL
**Goal:** Complete feature parity between Rust and Ajeeb self-hosted compiler

---

## Phase 0: Critical Fixes (Week 1)

**Goal:** Unblock bootstrap — make compiler.ajb compilable.

### Task 0.1: Add Missing Runtime Function Declarations to C Codegen

**File:** `compiler/main.ajb` (codegen section)
**Current:** 25/41 functions declared
**Missing:** 16 functions

| Function | Signature | Priority |
|----------|-----------|----------|
| `substr` | `string substr(string str, int start, int len)` | P0 |
| `indexOf` | `int indexOf(string haystack, string needle)` | P0 |
| `toLowerCase` | `string toLowerCase(string str)` | P0 |
| `toUpperCase` | `string toUpperCase(string str)` | P0 |
| `trim` | `string trim(string str)` | P0 |
| `startsWith` | `bool startsWith(string str, string prefix)` | P0 |
| `endsWith` | `bool endsWith(string str, string suffix)` | P0 |
| `replace` | `string replace(string str, string from, string to)` | P0 |
| `replace_all` | `string replace_all(string str, string from, string to)` | P0 |
| `split` | `string[] split(string str, string delimiter)` | P0 |
| `parseInt` | `int parseInt(string str)` | P0 |
| `parseFloat` | `float parseFloat(string str)` | P0 (blocked on float type) |
| `chr` | `string chr(int code)` | P0 |
| `rdPos` | `int rdPos(string buf)` | P0 |
| `wrPos` | `void wrPos(string buf, int pos)` | P0 |

**Implementation:** Add to `emitFnDecl()` in `main.ajb:1280-1311`.

### Task 0.2: Test Self-Compilation

```bash
# Step 1: Build ajeeb_native using Rust interpreter
cargo run --bin ajeeb_compiler -- --interpret compiler/compiler.ajb
# Check output.c generated

# Step 2: Compile output.c with GCC
gcc -O2 -o build/ajeeb_native build/output.c runtime/ajeeb_runtime.c -lm

# Step 3: Run ajeeb_native on compiler.ajb
./build/ajeeb_native compiler/compiler.ajb > build/output2.c

# Step 4: Verify bootstrap
diff build/output.c build/output2.c
sha256sum build/output.c build/output2.c
```

### Task 0.3: Fix Any Bootstrap Failures

- If `diff` shows differences: investigate and fix
- If compilation fails: add missing function declarations
- If runtime errors: fix runtime or codegen

**Exit Criteria:** `diff build/output.c build/output2.c` shows no differences.

---

## Phase 1: Feature Parity — Lexer/Parser (Week 2)

**Goal:** Add missing tokens and syntax to match Rust compiler.

### Task 1.1: Add Missing Tokens

| Token | Code | Location |
|-------|------|----------|
| `float` | `30` | `lexer.ajb:114-154` |
| `pub` | `31` | `lexer.ajb:114-154` |
| `struct` | `32` | `lexer.ajb:114-154` |
| `enum` | `33` | `lexer.ajb:114-154` |
| `match` | `34` | `lexer.ajb:114-154` |
| `trait` | `35` | `lexer.ajb:114-154` |
| `impl` | `36` | `lexer.ajb:114-154` |
| `::` | `37` | `lexer.ajb:174-214` |
| `@` | `38` | `lexer.ajb:174-214` |
| `=>` | `39` | `lexer.ajb:174-214` |
| `_` | `40` | `lexer.ajb:174-214` |

### Task 1.2: Add Float Literal Parsing

**File:** `lexer.ajb` (readNumber)
**Logic:** Detect `.` in number, emit `tokType = 30` (float literal).

### Task 1.3: Add String Escape Handling

**File:** `lexer.ajb` (readString)
**Logic:** Parse `\n`, `\t`, `\"`, `\\`, `\0` and emit actual characters.

### Task 1.4: Add Parser Support for New Syntax

**File:** `stmt.ajb` and `expr.ajb`
**Tasks:**
- `pub` modifier on functions/classes
- `struct Name { fields }` parsing
- `enum Name { Variants }` parsing
- `trait Name { methods }` parsing
- `impl Type { methods }` parsing
- `match expr { pattern => expr }` parsing
- `|params| => body` lambda parsing
- Generic params `[T]` on functions
- Type arg list `[Int, String]` on calls
- `::` path syntax for imports

---

## Phase 2: Feature Parity — HIR/MIR (Week 3)

**Goal:** Add missing HIR nodes and MIR opcodes.

### Task 2.1: Add Missing HIR Types

| Type | HIR Code | Location |
|------|----------|----------|
| Float | `4` | `main.ajb:238-243` |
| Named (struct/enum) | `5` | `main.ajb:238-243` |
| Array | `6` | `main.ajb:238-243` |
| Generic | `7` | `main.ajb:238-243` |
| Fn (function pointer) | `8` | `main.ajb:238-243` |

### Task 2.2: Add Missing HIR Expressions

| Expression | HIR Code | Location |
|-----------|----------|----------|
| Float literal | `20` | `main.ajb:94-130` |
| Method call | `21` | `main.ajb:94-130` |
| Struct literal | `22` | `main.ajb:94-130` |
| Field access | `23` | `main.ajb:94-130` |
| Field assign | `24` | `main.ajb:94-130` |
| Index assign | `25` | `main.ajb:94-130` |
| Enum constructor | `26` | `main.ajb:94-130` |
| Unary minus | `27` | `main.ajb:94-130` |
| Unary not | `28` | `main.ajb:94-130` |
| Closure | `29` | `main.ajb:94-130` |
| Closure call | `30` | `main.ajb:94-130` |

### Task 2.3: Add Missing HIR Top-Level Definitions

| Definition | HIR Code | Location |
|-----------|----------|----------|
| Struct definition | `20` | `main.ajb:132-170` |
| Enum definition | `21` | `main.ajb:132-170` |
| Trait definition | `22` | `main.ajb:132-170` |
| Impl block | `23` | `main.ajb:132-170` |

### Task 2.4: Add Missing MIR Terminator

| Terminator | MIR Code | Location |
|-----------|----------|----------|
| Unreachable | `8` | `main.ajb:648` |

---

## Phase 3: Feature Parity — Codegen (Week 4)

**Goal:** Complete C backend parity and add LLVM backend skeleton.

### Task 3.1: Complete C Backend

| Feature | File | LOC |
|---------|------|-----|
| Float constants | `main.ajb:1166-1172` | 20 |
| Bool constants | `main.ajb:1166-1172` | 15 |
| Struct field access | `main.ajb:1211-1271` | 50 |
| Enum variant matching | `main.ajb:1259-1265` | 40 |
| Lambda/closure | `main.ajb` | 100 |
| Generic instantiation | `main.ajb` | 80 |

### Task 3.2: Add LLVM Backend Skeleton

**New file:** `compiler/llvm_codegen.ajb`
**Structure:**
- `emitLLVMHeader()` — LLVM IR header
- `emitLLVMFnDecl()` — Function declarations
- `emitLLVMBlock()` — Basic block emission
- `emitLLVMStmt()` — Statement codegen
- `emitLLVMExpr()` — Expression codegen
- `emitLLVMTerm()` — Terminator codegen

---

## Phase 4: Feature Parity — Semantic (Week 5)

**Goal:** Add type checking and semantic analysis.

### Task 4.1: Add Semantic Analyzer

**New file:** `compiler/semantic.ajb`
**Features:**
- Duplicate definition detection
- Type checking
- Variable resolution
- Method dispatch
- Generic instantiation

---

## Phase 5: Module System (Week 6)

**Goal:** Complete module system with cache and cycle detection.

### Task 5.1: Add Module Cache

**New file:** `compiler/module_cache.ajb`
**Features:**
- Cache compiled modules to disk
- SHA256-based invalidation
- Fast reload

### Task 5.2: Add Import Cycle Detection

**New file:** `compiler/cycle_detector.ajb`
**Features:**
- Track import stack
- Detect circular imports
- Error reporting

---

## Phase 6: LLVM Backend (Weeks 7-8)

**Goal:** Full LLVM codegen implementation.

### Task 6.1: Core LLVM Emission (800-1,000 LOC)
- Type mapping
- Basic blocks
- Instructions
- Control flow
- Runtime function declarations

### Task 6.2: String/Array Operations (300-400 LOC)
- String concatenation (5 patterns)
- Array allocation
- Struct allocation

### Task 6.3: Optimization Passes (200-300 LOC)
- Dead block elimination
- Unreachable block elimination
- Dead code elimination
- Constant folding

---

## Phase 7: Interpreter (Weeks 9-10)

**Goal:** Direct AST execution (no codegen).

### Task 7.1: Core Interpreter (1,000-1,500 LOC)
- AST evaluation
- Variable environment
- Function dispatch
- Builtin functions

---

## Phase 8: Package System (Weeks 11-12)

**Goal:** Parth integration for packages.

### Task 8.1: Package Resolution
- `import package;` → lookup in packages/
- Version resolution
- Dependency graph

---

## Timeline Summary

| Phase | Weeks | Deliverable |
|-------|-------|-------------|
| Phase 0: Critical Fixes | 1 | Bootstrap working |
| Phase 1: Lexer/Parser | 2 | All tokens and syntax |
| Phase 2: HIR/MIR | 3 | All nodes and opcodes |
| Phase 3: Codegen | 4 | C backend 100%, LLVM skeleton |
| Phase 4: Semantic | 5 | Type checking |
| Phase 5: Modules | 6 | Cache and cycle detection |
| Phase 6: LLVM | 7-8 | Full LLVM codegen |
| Phase 7: Interpreter | 9-10 | Direct AST execution |
| Phase 8: Package System | 11-12 | Parth integration |
| **Total** | **12 weeks** | **Full feature parity** |

---

## Success Criteria

| Criterion | Metric |
|-----------|--------|
| Bootstrap | `diff output.c output2.c` shows no differences |
| Self-compilation | compiler.ajb compiles itself without Rust |
| Test suite | All 6 core tests pass |
| Feature parity | 100% parity with Rust compiler |
| Performance | <2x slower than Rust for bootstrap |
| No external dependencies | Only gcc and llc (LLVM) |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Bootstrap failure | Phase 0 catches this first |
| Performance regression | Benchmark after each phase |
| Missing feature | Feature parity audit guides priorities |
| Scope creep | Strict phase boundaries |
| Time overrun | Phase 0 is critical path; rest is incremental |
