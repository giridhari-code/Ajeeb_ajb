# 🚀 Ajeeb Compiler (Version 0.1)

**Ajeeb** ek high-performance, hybrid-syntax programming language hai jise **AI, Robotics, aur Gaming** ke liye design kiya ja raha hai. Iska core syntax **TypeScript** jaisa clean aur modern hai, lekin iska backend direct ultra-fast **Assembly Code** generate karta hai.

Yeh compiler poori tarah se **Rust** me likha gaya hai aur iska final goal **Bootstrapping** (Ajeeb me hi Ajeeb ka compiler chalana) hai. Current development environment mobile me **Termux (Ubuntu v22.04 / aarch64)** par chal raha hai.

---

## 🏗️ Architecture Design

Compiler ka workflow 3 main stages me divided hai:

1. **Lexer (Tokenizer):** `.ajb` file ke high-level code string ko chote-chote blocks (Tokens) me todta hai.
2. **Parser / Generator:** Tokens ka pattern check karta hai (Syntax Validation) aur unhe direct machine registers ke assembly instructions me badalta hai.
3. **Assembler (Target):** Assembly code (`.asm`) ko executable binary me convert karta hai.

---
