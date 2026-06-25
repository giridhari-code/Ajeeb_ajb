# Backend Controller Report (C6)

## Implementation

### Default Backend
- **LLVM** is now the default when `llc` is available
- Detection order: `llc` first, then `gcc`, then interpreter-only

### CLI Flags
| Flag | Effect |
|------|--------|
| `--llvm`, `-l`, `--backend=llvm` | Force LLVM backend |
| `--gcc`, `-g`, `--backend=c` | Force GCC/C backend |
| `--interpret`, `-i` | Interpreter only |
| Default (no flag) | LLVM with automatic fallback to GCC |

### Automatic Fallback
When LLVM compilation fails at any stage (llc, as, or cc), the compiler automatically falls back to the GCC/C backend with a warning message:
```
⚠️  LLVM failed (llc), falling back to GCC...
```

### Files Modified
1. **Rust compiler** (`crates/ajeeb-compiler/src/main.rs`):
   - `detect_backend()`: Prioritizes llc over gcc
   - Added `--backend=llvm` and `--backend=c` CLI flags
   - Fallback logic already existed (line 418)

2. **Self-hosted compiler** (`compiler/main.ajb`):
   - `detectBackend()`: Prioritizes llc over gcc
   - Added `--backend=llvm` and `--backend=c` CLI flags
   - LLVM backend now falls back to GCC on failure
   - Help text updated with new flags and default info

### Verification
```
$ ./build/ajeeb_compiler test.ajb           # Default: LLVM
⚡ Backend: LLVM (llc + as + ld)

$ ./build/ajeeb_compiler test.ajb --backend=llvm  # Force LLVM
⚡ Backend: LLVM (llc + as + ld)

$ ./build/ajeeb_compiler test.ajb --backend=c     # Force C
🔧 Backend: GCC (C codegen)

$ ./build/ajeeb_compiler test.ajb --gcc           # Force GCC (alias)
🔧 Backend: GCC (C codegen)
```
