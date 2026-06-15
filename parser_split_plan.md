# Parser Modularization Plan

## Overview

Split `crates/ajeeb-compiler/src/parser.rs` (1915 lines) into a `parser/` directory with sub-modules, following the same pattern used by the LLVM backend (`src/llvm/`). The struct definition and all `impl Parser` blocks remain; each sub-module extends `Parser` with additional methods.

**Pattern (copy from `src/llvm/`):**
```rust
// sub_module.rs
use super::Parser;
impl Parser { pub(super) fn method(&mut self) -> Result<..., CompileError> { ... } }
```

---

## Phase 1: Audit — Function Categorization

### 1. Core Parser Infrastructure (13 functions, ~134 lines)
| Function | Lines | Visibility | Purpose |
|---|---|---|---|
| `Parser::new` | 18-31 | `pub` | Constructor from tokens |
| `Parser::with_positions` | 33-44 | `pub` | Constructor with line/col info |
| `Parser::line` | 46-51 | private | Current line number |
| `Parser::col` | 53-58 | private | Current column number |
| `Parser::err_at` | 60-62 | private | Build error with line/col |
| `Parser::err` | 64-66 | private | Build error at current pos |
| `Parser::peek` | 68-70 | private | Look at current token |
| `Parser::peek_next` | 72-74 | private | Look ahead one token |
| `Parser::advance` | 76-82 | private | Consume and return current token |
| `Parser::expect` | 84-102 | private | Consume expected token |
| `Parser::expr_pos` | 104-131 | private (associated fn) | Extract line/col from Expr |
| `Parser::token_debug` | 133-195 | private | Token → debug string |
| `Parser::peek_type_args` | 1844-1887 | private | Heuristic: `[...]` contains type args? |

### 2. Expression Parsing (12 functions, ~615 lines)
| Function | Lines | Purpose |
|---|---|---|
| `parse_expression` | 1119-1124 | Entry: match expr or assignment |
| `parse_match_expr` | 1126-1159 | `match val { pat => body, ... }` |
| `parse_pattern` | 1161-1218 | Pattern parsing (called from match) |
| `parse_assignment` | 1220-1263 | `lhs = rhs` |
| `parse_or` | 1265-1281 | `||` |
| `parse_and` | 1283-1299 | `&&` |
| `parse_equality` | 1301-1320 | `==`, `!=` |
| `parse_comparison` | 1322-1347 | `<`, `>`, `<=`, `>=` |
| `parse_term` | 1349-1368 | `+`, `-` |
| `parse_factor` | 1370-1389 | `*`, `/` |
| `parse_unary` | 1391-1407 | `-expr`, `!expr` |
| `parse_primary` | 1433-1840 | Literals, idents, calls, fields, index |
| `parse_struct_lit` | 1409-1431 | `Name { field: val, ... }` (called from parse_primary) |
| `parse_call_args` | 1903-1914 | `(arg1, arg2, ...)` |
| `parse_type_arg_list` | 1890-1901 | `[Type1, Type2]` |

### 3. Statement Parsing (12 functions, ~370 lines)
| Function | Lines | Purpose |
|---|---|---|
| `parse_program` | 274-285 | Top-level: @import phase then statement phase |
| `parse_pub` | 287-294 | Optional `pub` keyword |
| `parse_statement` | 296-338 | Top-level dispatch |
| `parse_import` | 340-371 | `import path::path;` |
| `parse_at_import` | 373-403 | `@import path.path;` (file-start only) |
| `parse_let_decl` | 405-421 | `let name: type = expr;` |
| `parse_const_decl` | 423-439 | `const name: type = expr;` |
| `parse_if_stmt` | 441-471 | `if (cond) { ... } else { ... }` |
| `parse_while_stmt` | 473-484 | `while (cond) { ... }` |
| `parse_for_stmt` | 486-524 | `for (init; cond; update) { ... }` |
| `parse_return_stmt` | 1089-1101 | `return expr;` or `return;` |
| `parse_expr_stmt` | 1103-1109 | Expression as statement |
| `parse_block` | 1111-1117 | `{ stmt; stmt; ... }` |

### 4. Declaration Parsing (8 functions, ~552 lines)
| Function | Lines | Purpose |
|---|---|---|
| `parse_fn_def` | 526-624 | `fn name[T: Bound](params) -> RetType { body }` |
| `parse_param_name` | 835-841 | Parameter name (ident or `self`) |
| `parse_class_def` | 626-665 | `class Name { fields methods }` |
| `parse_struct_def` | 667-735 | `struct Name[T] { field: type, ... }` |
| `parse_enum_def` | 737-833 | `enum Name[T] { Variant, Variant(type), ... }` |
| `parse_trait_def` | 843-934 | `trait Name[T: Bound] { fn sig(); ... }` |
| `parse_impl_block` | 936-1087 | `impl[T] Trait for Type { ... }` or `impl Type { ... }` |

### 5. Type Parsing (3 functions, ~76 lines)
| Function | Lines | Purpose |
|---|---|---|
| `parse_type` | 197-209 | `: type` or `-> type` (with prefix) |
| `parse_type_postfix` | 213-244 | Handle `[]` and `[Args]` postfix |
| `parse_single_type` | 246-272 | Parse base type name/int/float/bool/void |

---

## Phase 2: Modularization Design

### Recommended File Layout

```
parser/
├── mod.rs        --  struct Parser definition + new/with_positions
│                       + infrastructure (line/col/err/peek/advance/expect/expr_pos/token_debug)
│                       + re-exports from sub-modules
│                       + peek_type_args (needs direct token array access)
│
├── stmt.rs       --  parse_program, parse_pub, parse_statement
│                       parse_block, parse_expr_stmt
│                       parse_let_decl, parse_const_decl
│                       parse_if_stmt, parse_while_stmt, parse_for_stmt
│                       parse_return_stmt
│                       parse_import, parse_at_import
│                       (~370 lines)
│
├── expr.rs       --  parse_expression, parse_assignment
│                       parse_or, parse_and, parse_equality, parse_comparison
│                       parse_term, parse_factor, parse_unary, parse_primary
│                       parse_struct_lit, parse_call_args, parse_type_arg_list
│                       parse_match_expr, parse_pattern
│                       (~615 lines — the largest file)
│
├── decls.rs      --  parse_fn_def, parse_param_name
│                       parse_class_def, parse_struct_def, parse_enum_def
│                       parse_trait_def, parse_impl_block
│                       (~552 lines)
│
└── types.rs      --  parse_type, parse_type_postfix, parse_single_type
│                       (~76 lines)
```

### Rationale for 5 files vs. 10 files

The suggested 10-file layout creates 4 singleton-function files (`structs.rs`, `enums.rs`, `traits.rs`, `impls.rs`) and several very small files. The 5-file layout groups them into meaningful units:

| File | Lines | Rationale |
|---|---|---|
| `mod.rs` | ~195 | Struct def + infrastructure. Stays small. |
| `stmt.rs` | ~370 | All statement-level parsers. Cohesive unit. |
| `expr.rs` | ~615 | All expression/precedence parsers. Natural boundary. |
| `decls.rs` | ~552 | All top-level declarations (fn/class/struct/enum/trait/impl). These share generic param handling and call `parse_fn_def`. |
| `types.rs` | ~76 | Type annotation parsing. Called by everything. |

### Alternative: Keep the 10-file layout (if preferred)

```
parser/
├── mod.rs        -- struct def + infrastructure + re-exports
├── expr.rs       -- expression + pattern + match
├── stmt.rs       -- statements + import
├── types.rs      -- type parsing
├── structs.rs    -- parse_struct_def (68 lines)
├── enums.rs      -- parse_enum_def (96 lines)
├── traits.rs     -- parse_trait_def (91 lines)
├── impls.rs      -- parse_impl_block (151 lines)
├── modules.rs    -- parse_program, parse_import, parse_at_import (72 lines)
└── patterns.rs   -- parse_pattern, parse_match_expr (90 lines)
```

**Downside:** 7 files are <100 lines each; 5 files in the recommended layout are more balanced.

---

## Phase 3: Dependency Analysis

### Function Dependency Graph

```
parse_program
  ├── parse_at_import
  └── parse_statement
        ├── parse_import
        ├── parse_pub
        ├── parse_let_decl     ──→ parse_type, parse_expression
        ├── parse_const_decl   ──→ parse_type, parse_expression
        ├── parse_if_stmt      ──→ parse_expression, parse_block → parse_statement
        ├── parse_while_stmt   ──→ parse_expression, parse_block → parse_statement
        ├── parse_for_stmt     ──→ parse_let_decl, parse_expression, parse_block
        ├── parse_fn_def       ──→ parse_type, parse_param_name, parse_block, [generic_type_params state]
        ├── parse_return_stmt  ──→ parse_expression
        ├── parse_class_def    ──→ parse_fn_def, parse_pub, parse_type, [current_class state]
        ├── parse_struct_def   ──→ parse_type, [generic_type_params state]
        ├── parse_enum_def     ──→ [generic_type_params state]
        ├── parse_trait_def    ──→ parse_type, parse_param_name, [generic_type_params state]
        ├── parse_impl_block   ──→ parse_fn_def, parse_type, [generic_type_params/bounds state]
        ├── parse_match_expr   ──→ parse_expression, parse_pattern
        ├── parse_expr_stmt    ──→ parse_expression
        └── break/continue     ──→ (trivial)

parse_expression
  └── parse_match_expr ──→ parse_expression, parse_pattern
  └── parse_assignment  ──→ parse_or → parse_and → parse_equality → parse_comparison
                            → parse_term → parse_factor → parse_unary → parse_primary
                              ├── parse_struct_lit ──→ parse_expression
                              ├── parse_call_args  ──→ parse_expression
                              ├── parse_type_arg_list ──→ parse_type_postfix
                              └── peek_type_args

parse_type ──→ parse_type_postfix ──→ parse_single_type
                              ↕ (called from parse_fn_def, parse_let_decl, parse_struct_def, etc.)
```

### Cross-Module Call Analysis

| Caller | Callee | Risk if separate | Mitigation |
|---|---|---|---|
| `parse_statement` (stmt.rs) | `parse_expression` (expr.rs) | Low — public method | `impl Parser` in expr.rs, visible from stmt.rs |
| `parse_statement` (stmt.rs) | `parse_fn_def` (decls.rs) | Low | Same visibility pattern |
| `parse_statement` (stmt.rs) | `parse_type` (types.rs) | Low | Same |
| `parse_primary` (expr.rs) | `parse_type_arg_list` (expr.rs) | None — same file | Already grouped together |
| `parse_fn_def` (decls.rs) | `parse_type` (types.rs) | Low | Same |
| `parse_fn_def` (decls.rs) | `parse_block` → `parse_statement` | Medium — circular | `decls.rs` calls into `stmt.rs`; `stmt.rs` dispatches to `decls.rs` |
| `parse_class_def` (decls.rs) | `parse_fn_def` (decls.rs) | None — same file | Already grouped together |
| `parse_struct_def` (decls.rs) | `parse_type` (types.rs) | Low | Same |
| `parse_enum_def` (decls.rs) | (inline type parsing) | None | Uses `advance()` directly for inline type fields |
| `parse_impl_block` (decls.rs) | `parse_fn_def` (decls.rs) | None | Same file |
| `parse_trait_def` (decls.rs) | `parse_param_name`, `parse_type` | Low | Both: one in decls.rs, one in types.rs |
| `parse_type_arg_list` (expr.rs) | `parse_type_postfix` (types.rs) | Low | |
| `parse_match_expr` (expr.rs) | `parse_pattern` (expr.rs) | None — same file | Already grouped together |
| `parse_for_stmt` (stmt.rs) | `parse_let_decl`, `parse_expr_stmt` (stmt.rs) | None — same file | Already grouped together |

### Circular Dependency

The critical circular dependency:
- **`decls.rs` → `stmt.rs`**: `parse_fn_def` calls `parse_block` which is in stmt.rs
- **`stmt.rs` → `decls.rs`**: `parse_statement` dispatches to `parse_fn_def`, `parse_class_def`, etc.

**This is not a problem** because all modules extend the same `Parser` struct via `impl Parser` blocks. Rust allows `impl Parser` blocks to be spread across files without circular import issues.

### Shared Mutable State (Risk Assessment)

| State Field | Type | Modified By | Extraction Risk |
|---|---|---|---|
| `pos` | `usize` | `advance`, `expect` | None — always via self |
| `var_types` | `HashMap<String, TypeAnnot>` | `parse_let_decl`, `parse_const_decl` | Low — stmt.rs only |
| `current_class` | `Option<String>` | `parse_class_def` | Low — decls.rs only |
| `generic_type_params` | `Vec<String>` | `parse_fn_def`, `parse_struct_def`, `parse_enum_def`, `parse_trait_def`, `parse_impl_block` | **Medium** — pushed/popped across 5 functions in decls.rs |
| `generic_type_bounds` | `HashMap<String, Vec<String>>` | `parse_fn_def`, `parse_impl_block`, `parse_struct_def`, `parse_enum_def`, `parse_trait_def` | **Medium** — pushed/popped, must stay in sync with type_params |

All state is on `self` — no separate struct needed. Extraction risk is low because all state lives on `Parser` and each `impl Parser` block in sub-modules accesses it through `&mut self`.

---

## Phase 4: Safe Extraction Order

Extract in order **from lowest risk to highest risk**. After each step, verify `cargo test` + `bash tests/bootstrap_check.sh`.

### Step 0: Create `parser/` directory + `mod.rs` skeleton
- Rename `parser.rs` → `parser/mod.rs`
- Move infrastructure methods into `mod.rs`
- Add `pub mod stmt; pub mod expr; pub mod decls; pub mod types;`
- **Risk: None** — single file rename

### Step 1: Extract `types.rs` (3 functions, ~76 lines)
- `parse_type`, `parse_type_postfix`, `parse_single_type`
- These depend only on `advance`, `peek`, `peek_next`, `err`, `generic_type_params`
- No functions outside `types.rs` call these through any path other than `self.parse_type()`
- **Migration risk: Low**

### Step 2: Extract `expr.rs` (15 functions, ~615 lines)
- `parse_expression`, `parse_assignment`, `parse_or`, `parse_and`, `parse_equality`, `parse_comparison`, `parse_term`, `parse_factor`, `parse_unary`, `parse_primary`, `parse_struct_lit`, `parse_call_args`, `parse_type_arg_list`, `parse_match_expr`, `parse_pattern`, `peek_type_args`
- These call `self.parse_type_postfix()` (now in `types.rs`) and `self.parse_expression()` (recursive within same file)
- `peek_type_args` uses `self.tokens` directly — needs `use super::Token`
- **Migration risk: Low-Medium** (large but mechanical)

### Step 3: Extract `stmt.rs` (12 functions, ~370 lines)
- `parse_program`, `parse_pub`, `parse_statement`, `parse_block`, `parse_expr_stmt`, `parse_let_decl`, `parse_const_decl`, `parse_if_stmt`, `parse_while_stmt`, `parse_for_stmt`, `parse_return_stmt`, `parse_import`, `parse_at_import`
- These call `self.parse_expression()` (expr.rs), `self.parse_type()` (types.rs), `self.parse_fn_def()` (decls.rs)
- **Migration risk: Low** (entry point stays, dispatches to all sub-modules)

### Step 4: Extract `decls.rs` (8 functions, ~552 lines)
- `parse_fn_def`, `parse_param_name`, `parse_class_def`, `parse_struct_def`, `parse_enum_def`, `parse_trait_def`, `parse_impl_block`
- These call `self.parse_type()` (types.rs), `self.parse_block()` → `self.parse_statement()` (stmt.rs)
- Shared generic_type_params/bounds push/pop logic is entirely within this file
- **Migration risk: Medium** — careful with state management on generic params

### Step 5: Clean up `mod.rs`
- Remove any functions accidentally left behind (everything should be in sub-modules)
- Add `pub use` re-exports for the public API (`parse_program`, `new`, `with_positions`)
- Add doc comments to each sub-module
- **Risk: None**

---

## Phase 5: Verification Plan

### After Every Extraction Step

```bash
cargo test
bash tests/bootstrap_check.sh
```

### What to Verify

1. **cargo test passes** (all 16 Rust tests + all Ajeeb interpreter tests)
2. **bootstrap_check.sh succeeds**:
   - `output.c` compiles
   - `output2.c` = `output.c` (byte-for-byte identical)
   - SHA256 of both files identical
3. **AST unchanged** — no change to `ast.rs` (guaranteed since we're only moving methods)
4. **No public API change** — `Parser` still exposes `new()`, `with_positions()`, `parse_program()`

### Rollback Procedure

If any step breaks:
1. Revert the last extraction: `git checkout -- src/parser/`
2. Restore from working backup: `git checkout sources/parser.rs.bak`
3. Investigate: `git diff` to find what was lost

### Final State

After all 5 steps:
- `parser.rs` deleted
- `parser/mod.rs` — 195 lines (struct + infrastructure)
- `parser/types.rs` — 76 lines
- `parser/expr.rs` — 615 lines
- `parser/stmt.rs` — 370 lines
- `parser/decls.rs` — 552 lines
- Total: ~1808 lines (slight reduction from 1915 due to `pub mod` + `use super::Parser` boilerplate)

### Updating Other Files

These files reference `parser::Parser` and need no changes:
- `src/lib.rs` — change `pub mod parser;` → `pub mod parser;` (same, now a directory)
- `src/module.rs` — `use crate::parser::Parser;` unchanged
- `src/main.rs` — `use parser::Parser;` unchanged

---

## Migration Risk Summary

| Step | Risk | Lines | Verification |
|---|---|---|---|
| Step 0: Create dir + mod.rs | None | 0 | File rename only |
| Step 1: types.rs | Low | 76 | `cargo test` + bootstrap |
| Step 2: expr.rs | Low-Med | 615 | `cargo test` + bootstrap |
| Step 3: stmt.rs | Low | 370 | `cargo test` + bootstrap |
| Step 4: decls.rs | Medium | 552 | `cargo test` + bootstrap |
| Step 5: Cleanup mod.rs | None | 0 | `cargo test` + bootstrap |

**Total: ~1915 lines split into 5 files with zero behavior change and full bootstrap verification after each step.**
