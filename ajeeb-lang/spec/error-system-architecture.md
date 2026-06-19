# Error System Architecture

## Overview
Add built-in `Option[T]` and `Result[T,E]` types, pattern exhaustiveness checking, and runtime stack traces on panic.

## Part A: Option[T] and Result[T,E] as Standard Library Types

### Option[T] (packages/ajeeb-std/option.ajb)
```
pub enum Option[T] {
    Some(T),
    None,
}
```

### Result[T,E] (packages/ajeeb-std/result.ajb)
```
pub enum Result[T,E] {
    Ok(T),
    Err(E),
}
```

These work with existing generic enum support. No compiler changes needed for the types themselves — they are pure library code.

## Part B: Pattern Exhaustiveness Checking

### Current State
The semantic analyzer does NOT check if match arms cover all variants. A match with only `Some(_)` on an `Option[T]` compiles silently.

### Required Changes (semantic.rs)

#### Exhaustiveness Check in `infer_expr_type` for `Match`
When matching on an enum value:
1. Determine the enum type from the matched value
2. Get all variants of that enum from `self.enum_defs`
3. Collect which variants are covered by match arms
4. Check if any variant is missing (error if so)
5. If a wildcard `_` arm exists, any missing variants are OK

#### Algorithm
```rust
fn check_exhaustiveness(&self, value_ty: &TypeAnnot, arms: &[MatchArm], line: usize, col: usize) {
    if let TypeAnnot::Class(enum_name) = value_ty {
        if let Some(variants) = self.enum_defs.get(enum_name) {
            let mut covered: HashSet<&str> = HashSet::new();
            let mut has_wildcard = false;
            
            for arm in arms {
                match &arm.pattern {
                    Pattern::EnumVariant { variant, .. } => { covered.insert(variant.as_str()); }
                    Pattern::Wildcard => { has_wildcard = true; }
                    _ => {}
                }
            }
            
            if !has_wildcard {
                for variant in variants {
                    if !covered.contains(variant.name.as_str()) {
                        self.errors.push(error!("Non-exhaustive pattern: missing variant '{}'", variant.name));
                    }
                }
            }
        }
    }
}
```

## Part C: Runtime Stack Traces

### Current State
When a panic occurs (e.g., division by zero, unknown variable), the evaluator prints an error message but provides no call stack information.

### Required Changes (eval.rs)

#### FrameInfo Struct
```rust
struct FrameInfo {
    function_name: String,
    line: usize,
    col: usize,
}
```

#### Evaluator Changes
Add to `Evaluator`:
```rust
call_stack: Vec<FrameInfo>,
```

#### Push/Pop Frames
In `exec_fn_call_body` (around function invocation):
```rust
self.call_stack.push(FrameInfo {
    function_name: name.to_string(),
    line: current_line,
    col: current_col,
});
// ... execute function body ...
self.call_stack.pop();
```

#### Display Stack Trace
Create helper:
```rust
fn print_stack_trace(&self) {
    eprintln!("\nStack Trace (from top to bottom):");
    for (i, frame) in self.call_stack.iter().rev().enumerate() {
        eprintln!("  {}: {} (line {}, col {})", i, frame.function_name, frame.line, frame.col);
    }
}
```

#### Hook into Panic Points
Call `print_stack_trace()` when:
- Division by zero (eval.rs line 278)
- Unknown variable (eval.rs line 264)
- Unknown function (eval.rs line 1469)
- Array index out of bounds
- Any other runtime error

### Implementation Order
1. Add `call_stack` to `Evaluator`
2. Add `FrameInfo` struct
3. Push/pop frames in `exec_fn_call_body`
4. Add `print_stack_trace` method
5. Call it at error points
6. Write stack trace test

## Testing

### Option Tests (test_option.ajb)
```
import std::option;

let x: Option[Int] = Option::Some(42);
match x {
    Option::Some(val) => assert_eq(val, 42),
    Option::None => assert_eq(0, 1),
}
```

### Result Tests (test_result.ajb)
```
import std::result;

let x: Result[Int, String] = Result::Ok(42);
match x {
    Result::Ok(val) => assert_eq(val, 42),
    Result::Err(msg) => assert_eq(0, 1),
}
```

### Exhaustiveness Test (test_exhaustive.ajb)
Tests that missing a variant in a match produces a semantic error.

### Stack Trace Tests (test_stacktrace.ajb)
Tests that a runtime error prints a stack trace with function names and line numbers.

## Migration Path
1. Create `packages/ajeeb-std/option.ajb` and `packages/ajeeb-std/result.ajb` as standard library files
2. Add exhaustiveness checking in semantic analyzer
3. Add stack trace support in evaluator
4. Write tests for all three features
