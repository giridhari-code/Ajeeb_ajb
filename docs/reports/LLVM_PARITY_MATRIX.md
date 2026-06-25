# LLVM Parity Matrix

## MIR Opcode Coverage (Rust LLVM backend vs C backend)

| Opcode | MIR | C Backend | LLVM Backend | Status |
|--------|-----|-----------|--------------|--------|
| ASSIGN | op=2, ext=101 | `tN = tM;` | load/store | ✅ |
| CONST_LOAD | op=2, ext=1/3 | `tN = value;` | store | ✅ |
| BINOP_ADD | ext=1 | `+` | add/str_concat | ✅ |
| BINOP_SUB | ext=2 | `-` | sub | ✅ |
| BINOP_MUL | ext=3 | `*` | mul | ✅ |
| BINOP_DIV | ext=4 | `/? :` | sdiv + zero check | ✅ |
| BINOP_EQ | ext=5 | `==`/strcmp | icmp eq / strcmp | ✅ |
| BINOP_NE | ext=6 | `!=`/strcmp | icmp ne / strcmp | ✅ |
| BINOP_LT | ext=7 | `<` | icmp slt | ✅ |
| BINOP_GT | ext=8 | `>` | icmp sgt | ✅ |
| BINOP_LE | ext=9 | `<=` | icmp sle | ✅ |
| BINOP_GE | ext=10 | `>=` | icmp sge | ✅ |
| BINOP_AND | ext=11 | `&&` | and i1 | ✅ |
| BINOP_OR | ext=12 | `\|\|` | or i1 | ✅ |
| BINOP_MOD | ext=15 | `%` | srem + zero check | ✅ |
| BINOP_POW | ext=16 | `__ipow()` | call @__ipow | ✅ |
| CALL | op=4 | function call | call | ✅ |
| RETURN | op=5 | `return` | ret | ✅ |
| JUMP | op=6 | `goto` | br label | ✅ |
| BRANCH | op=7 | `if/else` | icmp + br | ✅ |
| PARAM | op=11 | `pN = pM` | load/store | ✅ |
| STR_CONCAT | op=12 | `str_concat()` | call @str_concat | ✅ |
| STR_LITERAL | ext=13 | `(intptr_t)"str"` | ptrtoint @.str.N | ✅ |
| DOTACCESS | op=13 | `tN = tM` | load/store | ✅ |
| NEW | op=14 | `allocBuf()` | call @allocBuf | ✅ |
| LOAD | op=9 | `tN = pM` | load/store | ✅ |
| __INDEX | intrinsic | GEP + load | GEP + load | ✅ |
| __INDEX_ASSIGN | intrinsic | GEP + store | GEP + store | ✅ |
| __ARRAY_LIT | intrinsic | variadic call | variadic call | ✅ |

## Runtime Function Declarations

| Function | Args | C Header | LLVM Preamble | Status |
|----------|------|----------|---------------|--------|
| indexOf | 2 | `indexOf(s, sub)` | `@indexOf(i64, i64)` | ✅ |
| __array_lit | variadic | `__array_lit(count, ...)` | `@__array_lit(i64, ...)` | ✅ |
| __index | 2 | `__index(arr, idx)` | `@__index(i64, i64)` | ✅ |
| __index_assign | 3 | `__index_assign(arr, idx, val)` | `@__index_assign(i64, i64, i64)` | ✅ |

## Regression Test Results

| Test | LLVM | C (GCC) | Notes |
|------|------|---------|-------|
| test_simple | ✅ | ✅ | |
| test_small | ✅ | ✅ | |
| test_strings | ✅ | ✅ | Fixed indexOf arity |
| test_math | ✅ | ✅ | |
| test_for | ✅ | ✅ | |
| test_if | ✅ | ✅ | |
| test_while | ✅ | ✅ | |
| test_array | ✅ | ✅ | Added __index, __index_assign |
| cross_simple | ✅ | ⚠️ | Pre-existing C codegen var-decl bug |
| compiler.ajb (bootstrap) | ✅ | ✅ | 145KB binary |
| parth_m1.ajb | ✅ | ✅ | |

## Summary
- **LLVM backend: 100% test pass rate**
- **C backend: 9/10 pass** (cross_simple has pre-existing variable declaration bug in Rust C codegen)
- All MIR opcodes implemented in both backends
- String literal emission fixed (previously stubbed to 0)
- Array runtime functions added to both C runtime and LLVM declarations
