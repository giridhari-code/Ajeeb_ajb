# Ajeeb LLVM Backend

The LLVM backend compiles Ajeeb source directly to LLVM IR. It operates on AST nodes (or optionally on MIR), emitting textual LLVM IR that is then compiled to native code via `llc`/`clang` or `gcc`.

## Architecture

```
Ajeeb AST ──→ Codegen::compile() ──→ LLVM IR string ──→ llc/clang ──→ native binary

Ajeeb AST ──→ THIR ──→ MIR ──→ Codegen::compile_mir() ──→ LLVM IR string
```

Source: `ajeebc/crates/ajeeb-compiler/src/llvm/`

| File | Lines | Responsibility |
|------|-------|---------------|
| `mod.rs` | 316 | `Codegen` struct, `compile()` entry, extern declarations |
| `expr.rs` | 761 | Expression emission (literals, binary ops, calls, indexing) |
| `stmt.rs` | 320 | Statement emission (if/else, while, for, assignments) |
| `mir.rs` | 772 | MIR-to-LLVM emission (`compile_mir`, `emit_mir_fn`) |
| `strings.rs` | 24 | Global string constant generation |
| `types.rs` | 310 | Variable type tracking for method dispatch |
| `methods.rs` | 14 | Method name resolution |
| `generic.rs` | 124 | Generic type/function monomorphization |

## 1. Entry Point

### AST Direct Compilation

```rust
pub fn compile(&mut self, stmts: &[Stmt]) -> Result<String, String>
```

The main entry point (`mod.rs:180`). Two-pass:

1. **First pass** — collects user-defined functions, struct/enum definitions, impl blocks, and method mangled names.
2. **Second pass** — emits code for all non-`main` functions, then wraps top-level statements (including `main` body) into `define i64 @main()`.

Top-level `set`/`const` statements become LLVM global variables (`@__ajb_global_<name>`). All values are `i64` — strings are pointer-as-int, arrays are heap-allocated, structs are heap-allocated arrays of `i64`.

Output is assembled as: `globals + functions + main_body`.

### MIR Compilation

```rust
pub fn compile_mir(&mut self, prog: &MirProgram) Result<String, String>
```

Entry for MIR-to-LLVM (`mir.rs:9`). Iterates `prog.functions`, calling `emit_mir_fn` for each. Generates a trivial `main` if none exists in the program.

## 2. MIR Instructions → LLVM IR

The MIR pipeline emits LLVM IR from basic blocks. Each `MirFn` becomes a `define i64 @name(...)` function.

### Statement Mapping

| MIR | LLVM IR | Notes |
|-----|---------|-------|
| `Assign { dest, value }` | `store i64 <val>, ptr <alloca>` | Or `mir_temps` for SSA temporaries |
| `Call { dest, func, args }` | `call i64 @func(i64 a0, i64 a1, ...)` | Struct constructors/getters/setters generated on demand |

### Rvalue Mapping

| MIR | LLVM IR |
|-----|---------|
| `Use(operand)` | load from variable alloca or temp |
| `Const(Int(n))` | `add i64 0, n` |
| `Const(Float(f))` | `add i64 0, <bitcast_bits>` |
| `Const(Str(s))` | `getelementptr` into `@.str.N` global, then `ptrtoint` |
| `Const Bool(b)` | `add i64 0, 1` or `add i64 0, 0` |
| `BinaryOp(Add, l, r)` | `add i64 l, r` (or `call @str_concat` if string) |
| `BinaryOp(Sub, l, r)` | `sub i64 l, r` |
| `BinaryOp(Mul, l, r)` | `mul i64 l, r` |
| `BinaryOp(Div, l, r)` | `sdiv` with divide-by-zero protection via `select` |
| `BinaryOp(Eq, l, r)` | `icmp eq i64 l, r` → `zext i1 to i64` |
| `BinaryOp(Neq, l, r)` | `icmp ne i64 l, r` → `zext` |
| `BinaryOp(Lt, l, r)` | `icmp slt i64 l, r` → `zext` |
| `BinaryOp(Gt, l, r)` | `icmp sgt i64 l, r` → `zext` |
| `BinaryOp(Le, l, r)` | `icmp sle i64 l, r` → `zext` |
| `BinaryOp(Ge, l, r)` | `icmp sge i64 l, r` → `zext` |
| `BinaryOp(And, l, r)` | `and i64 l, r` |
| `BinaryOp(Or, l, r)` | `or i64 l, r` |

Float operations: when either operand is a known float, `bitcast i64 to double`, then `fadd`/`fsub`/`fmul`/`fdiv`/`fcmp`, then `bitcast double back to i64`.

### Terminator Mapping

| MIR | LLVM IR |
|-----|---------|
| `Goto(target)` | `br label %mir_b<target>` |
| `SwitchInt { cond, targets, default }` | `icmp ne i64 <cond>, 0` → `br i1 %cmp, label %true_label, label %false_label` |
| `Return(Some(val))` | `ret i64 <val>` |
| `Return(None)` | `ret i64 0` |
| `Unreachable` | `unreachable` |

## 3. String Handling

Strings are represented as `i64` values holding raw pointers to null-terminated C strings.

### String Constants

Global string constants are emitted as LLVM `private unnamed_addr constant` arrays (`strings.rs`):

```
@.str.0 = private unnamed_addr constant [6 x i8] c"hello\00"
```

Non-printable bytes are escaped as `\XX` (two hex digits). The constant includes the null terminator.

### String Operations

String concatenation calls the C runtime:
```
%3 = call i64 @str_concat(i64 %1, i64 %2)
```

The backend tracks string values via `string_vars` (variable names) and `string_regs` (LLVM register names). When `BinOp::Add` has a string operand, it emits `@str_concat` instead of `add i64`.

### String Printing

`print`/`println` with a single string arg: `inttoptr i64 %val to ptr` → `@puts` or `@printf`.

### String Comparison

String `==` uses `strcmp_ajeeb` (calls into C runtime):
```
%cmp = call i64 @strcmp_ajeeb(i64 %s1, i64 %s2)
%result = icmp eq i64 %cmp, 0
```

**Important:** The `icmp eq i64` pointer comparison does NOT work for strings (different allocations for equal content). `strcmp_ajeeb` must be used.

### String Indexing

`__index(str, i)` → `call i64 @charCode(i64 %str, i64 %idx)`

`__index_assign(str, i, val)` → `call void @strSet(i64 %str, i64 %i, i64 %val)`

## 4. Array Handling

Arrays are heap-allocated via `malloc`. Layout: `[length_i64, elem0, elem1, ...]` — the first `i64` slot stores the length.

### Array Literal

`__array_lit(e0, e1, ...)` → calls into C runtime which allocates and fills the array.

### Array Indexing

`__index(arr, i)` → GEP with offset `i + 1` (skipping length prefix), then `load i64`:
```llvm
%offset = add i64 1, %idx
%elem_ptr = getelementptr inbounds i64, ptr %arr_ptr, i64 %offset
%val = load i64, ptr %elem_ptr
```

### Array Index Assignment

`__index_assign(arr, i, val)` → GEP + `store i64 %val, ptr %elem_ptr`.

### Array Length

`len(arr)` is redirected to `arr_len()` when the operand is a known array register.

### Array Printing

Arrays are printed via the C runtime function `array_to_string`.

## 5. Function Codegen

### User-Defined Functions

`emit_fn_def(name, params, body)` (`stmt.rs:42`):

1. Saves current `Codegen` state (unnamed count, type tracking sets).
2. Emits `define i64 @name(i64 %0, i64 %1, ...) {`.
3. For each parameter: `alloca i64`, `store` the incoming value.
4. Collects all `set`/`const` variables in the body via `collect_vars`, emits `alloca` for each.
5. Emits body statements.
6. Appends `ret i64 0` if no explicit return was emitted.

### Method Dispatch

Impl blocks register methods with mangled names:
- Inherent: `TypeName_methodName`
- Trait: `TypeName_TraitName_methodName`

The `resolve_method` function checks the `method_map` for the mangled name.

### Struct Operations

Struct constructors, getters, and setters are generated on demand during MIR codegen:

- **Constructor** (`__struct_Name`): `malloc` → store each field at GEP offset → return pointer as `i64`.
- **Getter** (`__struct_get_Name_field`): `inttoptr` → GEP at field offset → `load i64`.
- **Setter** (`__struct_set_Name_field`): `inttoptr` → GEP → `store i64`.

### Enum Operations

Enum variants are tagged with integer IDs (0, 1, 2, ...). Enum values are represented as pointers to a heap-allocated tuple of `(tag_i64, field0, field1, ...)`.

### Generic Monomorphization

Generic functions are stored in `generic_fns`. When called with concrete types, `subst_type_ann` and `subst_expr` produce a specialized copy which is then emitted as a new function.

## 6. Known Limitations

1. **All values are `i64`** — no native i32, i16, i8, or float register support. Floats are stored as `i64` bit patterns and `bitcast` to/from `double` on each operation.

2. **String equality must use `strcmp_ajeeb`** — `==` on string pointers compares addresses, not content. The codegen emits `strcmp_ajeeb` for string `==` in MIR mode, but the AST direct path uses `icmp eq i64`.

3. **No GC** — struct and array allocations via `malloc` are never freed.

4. **`__index` limitation** — array indexing with non-constant index expressions is lowered through a synthetic `__index` function call. Constant indices are folded directly.

5. **Struct fields are untyped at runtime** — all fields stored as `i64`. Nested structs (struct containing struct) store the inner pointer as `i64`.

6. **Float prints as integer** — `print(float_val)` uses `snprintf` with `%ld`, printing the raw bit pattern, not the decimal value.

7. **Single-threaded** — no concurrency support.

8. **`println` appends newline** — uses `@puts` which always adds `\n`.

## 7. Runtime Function Declarations

The backend lazily declares C runtime extern functions via `declare_extern(name)` (`mod.rs:131`). Declarations are emitted once into `self.globals`.

### Buffer Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `getStateBuf` | `i64 ()` | Returns pointer to 16KB state buffer |
| `getOutbuf` | `i64 ()` | Returns pointer to 64KB output buffer |

### String Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `len` | `i64 (i64)` | String length |
| `str_concat` | `i64 (i64, i64)` | Concatenate two strings |
| `strcmp_ajeeb` | `i64 (i64, i64)` | Compare strings (returns 0 if equal) |
| `substring` | `i64 (i64, i64, i64)` | Substring(str, start, len) |
| `indexOf` | `i64 (i64, i64, i64)` | IndexOf(str, sub, from) |
| `contains` | `i64 (i64, i64)` | Contains(str, sub) |
| `startsWith` | `i64 (i64, i64)` | StartsWith(str, prefix) |
| `endsWith` | `i64 (i64, i64)` | EndsWith(str, suffix) |
| `replace` | `i64 (i64, i64, i64)` | Replace(str, old, new) |
| `toUpperCase` | `i64 (i64)` | Uppercase string |
| `toLowerCase` | `i64 (i64)` | Lowercase string |
| `trim` | `i64 (i64)` | Trim whitespace |
| `charCode` | `i64 (i64, i64)` | Char code at index |
| `chr` | `i64 (i64, i64)` | Character from code |
| `strSet` | `void (i64, i64, i64)` | Set character at index |
| `getStr` | `i64 (i64)` | Get string from buffer |

### Numeric Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `itoa` | `i64 (i64)` | Integer to string |
| `arr_len` | `i64 (i64)` | Array length |

### I/O Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `readArg` | `i64 (i64)` | Read command-line argument |
| `readFile` | `i64 (i64)` | Read file to string |
| `writeFile` | `void (i64, i64)` | Write string to file |
| `writeAppend` | `void (i64, i64)` | Append string to file |
| `writeByte` | `void (i64, i64)` | Write single byte |
| `getInt` | `i64 (i64, i64)` | Read integer from buffer |
| `setInt` | `void (i64, i64, i64)` | Write integer to buffer |

### Memory Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `malloc` | `ptr (i64)` | Allocate memory |
| `free` | `void (ptr)` | Free memory |
| `allocBuf` | `i64 (i64)` | Allocate N+1 zero-initialized bytes from arena |

### System Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `exec` | `i64 (i64)` | Run shell command, return exit code |
| `mkdir` | `i64 (i64)` | Create directory (with parents) |
| `exit` | `void (i32)` | Exit process |

### Array Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `array_to_string` | `i64 (i64, i64)` | Convert array to printable string |

### Library Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `lib_open` | `i64 (i64)` | dlopen a shared library |
| `lib_sym` | `i64 (i64, i64)` | dlsym from library handle |

### Network Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `tcp_listen` | `i64 (i64)` | Listen on port |
| `tcp_accept` | `i64 (i64)` | Accept TCP connection |
| `tcp_connect` | `i64 (i64, i64)` | Connect to host:port |
| `tcp_read` | `i64 (i64, i64)` | Read from TCP socket |
| `tcp_write` | `void (i64, i64)` | Write to TCP socket |
| `tcp_close` | `void (i64)` | Close TCP socket |
| `tls_connect` | `i64 (i64)` | TLS connect |
| `tls_read` | `i64 (i64)` | TLS read |
| `tls_write` | `void (i64, i64)` | TLS write |
| `tls_close` | `void (i64)` | TLS close |
| `dns_lookup` | `i64 (i64)` | DNS lookup |

### C Standard Library

| Function | Signature | Description |
|----------|-----------|-------------|
| `puts` | `i32 (ptr)` | Print string with newline |
| `printf` | `i32 (ptr, ...)` | Formatted print |
| `snprintf` | `i32 (ptr, i64, ptr, ...)` | Formatted print to buffer |
| `fprintf` | `i32 (ptr, ptr, ...)` | Print to file handle |

### Global Buffers

```
@__ajeeb_buf  = global [16384 x i8] zeroinitializer   # 16KB general buffer
@__ajeeb_outbuf = global [65536 x i8] zeroinitializer  # 64KB output buffer
@stderr       = external global ptr                      # stderr file handle
```
