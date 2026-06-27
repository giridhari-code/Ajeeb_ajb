# Ajeeb Language

Ajeeb ek fun-language hai jo Hindi+English (Hinglish) mein error deti hai.

## Install (sirf ek baar)

```bash
bash install.sh
```

Isse yeh banege:
- `build/ajeebc` — Rust compiler (Ajeeb → LLVM IR)
- `build/piri` — Piri interpreter (Ajeeb code seedha chalao)
- `build/parth` — CLI project manager

## Use karo

```bash
./build/parth init my-project
cd my-project
../build/parth run       # interpret karo (Piri)
../build/parth build     # compile karo (native binary)
../build/parth test      # tests chalao
```

Ya PATH mein add kar lo:
```bash
export PATH="$(pwd)/build:$PATH"
parth init demo && cd demo && parth run
```

## Example

`src/main.ajb`:
```ajeeb
function main(): int {
    println("Namaste Duniya!\n");
    return 0;
}
```

## Commands

| Command | Kya karta hai |
|---------|---------------|
| `parth init [name]` | Current dir mein project banaye |
| `parth new <name>` | Naya folder bana ke project banaye |
| `parth build` | Native binary banaye (LLVM + gcc) |
| `parth run` | Piri se code chalaaye |
| `parth test` | Tests chalaaye |

## Errors

Saare errors **Hinglish** mein aate hain! Examples:
- "Yeh kya hai bhai?" — syntax error
- "Function define nahi hai!" — undefined function
- "Type mismatch ho gaya!" — type error

## Architecture

```
.ajb file → Lexer → Parser → HIR → MIR → Piri (interpreter)
                                         → LLVM IR → native binary
```
