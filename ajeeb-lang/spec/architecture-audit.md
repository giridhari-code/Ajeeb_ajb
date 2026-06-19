# Ajeeb Architecture Audit — Foundational Gaps Analysis

> **Context**: Ajeeb is an AST-interpreted language with a C runtime backend. The compiler (~3500 lines across 10 modules) handles lexing, parsing, semantic analysis, and direct execution. The Parth package manager (~2300 lines) provides dependency resolution. Four backend packages (json, log, http, db) are implemented. All values are `intptr_t` in the C runtime, strings leak memory, there is no GC, no borrow checker, no async runtime, and no native compilation pipeline.

---

## 1. Runtime Architecture & Memory Management

### Current State
- **Runtime**: `ajeeb_runtime.c` (420 lines) — raw `malloc`/`free` with no free strategy; every string operation (`readFile`, `str_concat`, `substring`, `replace`, `trim`, `toUpperCase`, `toLowerCase`, `itoa`) calls `malloc` and **never calls `free`**; the only cleanup is `flush_cached_files` at exit (for file handles only)
- **Value representation**: all values passed as `intptr_t` between Rust evaluator and C runtime — untyped, no tag, no discriminant; strings are raw pointers to `malloc`'d buffers
- **Rust evaluator** (`eval.rs`, 1606 lines): `RuntimeValue` enum with `Rc<RefCell<String>>` for strings, `Rc<RefCell<Vec<RuntimeValue>>>` for arrays — reference cycles never collected, all variables cloned per function call (`self.variables.clone()` at line 1522)
- **No GC, no RC, no arena, no region allocator**

### Comparison
| Language | Strategy | Notes |
|----------|----------|-------|
| Rust | Ownership + borrow checker | Zero-cost, compile-time guaranteed |
| Go | Concurrent GC (tri-color, non-generational) | Low-latency, <1ms typical pause |
| Zig | Manual with arena allocator | Explicit allocator parameter everywhere |
| Java | Generational GC (G1, ZGC) | Sub-millisecond pause, TB heaps |
| Python | Reference counting + generational GC | Cyclic reference collector |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| No memory management strategy | **P0** | 3-6 months | Every string operation leaks; no GC, RC, or arena; `intptr_t` untyped representation is a security risk |
| No allocator abstraction | **P1** | 1-2 months | All allocations go through `malloc`; no arena, no bump allocator, no per-function region |
| No value tagging/discriminant | **P1** | 2-4 weeks | `intptr_t` representation can't distinguish Int from Pointer at runtime; causes UB in C runtime |
| Rc/RefCell cycles leak in Rust eval | **P2** | 2-4 weeks | Arrays of class instances can form reference cycles; `Weak` not used anywhere |

---

## 2. Async / Concurrency Model

### Current State
- **Zero concurrency**: `eval.rs` is single-threaded, single-core; no `std::thread`, no `std::sync`, no channels, no atomics
- **No async runtime**: no event loop, no futures, no async/await, no `epoll`/`kqueue`/`io_uring`
- **Blocking I/O**: `tcp_accept` blocks the entire interpreter; `readFile` blocks; `writeFile` blocks
- **TCP listener**: Ajeeb HTTP server calls `tcp_accept` in a tight loop — one connection at a time, no connection pooling
- **No `Send`/`Sync` traits or equivalent**: not possible to express thread safety

### Comparison
| Language | Concurrency Model |
|----------|------------------|
| Rust | `async`/`await` + `tokio`/`async-std`; `Send` + `Sync` traits |
| Go | Goroutines (M:N scheduling), channels, `select` |
| Zig | `async`/`await` (stackful), event loop in std |
| Java | Virtual threads (Project Loom), `CompletableFuture`, `ForkJoinPool` |
| Python | `asyncio` (event loop), `threading` (GIL-bound), `multiprocessing` |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| No async runtime | **P0** | 6-12 months | Blocking I/O kills throughput; HTTP server serves 1 request at a time; no WebSocket, no streaming |
| No threading model | **P0** | 4-8 months | No threads, no parallelism, no `std::thread`; CPU-bound tasks block everything |
| No concurrency primitives | **P1** | 2-4 months | No mutex, no RwLock, no channels, no atomics, no condvar |
| No async I/O for TCP | **P1** | 3-6 months | `tcp_accept`, `tcp_read`, `tcp_write` are all blocking; no `epoll` integration |

---

## 3. Networking / TCP

### Current State
- **TCP builtins**: `tcp_listen`, `tcp_accept`, `tcp_read`, `tcp_write`, `tcp_close` implemented in `eval.rs` (lines 1373-1442) using Rust's `std::net::TcpListener` and `TcpStream`
- **Single-threaded accept loop**: HTTP server calls `tcp_accept` → `tcp_read` → handler → `tcp_write` → repeat; no connection pool, no keep-alive, no TLS
- **No DNS resolution**, no UDP, no Unix sockets, no named pipes
- **`tcp_read` sets non-blocking** (`stream.set_nonblocking(true).ok()` at line 1398) but then ignores `WouldBlock` — returns empty string on EAGAIN
- **C runtime TCP** (lines 332-385 in `ajeeb_runtime.c`) duplicates the same logic: blocking `accept`, `read`, `write` with POSIX sockets

### Comparison
| Language | Networking |
|----------|-----------|
| Rust | `tokio::net`, `hyper`, `tonic`; zero-cost async I/O |
| Go | `net` package with goroutine-per-connection; HTTP/2, TLS built-in |
| Zig | `std.net` with async I/O; `zap`/`zhttp` frameworks |
| Java | `java.nio.channels`, Netty, Vert.x; virtual thread-per-connection |
| Python | `asyncio` transports, `aiohttp`, `Twisted`, `Tornado` |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| No non-blocking I/O model | **P0** | 3-6 months | Blocking accept/read/write kills concurrency; needs epoll/kqueue/io_uring |
| No TLS/SSL | **P1** | 2-4 months | Can't serve HTTPS; no certificate handling, no `openssl` bindings |
| No HTTP/2 or WebSocket | **P2** | 3-6 months | HTTP server is raw TCP with manual parsing; no multiplexing, no streaming |
| No DNS resolver | **P2** | 2-4 weeks | `tcp_connect` takes IP only; no `getaddrinfo` wrapper |
| No UDP/Unix sockets | **P2** | 1-2 months | Only TCP via `tcp_listen`/`tcp_accept` |

---

## 4. FFI Design

### Current State
- **`interop.rs`** (40 lines): `LanguageBridge` struct with `HashMap<String, String>` — stores module name → language name pairs; `load_compatibility_block` just prints and stores a string; `resolve` looks up the string; **zero actual FFI functionality**
- **No C ABI binding**: can't call any C function, can't link to shared libraries, can't pass structs across FFI boundary
- **No WASM interop**: no `wasmtime`/`wasmer` integration
- **No dynamic library loading**: no `dlopen`/`dlsym`/`LoadLibrary`/`GetProcAddress`

### Comparison
| Language | FFI |
|----------|-----|
| Rust | `extern "C"`, `#[no_mangle]`, `bindgen`, `cbindgen`; seamless C/FFI |
| Go | `cgo` (slow but functional); `//export` and `import "C"` |
| Zig | `@cImport`, `@cInclude`, `extern fn`; best-in-class C interop |
| Java | JNI (verbose), JNA (easier), Panama (modern) |
| Python | `ctypes`, `cffi`, `pybind11` (C++), `numpy` C API |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| FFI is empty scaffolding | **P0** | 6-12 months | `LanguageBridge` is a HashMap of strings; no actual C ABI binding |
| No C ABI call support | **P0** | 3-6 months | Can't call any system library (libc, libm, libssl, etc.) |
| No dynamic linking | **P1** | 2-4 months | No `dlopen` pattern; all C code must be compiled statically |
| No WASM interop | **P2** | 3-6 months | No WebAssembly sandboxing or plugin system |

---

## 5. Type System Limitations

### Current State
- **No trait objects**: `trait` is only for static method dispatch via name mangling; you cannot write `fn foo(x: &dyn Trait)` or have heterogeneous collections
- **No `Self` type**: trait methods can't reference the implementing type
- **No associated types**: `trait Iterator { type Item; }` is impossible
- **No lifetimes**: all values are cloned per function call; no references, no borrow checking
- **No higher-kinded types**: no `Monad`, `Functor`, etc.
- **No union types, no intersection types, no type aliases**
- **`types_match` is extremely permissive**: `Generic(_)` matches everything; `Int ↔ String` is allowed (lines 327-331 in semantic.rs); `Int ↔ Class("Array")` matches (lines 343-366)
- **Generics are erased at runtime**: `GenericCall` is treated identically to `FnCall` in evaluator

### Comparison
| Feature | Rust | Go | Zig | Java | Python | Ajeeb |
|---------|------|----|-----|------|--------|-------|
| Trait objects | ✅ `dyn Trait` | ✅ `interface{}` | ✅ `anytype` | ✅ | ✅ (duck) | ❌ |
| Associated types | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Lifetime tracking | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Borrow checking | ✅ | ❌ | ❌ (runtime) | ❌ | ❌ | ❌ |
| HKTs | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Union types | ❌ | ❌ | ❌ | ❌ | ✅ (`\|`) | ❌ |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| No borrow checker / references | **P0** | 12-18 months | All values cloned per call; no shared state, no mutation tracking; foundational for performance |
| No trait objects / dynamic dispatch | **P1** | 3-6 months | Can't write generic code over trait bounds; no heterogeneous collections |
| No `Self` type or associated types | **P1** | 1-3 months | Traits can't express return types based on implementor; `Iterator::Item` impossible |
| `types_match` is too permissive | **P1** | 2-4 weeks | Int↔String coercion hides bugs; Generic matches everything including mismatched arity |
| No type aliases | **P2** | 1-2 weeks | `type UserId = Int` would reduce boilerplate |

---

## 6. Error Handling

### Current State
- **`CompileError`** (24 lines): `{ line, col, message }` — no error codes, no error recovery, no suggestions, no spans, no note/help/warning levels
- **Semantic errors are non-fatal**: compiler prints errors but continues to execution; runtime runs even with type errors
- **Runtime errors**: `eprintln!("[ERROR] ...")` and return `Int(0)` — division by zero returns 0, unknown variable returns 0, unknown function returns `Void`
- **No `Result[T, E]` integration at compiler level**: `packages/ajeeb-std/result.ajb` is a library enum with no compiler support; there's no `?` operator, no `try`, no error propagation
- **No panic/unwind mechanism**: no `catch`, no stack unwinding, no `defer`

### Comparison
| Language | Error Handling |
|----------|---------------|
| Rust | `Result<T, E>`, `?` operator, `panic!` + unwinding, `catch_unwind` |
| Go | `error` interface, `defer`/`panic`/`recover`, explicit `if err != nil` |
| Zig | Error union type `!T`, `try`/`catch`, error return traces |
| Java | Checked exceptions, `try`/`catch`/`finally`, `throw` |
| Python | Exceptions everywhere, `try`/`except`/`finally`, `raise` |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| No error recovery in parser | **P0** | 2-4 months | First parse error stops compilation; no error resilience, no "continue past error" |
| `CompileError` is minimal | **P1** | 2-4 weeks | No error codes, no spans, no suggestions, no diagnostics API |
| No `?` operator / error propagation | **P1** | 1-2 months | `Result` exists as library enum but has no language support |
| Semantic errors should be fatal | **P2** | 1 week | Running code with type errors is dangerous; should fail at compile time |
| Runtime errors return 0 silently | **P1** | 2-4 weeks | Division by zero, unknown variable, unknown function all silently return 0 |

---

## 7. Package Ecosystem (Parth)

### Current State
- **Parth** (688 lines): package manager with `new`, `add`, `remove`, `build`, `run`, `publish`, `search`, `install`, `sign`, `verify`, `audit`, `cache`, `workspace`, `info`, `clean` commands
- **Resolver** (549 lines): PubGrub-style backtracking dependency resolver with semver versioning, compound constraints (`^1.0.0`, `>=1.0.0 <2.0.0`, `||` combinators), lock file v2 with transitive deps
- **Registry** (519 lines): local index, SHA256 checksums, package signing, security advisories, audit scanning, cache management
- **Publish is local-only**: `register_package` writes to `~/.parth/index`; no HTTP upload logic; no registry server implementation
- **No lockfile integrity check on install**: `ensure_package` checks cache existence but `download_package` doesn't verify checksums
- **No SAT/Moore resolution**: resolver uses simple DFS with backtracking, not a proper PubGrub implementation

### Comparison
| Feature | Cargo | npm | Go Modules | Zig Build | Parth |
|---------|-------|-----|------------|-----------|-------|
| Lock file v2 with transitive deps | ✅ | ✅ | ✅ | ❌ | ✅ (basic) |
| Semver resolution | ✅ (caret by default) | ✅ (tilde by default) | ✅ (semver) | ✅ (git-based) | ✅ (basic) |
| Backtracking resolver | ✅ (PubGrub) | ✅ (DFS) | ✅ (MVS) | ❌ | ✅ (DFS) |
| Registry server | ✅ (crates.io) | ✅ (npmjs.com) | ✅ (proxy.golang.org) | ✅ (zig.pm) | ❌ (local-only) |
| Package signing | ❌ | ✅ (npm sign) | ✅ (checksum DB) | ❌ | ✅ (basic SHA256) |
| Workspace support | ✅ | ✅ | ❌ | ❌ | ✅ (basic) |
| Publish upload | ✅ | ✅ | ✅ | ❌ | ❌ (local index only) |
| Security audit | ✅ (cargo-audit) | ✅ (npm audit) | ❌ (govulncheck) | ❌ | ✅ (basic) |
| Build script support | ✅ (build.rs) | ✅ (postinstall) | ❌ | ✅ (build.zig) | ❌ |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| No registry server | **P0** | 3-6 months | `publish` writes to local index only; no HTTP API, no auth, no upload |
| No content-addressed storage | **P1** | 1-2 months | Packages stored by name/version; no integrity verification on install |
| No SAT solver | **P2** | 2-4 months | DFS backtracking may miss valid solutions; PubGrub not fully implemented |
| No build scripts | **P2** | 1-2 months | No way to run native code during build (e.g., codegen, protobuf compilation) |
| No binary distribution | **P2** | 2-3 months | Only source distribution; no prebuilt binaries or platform-specific artifacts |

---

## 8. Tooling

### Current State
- **Formatter** (ajeeb-fmt, 105 lines CLI + library): `--check`, `--write`, `--stdout`, indent config, max line width, tab support; handles all Phase 2 syntax (traits, impl, generics, match, enums)
- **No LSP**: no language server protocol implementation; no IDE support beyond basic text editing
- **No debugger**: no step-through debugging, no breakpoints, no variable inspection, no DWARF/PDB output
- **No testing framework**: `assert_eq`/`assert_neq`/`assert_contains` are builtins; no test runner, no benchmark harness, no coverage tool
- **No doc generator**: no `///` doc comments, no `rustdoc`-equivalent
- **No profiler**: no flamegraph, no CPU profiling, no memory profiling
- **No CI configuration**: no `.github/workflows`, no `Jenkinsfile`, no cross-platform CI matrix

### Comparison
| Tool | Rust | Go | Zig | Java | Python | Ajeeb |
|------|------|----|-----|------|--------|-------|
| Formatter | rustfmt | gofmt | zig fmt | prettier | black | ✅ basic |
| LSP | rust-analyzer | gopls | zls | eclipse.jdt.ls | pylance | ❌ |
| Debugger | lldb/gdb | delve | gdb | jdb | pdb | ❌ |
| Test runner | cargo test | go test | zig test | mvn test | pytest | ❌ (assert builtins) |
| Doc gen | rustdoc | godoc | ❌ | javadoc | sphinx | ❌ |
| Profiler | perf/flamegraph | pprof | perf | async-profiler | cProfile | ❌ |
| CI matrix | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| No LSP | **P0** | 6-12 months | No IDE support, no autocomplete, no go-to-definition, no hover info |
| No debugger | **P1** | 6-12 months | Can't step through code, inspect variables, set breakpoints |
| No test runner | **P1** | 2-4 months | `assert_eq` is a builtin but no test discovery, no test reporting, no benchmarks |
| No doc generator | **P2** | 2-4 months | No documentation comments, no doc generation |
| No profiler | **P2** | 3-6 months | No way to measure performance or find bottlenecks |

---

## 9. Cross-Platform Support

### Current State
- **C runtime**: uses POSIX sockets (`sys/socket.h`, `netinet/in.h`), `clock_gettime`, `read`/`write`; Windows path uses `_WIN32` with Winsock2 (lines 313-330, 332-385)
- **File paths**: `PathBuf` with `/` separator assumptions in module loader and test paths
- **No CI matrix**: no GitHub Actions, no cross-platform testing
- **No platform-conditional code**: no `#[cfg(target_os = "windows")]` equivalent; only `#ifdef _WIN32` in C runtime
- **No architecture-specific code**: assumes 64-bit (`i64`, `intptr_t`); no ARM, no RISC-V, no WASM target

### Comparison
| Feature | Rust | Go | Zig | Java | Python | Ajeeb |
|---------|------|----|-----|------|--------|-------|
| CI matrix | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Platform cfg | ✅ `#[cfg]` | ✅ `//go:build` | ✅ `@importBuiltin` | ✅ JVM | ✅ `sys.platform` | ❌ |
| Cross-compile | ✅ `--target` | ✅ `GOOS`/`GOARCH` | ✅ `-target` | ✅ JVM | ❌ (C extension) | ❌ |
| 32-bit support | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| ARM support | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| No CI/test matrix | **P1** | 1-2 weeks | No automated testing on Windows, macOS, Linux |
| No cross-compilation | **P1** | 2-4 months | Can't target ARM, WASM, or 32-bit platforms |
| C runtime has minimal platform abstraction | **P2** | 1-2 months | `_WIN32` paths exist but untested; no macOS-specific paths |
| No platform-conditional compilation in Ajeeb | **P2** | 1-2 months | No `#[cfg]` equivalent for platform-specific code |

---

## 10. Compilation Pipeline

### Current State
- **Pure AST interpreter**: `evaluate_program` walks the AST directly; no bytecode, no IR, no machine code
- **C codegen path**: `main.rs` lines 157-175 checks if `build/output.c` exists (generated by ??? — no codegen module exists); attempts `gcc` compilation but the c file isn't generated by any existing code path
- **No LLVM**: no `inkwell`/`llvm-sys` dependency; no LLVM IR generation
- **No Cranelift**: no JIT compilation
- **No WASM backend**: no `wasmtime` integration
- **No bytecode format**: no serialization, no caching of compiled modules

### Comparison
| Feature | Rust | Go | Zig | Java | Python | Ajeeb |
|---------|------|----|-----|------|--------|-------|
| Native compilation | ✅ LLVM | ✅ Go compiler | ✅ LLVM+ZIR | ✅ JIT (C2) | ❌ | ❌ |
| JIT | ❌ | ❌ | ❌ | ✅ C1/C2/Graal | ❌ | ❌ |
| Bytecode/IR | ✅ MIR/LLVM IR | ✅ SSA | ✅ ZIR | ✅ bytecode | ✅ .pyc | ❌ |
| C codegen | ✅ `rustc_codegen_llvm` | ❌ | ✅ | ❌ | ❌ | ❌ (stub) |
| WASM target | ✅ | ✅ | ✅ | ❌ | ✅ | ❌ |

### Gaps
| Gap | Priority | Complexity | Description |
|-----|----------|------------|-------------|
| No native compilation | **P0** | 12-24 months | AST interpretation is 100-1000x slower than native; no JIT, no LLVM, no Cranelift |
| C codegen path is non-functional | **P1** | 2-4 months | `main.rs` checks for `output.c` but no code generates it; this is a dead code path |
| No bytecode format | **P1** | 2-4 months | Every run re-parses and re-analyzes from scratch; no caching |
| No WASM backend | **P2** | 3-6 months | Can't run Ajeeb in browser or WASM runtime |
| No incremental compilation | **P2** | 3-6 months | Every run is full rebuild; no module caching |

---

## Summary: Roadmap Priorities

### Phase 1 — Critical (P0, 12-24 months total)
1. **Memory management** — Choose GC or ownership model; implement arena allocator in C runtime; tag runtime values
2. **Async/event loop** — Integrate epoll/kqueue/io_uring; non-blocking I/O; thread pool
3. **FFI — C ABI** — `extern "C"` binding; `dlopen`/`dlsym` loading; libc/libm wrappers
4. **Native compilation** — LLVM backend via `inkwell`; or transpile to C with proper codegen
5. **LSP** — Language Server Protocol implementation; autocomplete, go-to-definition, diagnostics

### Phase 2 — High (P1, 6-12 months)
1. **Trait objects** — `dyn Trait` with vtable-based dispatch
2. **Borrow checker** — Reference tracking, lifetime analysis, borrow checking
3. **Error recovery** — Resilient parser with error recovery; structured diagnostics
4. **Cross-platform CI** — GitHub Actions matrix for Linux/macOS/Windows
5. **Debugger support** — DWARF output or GDB/lldb integration
6. **Registry server** — HTTP API for publish/search/download with auth

### Phase 3 — Nice-to-have (P2, 3-6 months)
1. **Bytecode format** — Serialized compiled modules for faster startup
2. **WASM backend** — Browser and WASI targets
3. **Profiler** — CPU and memory profiling tools
4. **Doc generator** — Documentation comments and HTML generation
5. **Test framework** — `test` keyword, benchmark harness, coverage tool
6. **Standard library expansion** — HTTP client, crypto, regex, datetime, serialization

---

## File Reference Index

| File | Lines | Role |
|------|-------|------|
| `crates/ajeeb-compiler/src/eval.rs` | 1606 | AST interpreter — all runtime logic |
| `crates/ajeeb-compiler/src/semantic.rs` | 933 | Type checking + trait impl validation |
| `crates/ajeeb-compiler/src/parser.rs` | ~1400 | Recursive descent parser |
| `crates/ajeeb-compiler/src/lexer.rs` | 325 | Character-level tokenizer |
| `crates/ajeeb-compiler/src/ast.rs` | 296 | AST definition (Stmt, Expr, Pattern, etc.) |
| `crates/ajeeb-compiler/src/token.rs` | 63 | Token enum (63 variants) |
| `crates/ajeeb-compiler/src/module.rs` | 222 | File-based module loader |
| `crates/ajeeb-compiler/src/interop.rs` | 40 | FFI stub |
| `crates/ajeeb-compiler/src/error.rs` | 24 | CompileError struct |
| `crates/ajeeb-compiler/src/das_parser.rs` | 83 | parth.das config parser |
| `crates/ajeeb-compiler/src/main.rs` | 178 | CLI entry point |
| `runtime/ajeeb_runtime.c` | 420 | C runtime (malloc/string/TCP/SQLite) |
| `crates/parth/src/main.rs` | 688 | Parth CLI (new/add/build/run/publish/etc.) |
| `crates/parth/src/resolver.rs` | 549 | Dependency resolver (DFS+backtracking) |
| `crates/parth/src/registry.rs` | 519 | Registry client (local index/cache/signing/audit) |
| `crates/parth/src/config.rs` | 203 | parth.das config reader |
| `crates/parth/src/types.rs` | 337 | Version/Semver/Constraint/LockEntry types |
| `crates/ajeeb-fmt/src/main.rs` | 105 | CLI for formatter |
| `packages/ajeeb-std/` | 8 files | Standard library (collections, fs, io, math, string, array, result) |
| `docs/` | ~5 docs | Architecture docs |
