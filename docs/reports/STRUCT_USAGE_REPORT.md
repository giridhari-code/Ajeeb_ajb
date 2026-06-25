# Struct Usage Report — P0-2 Scope

## Summary
- **73 struct definitions** found across repository
- **~15 struct patterns** identified for Stage A parity
- **Minimum required for P0-2**: struct declarations, field access, struct literals, field assignment

## Struct Syntax Patterns Found

### 1. Struct Declaration (comma-separated)
```
struct User {
    name: string,
    age: int
}
```
**Files**: struct_basic.ajb, struct_literal.ajb, struct_field_access.ajb, struct_field_assign.ajb, struct_nested.ajb, struct_verify.ajb, inherent_basic.ajb, inherent_method_call.ajb, inherent_new.ajb, inherent_multiple_methods.ajb, cross_backend_test.ajb, many trait tests

### 2. Struct Declaration (semicolon-separated)
```
struct Route {
    method: String;
    path: String;
    handler: String;
}
```
**Files**: ajeeb-http/mod.ajb, ajeeb-db/mod.ajb, ajeeb-web/mod.ajb

### 3. Struct Literal Construction (brace syntax)
```
set user = User { name: "Ajeeb", age: 1 };
```
**Files**: struct_basic.ajb, struct_literal.ajb, struct_field_access.ajb, struct_field_assign.ajb

### 4. Struct Literal with Variable Values
```
set route = Route { method: method, path: path, handler: handler };
```
**Files**: ajeeb-http/mod.ajb, ajeeb-db/mod.ajb

### 5. Struct Field Access (dot notation)
```
print(user.name);
print(itoa(p.x));
```
**Files**: struct_basic.ajb, struct_literal.ajb, struct_field_access.ajb

### 6. Nested Field Access
```
print(user.address.city);
```
**Files**: struct_nested.ajb

### 7. Struct Field Assignment
```
user.age = 2;
```
**Files**: struct_field_assign.ajb

### 8. Struct Variable Declarations
```
set user = User { name: "Ajeeb", age: 1 };
set p = Point { x: 10, y: 20 };
```
**Files**: All struct tests

### 9. Function Parameters with Struct Type
```
function point_distance(p: Point): int {
```
**Files**: cross_backend_test.ajb

### 10. Function Return Type as Struct
```
fn new(name: string) -> User {
    return User { name: name };
}
```
**Files**: inherent_new.ajb, inherent_method_call.ajb, inherent_multiple_methods.ajb

## P0-2 Scope (Required Features)

| Feature | Priority | Files Using It |
|---------|----------|----------------|
| `struct Name { field: type }` declaration | HIGH | 73 definitions |
| `set x = Name { field: value }` literal | HIGH | All struct tests |
| `x.field` dot notation access | HIGH | struct_basic, struct_literal |
| `x.field = value` assignment | HIGH | struct_field_assign |
| Comma separator in fields | HIGH | struct_basic, struct_literal, etc. |
| Semicolon separator in fields | MEDIUM | ajeeb-http, ajeeb-db |

## NOT in P0-2 Scope (Deferred)

| Feature | Deferred To | Files Using It |
|---------|-------------|----------------|
| `impl Type { ... }` blocks | P0-3 (methods) | inherent_basic, inherent_method_call |
| `::` associated function calls | P0-3 | inherent_new |
| `trait` definitions | P0-3 | trait_basic, trait_dispatch |
| `impl Trait for Type` | P0-3 | trait_dispatch |
| Method dispatch | P0-3 | inherent_basic |
| Generics | P1 | generic_*.ajb |
| Enums | P1 | enum_*.ajb |
| Array type `Type[]` fields | P1 | ajeeb-http/mod.ajb |
| Nested struct types | P1 | struct_nested.ajb |

## Success Criteria

All real struct usages in repository compile via the self-hosted compiler:
- `struct_basic.ajb` — struct declaration + literal + field access
- `struct_literal.ajb` — struct literal + field access via itoa()
- `struct_field_access.ajb` — field access + print
- `struct_field_assign.ajb` — field assignment

## Implementation Plan

1. **Lexer**: Add `struct` keyword (token type 49)
2. **Statement handler**: Parse `struct Name { field: type, ... }` and emit `typedef struct { type name; ... } StructName;`
3. **Expression handler**: Parse `Name { field: value, ... }` and emit `(StructName){ .field = value, ... }`
4. **Expression handler**: Track struct field names for dot-notation access
5. **Variable declarations**: Support struct type annotations
