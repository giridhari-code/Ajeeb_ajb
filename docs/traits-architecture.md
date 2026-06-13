# Traits Architecture

## Overview
Add trait-based interfaces to Ajeeb, enabling polymorphism without class inheritance. Traits define method signatures; `impl` blocks provide implementations for specific types.

## Syntax

### Trait Definition
```
trait Foo {
    function bar(self: Self, x: Int) -> Int;
    function baz(self: Self) -> Void;
}
```

### Impl Block
```
impl Foo for MyStruct {
    function bar(self: MyStruct, x: Int) -> Int { ... }
    function baz(self: MyStruct) -> Void { ... }
}
```

## AST Changes

### token.rs — New Tokens
- `Trait` — keyword `trait`
- `Impl` — keyword `impl`
- `For` — keyword `for` (used in `impl Trait for Type`)

### ast.rs — New Nodes

```rust
Stmt::TraitDef {
    name: String,
    methods: Vec<TraitMethod>, // fn signatures with no body
    pub_: bool,
    line: usize,
    col: usize,
}

Stmt::ImplBlock {
    trait_name: String,
    type_name: String,       // the struct/enum implementing the trait
    methods: Vec<Stmt>,      // FnDef bodies
    line: usize,
    col: usize,
}

TraitMethod {               // fn signature within a trait
    name: String,
    params: Vec<(String, TypeAnnot)>,
    return_type: TypeAnnot,
}
```

### Stmt enum
Add `TraitDef { .. }` and `ImplBlock { .. }` variants.

## Parser Changes (parser.rs)

### `parse_statement`
- Match `Token::Trait` → call `parse_trait_def(pub_)`
- Match `Token::Impl` → call `parse_impl_block()`

### `parse_trait_def`
```
trait Foo { ... }
```
1. Consume `trait` keyword
2. Parse name identifier
3. Enter `{ ... }`
4. Parse `function` signatures (name, params, return type) — no body, ends with `;`
5. Store as `TraitMethod` vec

### `parse_impl_block`
```
impl TraitName for TypeName { ... }
```
1. Consume `impl` keyword
2. Parse trait name
3. Expect `for` keyword
4. Parse type name (identifier)
5. Enter `{ ... }`
6. Parse method definitions (`function` with full bodies)
7. Wrap in `Stmt::ImplBlock`

## Semantic Analyzer Changes (semantic.rs)

### New Fields
```rust
traits: HashMap<String, Vec<TraitMethod>>,
impls: HashMap<String, Vec<(String, Vec<Stmt>)>>, // type_name -> [(trait_name, methods)]
```

### First Pass (Signature Collection)
- `Stmt::TraitDef` → register in `self.traits`
- `Stmt::ImplBlock` → register in `self.impls`
- Check trait exists (error if impl references unknown trait)
- Check type exists (error if impl references unknown struct/enum)
- Check all trait methods are implemented (error if missing)
- Check no extra methods (warning or error)

### Body Checking
- `Stmt::ImplBlock` → check each method body using existing `check_stmt` logic
- Methods have access to `self: TypeName` as first parameter

## Evaluator Changes (eval.rs)

### Method Dispatch via Traits
When a `MethodCall` is evaluated on a struct/enum value:
1. Check if the object's type has an `impl` for the trait containing the method
2. Look up `mangled_name = type_name + "_" + trait_name + "_" + method`
3. Call the function with `self` as first arg

Alternatively, register trait methods with mangled names at startup:
```
StructName_TraitName_methodName
```

### Registration in `evaluate_program`
For `Stmt::ImplBlock`, register each method with mangled name:
```rust
let mangled = format!("{}_{}_{}", type_name, trait_name, method_name);
self.functions.insert(mangled, (params, body, return_type));
```

### Method Call Resolution
In `eval_expr` for `MethodCall`:
1. Evaluate `obj` → get RuntimeValue
2. Determine type name (from StructInstance.name, ClassInstance.class_name, or EnumVariant.enum_name)
3. Look for method in: (a) class methods, (b) trait impl methods
4. For trait impls, try mangled name `{type_name}_{trait_name}_{method}` — but we need to know which trait. A simpler approach: iterate over all registered impls for the type and try to find the method.

## Formatter Changes (formatter.rs)

### `format_stmt`
Add:
```rust
Stmt::TraitDef { name, methods, .. } => self.format_trait_def(name, methods)
Stmt::ImplBlock { trait_name, type_name, methods, .. } => self.format_impl_block(trait_name, type_name, methods)
```

### `format_trait_def`
```
trait Name {
    function method(self: Type, x: Int) -> Int;
}
```

### `format_impl_block`
```
impl TraitName for TypeName {
    function method(self: Type, x: Int) -> Int { ... }
}
```

## Implementation Order
1. Add `Trait`, `Impl`, `For` tokens to token.rs + lexer.rs
2. Add `Stmt::TraitDef`, `Stmt::ImplBlock` to ast.rs
3. Add `parse_trait_def`, `parse_impl_block` to parser.rs
4. Add trait registration + checking to semantic.rs
5. Add trait method dispatch to eval.rs
6. Add formatting to formatter.rs
7. Write tests
