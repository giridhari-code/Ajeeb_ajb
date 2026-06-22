# Ajeeb MIR (Mid-level Intermediate Representation)

MIR is a CFG-based (Control Flow Graph) intermediate representation that sits between HIR (High-level IR) and LLVM IR codegen. It is a simplified, SSA-like representation with explicit basic blocks and terminators.

Source: `ajeebc/crates/ajeeb-compiler/src/mir.rs` (253 lines), `ajeebc/crates/ajeeb-compiler/src/thir_to_mir.rs` (636 lines).

## 1. MIR Instruction Set

### Data Types

All MIR values are untyped at the representation level — the `HirType` annotations are carried along for codegen but operations don't enforce them. Values flow through named temporaries (`t0`, `t1`, ...) or variable names.

#### `MirProgram`

```rust
struct MirProgram {
    functions: Vec<MirFn>,
    structs: Vec<(String, Vec<(String, HirType)>)>,  // (name, fields)
    enums: Vec<(String, Vec<(String, Vec<HirType>)>)>, // (name, variants)
}
```

#### `MirFn`

```rust
struct MirFn {
    name: String,
    params: Vec<(String, HirType)>,
    return_type: HirType,
    blocks: Vec<BasicBlock>,
    locals: Vec<(String, HirType)>,
}
```

#### `BasicBlock`

```rust
struct BasicBlock {
    id: usize,
    statements: Vec<MirStmt>,
    terminator: Terminator,
}
```

Every basic block ends with exactly one terminator. Blocks are identified by their index in the `blocks` vector.

### Statements (`MirStmt`)

| Variant | Syntax | Description |
|---------|--------|-------------|
| `Assign { dest, value }` | `dest = value` | Assign an rvalue to a destination (variable or temp) |
| `Call { dest, func, args }` | `dest = func(args...)` | Call a function, optionally store result |

`dest` in `Call` is `Option<String>` — `None` for void calls (e.g., `writeFile`).

### Rvalues (`MirRvalue`)

| Variant | Description |
|---------|-------------|
| `Use(operand)` | Copy/forward an operand value |
| `BinaryOp(op, left, right)` | Binary operation on two operands |
| `Const(c)` | Literal constant |

### Operands (`MirOperand`)

| Variant | Description |
|---------|-------------|
| `Var(String)` | Named variable or temporary (e.g., `"x"`, `"t3"`) |
| `Constant(MirConst)` | Inline constant value |

### Constants (`MirConst`)

| Variant | Type |
|---------|------|
| `Int(i64)` | 64-bit signed integer |
| `Float(f64)` | 64-bit float |
| `Str(String)` | String literal |
| `Bool(bool)` | Boolean (`true`/`false`) |

### Binary Operators (`MirBinOp`)

| Op | Description | LLVM equivalent |
|----|-------------|-----------------|
| `Add` | Addition (integer, float, or string concat) | `add i64` / `fadd double` / `@str_concat` |
| `Sub` | Subtraction | `sub i64` / `fsub double` |
| `Mul` | Multiplication | `mul i64` / `fmul double` |
| `Div` | Division (safe — returns 0 on div-by-zero) | `sdiv i64` / `fdiv double` |
| `Eq` | Equality | `icmp eq i64` → `zext` / `fcmp oeq double` |
| `Neq` | Not equal | `icmp ne` / `fcmp une` |
| `Lt` | Less than | `icmp slt` / `fcmp olt` |
| `Gt` | Greater than | `icmp sgt` / `fcmp ogt` |
| `Le` | Less or equal | `icmp sle` / `fcmp ole` |
| `Ge` | Greater or equal | `icmp sge` / `fcmp oge` |
| `And` | Logical AND (bitwise on i64) | `and i64` |
| `Or` | Logical OR (bitwise on i64) | `or i64` |

### Terminators (`Terminator`)

Every basic block ends with exactly one terminator.

| Variant | Description |
|---------|-------------|
| `Goto(target)` | Unconditional jump to block `target` |
| `SwitchInt { cond, targets, default }` | Conditional branch: if `cond != 0`, go to `targets[0].1`; otherwise `default` |
| `Return(Option<MirOperand>)` | Return from function. `None` → `ret i64 0`; `Some(op)` → `ret i64 <val>` |
| `Unreachable` | Marks unreachable code (emits LLVM `unreachable`) |

`SwitchInt` is always used for conditionals (if/while/for). The `targets` vector always has exactly one entry `(1, true_block)` — the value `1` is a tag meaning "the true branch". The `default` is the false branch.

## 2. MIR Data Types (Type Tracking)

MIR itself is largely untyped — all values flow as `i64` in the LLVM backend. However, `HirType` annotations are preserved in:

- `MirFn.params` — parameter types
- `MirFn.return_type` — function return type
- `MirFn.locals` — local variable types
- `MirProgram.structs` — struct field types
- `MirProgram.enums` — enum variant field types

The LLVM codegen uses these annotations to:
- Determine if a parameter is a string (for `string_vars` tracking)
- Resolve method dispatch return types
- Generate struct constructors with correct field counts

## 3. How HIR Lowers to MIR

Source: `thir_to_mir.rs` — `MirBuilder` struct.

### Builder State

```rust
struct MirBuilder {
    current_blocks: Vec<BasicBlock>,   // completed blocks
    current_stmts: Vec<MirStmt>,       // statements for current in-progress block
    temp_counter: usize,               // generates t0, t1, t2, ...
    loop_stack: Vec<(usize, usize)>,   // (continue_target, break_target) per loop
    break_patches: Vec<usize>,         // blocks needing break target patched
    continue_patches: Vec<usize>,      // blocks needing continue target patched
    method_mangled_names: HashMap<String, Vec<String>>, // type_method → mangled names
}
```

### Lowering Entry

`build_program(hir)` iterates all HIR functions and impl methods, calling `build_fn` for each. Impl methods are mangled as `Type_method` or `Type_Trait_method`.

### Statement Lowering (`lower_stmt`)

| HIR | MIR |
|-----|-----|
| `Set { name, ty, value }` | `Assign { dest: name, value: Use(lower_expr(value)) }` + adds to locals |
| `Return(expr)` | `finish_block(Return(Some(lower_expr(expr))))` + starts unreachable block |
| `If { cond, then, else_ }` | `lower_if(...)` — creates SwitchInt + then/else/merge blocks |
| `While { cond, body }` | `lower_while(...)` — creates header/body/exit blocks with loop-back edge |
| `For { init, cond, update, body }` | `lower_for(...)` — init/header/body/update/exit blocks |
| `Expr(expr)` | `lower_expr(expr)` — discards result |
| `Break` | `finish_block(Goto(0))` — placeholder patched to exit block |
| `Continue` | `finish_block(Goto(0))` — placeholder patched to header/update block |

### Expression Lowering (`lower_expr`)

Expressions are lowered to `MirOperand` values, with side effects emitted as statements.

| HIR Expression | MIR Result |
|----------------|------------|
| `Int(n)` | `Constant(Int(n))` |
| `Float(f)` | `Constant(Float(f))` |
| `Str(s)` | `Constant(Str(s))` |
| `Bool(b)` | `Constant(Bool(b))` |
| `Var { name }` | `Var(name)` |
| `BinOp { op, left, right }` | `Assign { tN, BinaryOp(op, l, r) }` → `Var("tN")` (constant-folded if possible) |
| `Call { name, args }` | `Assign { tN, Call(name, args) }` → `Var("tN")` |
| `MethodCall { receiver, method, args }` | `Assign { tN, Call(mangled_name, [receiver, args...]) }` |
| `StructLit { name, fields }` | `Assign { tN, Call("__struct_name", field_values) }` |
| `FieldAccess { obj, field }` | `Assign { tN, Call("__struct_get_Type_field", [obj]) }` |
| `FieldAssign { obj, field, value }` | `Assign { tN, Call("__struct_set_Type_field", [obj, val]) }` |
| `ArrayLit { elems }` | `Assign { tN, Call("__array_lit", elem_values) }` |
| `Index { obj, idx }` | `Assign { tN, Call("__index", [obj, idx]) }` |
| `IndexAssign { obj, idx, value }` | `Assign { tN, Call("__index_assign", [obj, idx, val]) }` |
| `EnumCtor { enum_name, variant, args }` | `Assign { tN, Call("EnumName_Variant", args) }` |
| `UnaryMinus(inner)` | `Assign { tN, BinaryOp(Sub, Int(0), inner) }` |
| `UnaryNot(inner)` | `Assign { tN, BinaryOp(Eq, inner, Int(0)) }` |
| `Assign { name, value }` | `Assign { dest: name, value: Use(val) }` |

### If-Else Lowering (`lower_if`)

```
          ┌─────────────┐
          │  SwitchInt   │──cond==0──→ else_block (or merge if no else)
          │  cond        │
          └──────┬───────┘
                 │ cond!=0
                 ▼
          ┌─────────────┐
          │  then_block  │──Goto──→ merge_block
          └─────────────┘

          ┌─────────────┐
          │  else_block  │──Goto──→ merge_block  (if else exists)
          └─────────────┘
```

All `Goto(0)` placeholders within then/else are patched to `merge_block`.

### While Loop Lowering (`lower_while`)

```
pre_block ──Goto──→ header_block
                        │
                   SwitchInt cond
                   ┌────┴────┐
                   │         │
                   ▼         ▼
             body_block   exit_block
                   │
             Goto → header_block (loop-back)
```

Break targets are patched to `exit_block`. Continue targets are patched to `header_block`.

### For Loop Lowering (`lower_for`)

```
init_block ──Goto──→ header_block
                          │
                     SwitchInt cond
                     ┌────┴────┐
                     ▼         ▼
               body_block   exit_block
                     │
               Goto → update_block
                          │
                    update_stmt
                          │
                    Goto → header_block
```

Continue targets are patched to `update_block`. Break targets are patched to `exit_block`.

## 4. MIR Optimization

Source: `mir.rs:88-253` — `optimize_mir(prog)`.

Two optimization passes run on each function:

### Constant Folding (`constant_fold`)

Evaluates binary operations on two constant operands at compile time.

**Supported operations:**

| Op | Int×Int | Float×Float | Bool×Bool |
|----|---------|-------------|-----------|
| `Add` | `a + b` | `a + b` | — |
| `Sub` | `a - b` | `a - b` | — |
| `Mul` | `a * b` | `a * b` | — |
| `Div` | `a / b` (skip if `b==0`) | `a / b` (skip if `b==0.0`) | — |
| `Eq` | `a == b` | `a == b` | `a == b` |
| `Neq` | `a != b` | `a != b` | `a != b` |
| `Lt` | `a < b` | `a < b` | — |
| `Gt` | `a > b` | `a > b` | — |
| `Le` | `a <= b` | `a <= b` | — |
| `Ge` | `a >= b` | `a >= b` | — |
| `And` | — | — | `a && b` |
| `Or` | — | — | `a \|\| b` |

Division by zero is NOT folded (returns `None`, keeping the runtime check).

Constant folding also occurs inline during `lower_expr` in `thir_to_mir.rs:417-422`.

### Dead Block Elimination (`dead_block_elim`)

1. **Fix dangling references** — if any terminator targets a block index ≥ `blocks.len()`, creates `Unreachable` blocks up to that index. This handles cases where `lower_if` creates merge blocks that may reference non-existent blocks.

2. **Clamp targets** — all terminator targets are clamped to valid block indices as a safety net.

3. **Reachability analysis** — BFS from block 0, marking all reachable blocks.

4. **Remove unreachable blocks** — builds a remap table and rewrites all terminator targets to the new block indices. Block 0 is always kept.

5. **Renumber** — block IDs are updated to match new indices.
