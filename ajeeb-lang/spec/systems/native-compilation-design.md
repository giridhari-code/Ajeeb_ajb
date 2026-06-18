# Native Compilation via LLVM IR — Design Document

## 1. Audit Summary

### Current State
- **Pure AST interpreter**: `evaluate_program` walks AST nodes directly; no bytecode, no IR, no machine code
- **C codegen path is dead code**: `main.rs` lines 157-175 check for `build/output.c` — but no code in the compiler generates this file. The `gcc` invocation is unreachable
- **No LLVM dependency**: Cargo.toml has no `inkwell`/`llvm-sys`; no LLVM IR generation
- **No Cranelift/JIT**: no just-in-time compilation
- **No serialization/bytecode format**: every run re-parses and re-analyzes from scratch
- **Performance**: AST interpretation is 100-1000x slower than native; all values cloned per function call; every expression matched via nested `match` statements

### Build Pipeline (Parth `cmd_build`)
```
parth build → cargo run ajeeb-compiler combined.ajb → no output.c generated → gcc never runs
```

The `build/` directory has a stale `output.c` from an earlier prototype, but no codegen module exists.

### Identified Gaps
| Gap | Severity | Description |
|-----|----------|-------------|
| No native compilation | BLOCKER | Interpreter-only; 100-1000x slower than native |
| C codegen path non-functional | HIGH | `main.rs` checks for `output.c` but nothing generates it |
| No bytecode | HIGH | Every run re-parses from scratch |
| No JIT | MEDIUM | No hot-path optimization |

### Breaking Changes
1. **Evaluator must be split** — the monolithic `exec_fn_call_body` (~800 lines of match arms) must be refactored into: (a) IR-generation pass, (b) optional interpreter fallback
2. **`RuntimeValue` representation** — must remain available as interpreter fallback; LLVM codegen generates native functions that produce the same `RuntimeValue` structure
3. **`main.rs` compilation pipeline** — currently lex → parse → module → semantic → execute; must become: lex → parse → module → semantic → codegen → link → execute

---

## 2. Design: LLVM IR Backend

### 2.1 Crate Choice

Use [inkwell](https://github.com/TheDan64/inkwell) — safe Rust bindings for LLVM. LLVM 18+.

```
cargo add inkwell --features llvm18-0
```

### 2.2 Architecture

```
┌─────────────┐   ┌──────────────┐   ┌────────────────┐
│  AST (Stmt) │──▶│  LLVM IR Gen │──▶│  LLVM Module   │
│  Vec<Stmt>  │   │  CodegenPass │   │  (in memory)   │
└─────────────┘   └──────┬───────┘   └───────┬────────┘
                         │                    │
                         ▼                    ▼
                  ┌──────────────┐   ┌────────────────┐
                  │  Runtime     │   │  Object File   │
                  │  Functions   │   │  (build/*.o)   │
                  │  (intrinsics)│   └───────┬────────┘
                  └──────────────┘           │
                                             ▼
                                    ┌────────────────┐
                                    │  Linker (gcc)  │
                                    │  → executable  │
                                    └────────────────┘
```

### 2.3 LLVM Module Structure

```rust
struct LlvmCodegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    globals: HashMap<String, GlobalValue<'ctx>>,
    runtime_values: PointerValue<'ctx>,  // heap-allocated RuntimeValue pool
    current_fn: Option<FunctionValue<'ctx>>,
}
```

**Key design decision**: The LLVM backend generates code that calls into the same C runtime functions (`ajeeb_runtime.c`). Each Ajeeb function becomes an LLVM function that returns an `AjeebValue` struct (by value, or via out-parameter).

### 2.4 RuntimeValue Lowering

LLVM struct for runtime values:

```llvm
%RuntimeValue = type { i32, i64, double, ptr, i64, i64 }
; Fields: tag (i32), as_int (i64), as_float (double),
;         string_ptr (ptr), array_data (ptr), array_len (i64)
```

Ajeeb code → LLVM IR example:

```ajeeb
function add(a: int, b: int): int {
    return a + b;
}
```

```llvm
define %RuntimeValue @add(%RuntimeValue %a, %RuntimeValue %b) {
  %a_int = extractvalue %RuntimeValue %a, 1
  %b_int = extractvalue %RuntimeValue %b, 1
  %result = add i64 %a_int, %b_int
  %rv = insertvalue %RuntimeValue undef, i32 0, 0  ; AJB_INT
  %rv2 = insertvalue %RuntimeValue %rv, i64 %result, 1
  ret %RuntimeValue %rv2
}
```

### 2.5 String Operations

String builtins already exist in `ajeeb_runtime.c`. The LLVM backend calls them:

```llvm
declare %RuntimeValue @str_concat(%RuntimeValue, %RuntimeValue)
```

No need to reimplement string ops in LLVM IR.

### 2.6 Class/Struct/Enum Lowering

Classes, structs, and enum variants are heap-allocated `AjeebValue` arrays. The LLVM backend:

1. Allocates on the arena (via `arena_alloc` from runtime)
2. Stores fields as consecutive `AjeebValue`s
3. Returns a pointer wrapped in `AjeebValue` with `AJB_STRUCT`/`AJB_ENUM` tag

---

## 3. Design: Executable Generation

### 3.1 Compilation Pipeline

```
┌─────────┐   ┌──────────┐   ┌──────────┐   ┌─────────┐   ┌──────────┐   ┌──────────┐
│  Source  │──▶│  Lexer  │──▶│  Parser  │──▶│Semantic │──▶│  LLVM    │──▶│  Object  │
│  .ajb    │   │         │   │          │   │Analyzer │   │  Codegen │   │  File    │
└─────────┘   └──────────┘   └──────────┘   └─────────┘   └────┬─────┘   └────┬─────┘
                                                                │              │
                                                                ▼              ▼
                                                         ┌──────────┐   ┌──────────┐
                                                         │ Runtime  │   │  Linker  │
                                                         │ .c/.o    │──▶│  (gcc)   │──▶ executable
                                                         └──────────┘   └──────────┘
```

### 3.2 Temporary Interpreter Fallback

During development, the compiler keeps the interpreter path. CLI flag selects mode:

```
ajeeb_compiler --mode=interpreter file.ajb     # current behavior
ajeeb_compiler --mode=native file.ajb          # LLVM codegen → executable
ajeeb_compiler --mode=both file.ajb            # compare results (testing)
```

### 3.3 Linking

Linked with:
- `runtime/ajeeb_runtime.c` (compiled to object)
- `-lm` (math)
- Optional: `-lsqlite3` (if SQLite), `-lcurl` (if libcurl)

---

## 4. Implementation Plan

### Phase 1A: LLVM IR for Expression Subset (3-4 weeks)

1. Add `inkwell` dependency
2. Create `crates/ajeeb-compiler/src/codegen.rs`
3. Implement `LlvmCodegen` struct with context/module/builder
4. Lower integer, float, bool, string literals
5. Lower binary operations (add, sub, mul, div, eq, cmp, and, or)
6. Lower variable load/store (let, const, assignment)
7. Write test: compile `fn add(a: int, b: int): int { return a + b; }` → verify IR

### Phase 1B: Control Flow + Functions (2-3 weeks)

1. Lower `FnDef` → LLVM function definition
2. Lower `If`/`While`/`For` → LLVM basic blocks + branches
3. Lower `Return` → LLVM ret
4. Lower `FnCall` → LLVM call
5. Write test: compile `fn factorial(n: int): int` → verify IR

### Phase 1C: Complex Types (3-4 weeks)

1. Lower `StructLit`, `Field`, `FieldAssign` → arena alloc + gep
2. Lower `EnumCtor`, `EnumRef`, `Match` → tag-checked branches
3. Lower `ArrayLit`, `Index`, `IndexAssign`
4. Lower `MethodCall` (class methods + trait dispatch)
5. Link with `ajeeb_runtime.c` for string operations

### Phase 1D: Executable Generation (1-2 weeks)

1. LLVM `module.verify()` → `module.print_to_file()`
2. Emit object file via `TargetMachine`
3. Invoke `gcc` to link with runtime → executable
4. Run compiled executable, compare output with interpreter

### Phase 1E: Parth Integration (1 week)

1. Update `parth build` to use LLVM backend when available
2. `parth.das` config: `[compiler] backend = "llvm"` or `"interpreter"`
3. Handle compilation of package dependencies

---

## 5. Complexity Estimate

| Step | Lines Changed | Files | Effort |
|------|--------------|-------|--------|
| LLVM IR scaffold | ~300 | `codegen.rs` (new), `Cargo.toml` | 1 week |
| Literals + binary ops | ~400 | `codegen.rs` | 1 week |
| Variables + control flow | ~500 | `codegen.rs` | 2 weeks |
| Functions + calls | ~400 | `codegen.rs` | 1 week |
| Struct/Enum/Match | ~600 | `codegen.rs` | 2 weeks |
| Array + classes | ~400 | `codegen.rs` | 1 week |
| Executable emission | ~200 | `codegen.rs`, `main.rs` | 1 week |
| Parth integration | ~100 | `parth/src/main.rs` | 2 days |
| **Total** | **~2900** | **~8 files** | **9-11 weeks** |

---

## 6. Migration Path

1. Keep interpreter as default; `--mode=native` is opt-in during development
2. Start with pure expression compilation (no I/O builtins)
3. Add I/O builtins by linking against `ajeeb_runtime.c`
4. Run test suite in `--mode=both` — assert identical output
5. When coverage is complete, make `--mode=native` default
6. Keep interpreter for tiny scripts and hot-reload scenarios

---

## 7. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| LLVM version mismatch | Pin LLVM 18; document build requirements |
| inkwell API instability | Pin inkwell version; limit feature surface |
| RuntimeValue layout mismatch | Single source of truth: the C header `runtime_value.h` |
| Missing string builtins | Always keep interpreter path as fallback |
| Linker errors with complex types | Test each type incrementally; start with int-only programs |
