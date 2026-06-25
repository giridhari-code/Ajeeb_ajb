# PARTH M0 REPORT — Ajeeb Package Manager Bootstrap

## Status: COMPLETE

Parth M0 is a working Ajeeb package manager with 4 commands: `init`, `new`, `build`, `run`.

---

## Files Created/Modified

| File | LOC | Action | Purpose |
|------|-----|--------|---------|
| `parth/parth_m0.ajb` | 819 | **New** | Single-file concatenation of all modules |
| `parth/src/builder.ajb` | 134 | Modified | Fixed build pipeline for self-hosted compiler |
| `parth/src/main.ajb` | 161 | Unchanged | CLI dispatcher (already had init/new) |
| `parth/src/parser.ajb` | 206 | Unchanged | parth.das config parser |
| `parth/src/resolver.ajb` | 259 | Unchanged | Dependency resolver |
| `parth/src/runner.ajb` | 62 | Unchanged | Run pipeline |
| `build/parth_m0` | binary | **New** | Compiled native binary |

**Total Ajeeb LOC:** 819 (single-file) + 820 (5 source modules) = 1,639

---

## Commands Implemented

| Command | Description | Status |
|---------|-------------|--------|
| `parth init [name]` | Create project in current directory | Working |
| `parth new <name>` | Create named project directory | Working |
| `parth build` | Compile src/main.ajb → native binary | Working |
| `parth run` | Build + execute the binary | Working |
| `parth help` | Show usage information | Working |

---

## Build Process

Parth M0 was built using the self-hosted compiler pipeline:

```
1. Concatenate parth/src/{parser,resolver,builder,runner,main}.ajb → parth/parth_m0.ajb
2. Self-hosted compiler: ajeebc parth/parth_m0.ajb → build/output.c (C codegen)
3. Fix known C output issues (missing exec/mkdir decls, argc redeclaration)
4. gcc: output.c + runtime → build/parth_m0 (native binary)
```

### Known Compiler Issues (worked around in M0)

1. **Missing `exec`/`mkdir` declarations**: C output doesn't include declarations for `exec()` and `mkdir()`. Fixed via sed post-processing.
2. **`argc` redeclaration**: Compiler's `main()` signature uses `int argc` but then declares `intptr_t argc` inside. Fixed by renaming local variable.
3. **Import path hardcoding**: Compiler's import handler hardcodes `compiler/` prefix, preventing multi-file Ajeeb projects from compiling. Solved by concatenating into single file.

---

## Verification

### Test 1: `parth new` + `parth build` + binary execution
```
$ parth new test_build
✓ Project files bana: test_build

$ cd test_build && parth build
Project: test_build
Entry:   src/main.ajb
Output:  build/
Target:  native
Compile ho raha hai (native target)...
  ✓ Binary → build/test_build
Build safal! Chalao: ./build/test_build

$ ./build/test_build
Hello from Ajeeb!
```

### Test 2: `parth init`
```
$ mkdir test_init && cd test_init
$ parth init hello
✓ Project initialize ho gaya: hello
$ ls
parth.das  src/  build/
```

### Test 3: `parth run`
```
$ parth run
[builds project]
Chala rahe hain 'test_final'...
---
Hello from Ajeeb!
---
```

### Test 4: Help output
```
$ parth
Usage: parth <init|new|build|run|test|install|generate-lockfile>

Commands:
  init [name]       Project initialize karo current dir mein
  new <name>        Naya project banao
  build             ajeebc se compile karo (LLVM + gcc)
  run               ParthI interpreter se chalao
```

---

## Architecture

```
parth_m0.ajb (single file, 819 lines)
├── parser.ajb functions (206 LOC)
│   ├── bw/br (buffer read/write helpers)
│   ├── findChar, trimStart, trimEnd, skipBlankLines
│   ├── parseDasConfig (INI parser for parth.das)
│   └── getConfig*/getBuild*/getCompiler*/getDep* (config accessors)
├── resolver.ajb functions (259 LOC)
│   ├── fileExists, findPackageLocal, findPackageCache
│   ├── fetchPackage (git clone)
│   ├── resolveAll (dependency resolution)
│   ├── addDep, removeDep (parth.das modification)
│   └── generateLockfile
├── builder.ajb functions (134 LOC)
│   ├── readSource, makeBuildDir, findRuntime, findAjeebc
│   └── buildProject (ajeebc → C → gcc pipeline)
├── runner.ajb functions (62 LOC)
│   ├── findParthi
│   └── runProject (build + execute)
└── main.ajb functions (161 LOC)
    ├── cmdInit, cmdNew (project scaffolding)
    ├── cmdInstall (tool downloader)
    ├── runTests (stub)
    └── main (CLI dispatcher)
```

---

## Limitations (M0 Scope)

- **Single-file only**: Multi-file projects not supported due to compiler import bug
- **No dependency resolution at build time**: Local packages only; git fetch available but not tested
- **No version matching**: Dependencies are exact-match only
- **No lockfile**: `generate-lockfile` command exists but only writes local package info
- **No registry**: No remote package discovery or publishing
- **No test runner**: `runTests` is a stub
- **Hardcoded paths**: `findRuntime()` searches hardcoded paths; `cp` command uses absolute path to ajeebc runtime

---

## Self-Hosted Compiler Build

```bash
# Build parth M0 from Ajeeb source
cd /root/ajeeb_compiler
ajeebc parth/parth_m0.ajb          # Produces build/output.c
# Fix C output issues (sed)
gcc -no-pie build/output.c runtime/ajeeb_runtime.c -o build/parth_m0 -lm -ldl
```

---

## Next Steps (M1)

- Add `add`, `remove`, `install` commands
- Add version constraint matching (^, ~, >=)
- Add dependency tree display
- Add `test` command runner
- Fix compiler import path issue (enable multi-file projects)
