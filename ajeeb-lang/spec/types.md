# Ajeeb Type System

## Primitive Types

| Type     | Description              | Size   |
|----------|--------------------------|--------|
| `int`    | Signed 64-bit integer    | 8 bytes |
| `string` | Mutable UTF-8 string     | Arena-allocated |
| `bool`   | Boolean (0 or 1)         | 8 bytes |
| `void`   | No return value          | 0 bytes |

## Composite Types

### Arrays: `int[]`, `string[]` — 0-indexed, out of bounds returns 0
### Classes: Fields require type annotations, `self` refers to current instance

## Type Inference
```
let x = 42;        // inferred as int
let name = "hi";   // inferred as string
```

## Function Types
```
function add(a: int, b: int): int { ... }
function log(msg: string): void { ... }
```
