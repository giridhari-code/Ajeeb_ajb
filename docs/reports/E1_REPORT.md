# E1 Report — Self-host Bootstrap

**Status:** PASS

## Bootstrap Chain

```
Gen0 (Rust) → Gen1 (Ajeeb via LLVM) → Gen2 (Ajeeb via Gen1 C codegen)
```

### Step 1: Gen0 (Rust) compiles compiler.ajb → Gen1
- Binary: `build/compiler` (145,032 bytes)
- Backend: LLVM (llc → as → cc)
- Compile time: ~1.8 seconds

### Step 2: Gen1 runs on compiler.ajb → C code
- Output: `build/output.c` (2,705 lines)
- Time: ~30 seconds
- C compilation: `gcc build/output.c runtime/ajeeb_runtime.c -o build/compiler_gen2`

### Step 3: Gen2 runs on compiler.ajb → C code
- Output: `build/output.c` (2,705 lines)
- Time: ~13 seconds

### Step 4: Compare Gen1 C vs Gen2 C
- **IDENTICAL** — C output is stable across generations

## Test File Verification

All 7 core test files produce identical C output between Gen1 and Gen2:

| Test | Gen1 Lines | Gen2 Lines | Match | Output |
|------|-----------|-----------|-------|--------|
| test_simple | 85 | 85 | ✓ | Hello World |
| test_math | 85 | 85 | ✓ | 42 |
| test_if | 85 | 85 | ✓ | bada hai |
| test_while | 83 | 83 | ✓ | 0 1 2 |
| test_for | 87 | 87 | ✓ | 0 1 2 4 5 |
| test_strings | 85 | 85 | ✓ | Hello World HELLO ajeeb 1 1 Hello |
| cross_simple | 108 | 108 | ✓ | sum: 30 factorial(5): 120 ... |

## Bug Fix Applied

**Forward declarations for imported functions:** `compiler.ajb`'s `scanForwardDecls` only scans the current file's tokens. Functions from imported files (`stmt.ajb`, `expr.ajb`, `pass1.ajb`) used before they're defined get missing forward declarations in C output.

**Fix:** Added manual forward declarations for all imported functions in `compiler.ajb` main function (21 declarations for `isClassName`, `getVarType`, `registerVarType`, `parseType`, `parseStmt`, `parseExpr`, `parsePrimary`, etc.).

## Binary Sizes

| Generation | Size | Reduction from Gen0 |
|-----------|------|---------------------|
| Gen0 (Rust) | 14,928,880 bytes (14.2 MB) | — |
| Gen1 (Ajeeb) | 145,032 bytes (141 KB) | 99.0% |
| Gen2 (Ajeeb) | 145,112 bytes (141 KB) | 99.0% |

## Key Finding

Gen1/Gen2 are **self-hosting**: they can recompile `compiler.ajb` and produce identical C output. The bootstrap is stable. The self-hosted compiler (1,710 lines of Ajeeb) generates a 2,705-line C file that compiles to a 141 KB native binary.
