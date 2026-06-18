# Ajeeb Formal Grammar

```
program        → statement*

statement      → function_def | variable_decl | assignment ";" | if_stmt
               | while_stmt | for_stmt | return_stmt ";" | expr ";" | ";"

function_def   → "function" IDENT "(" parameter_list? ")" (":" type)? block
parameter_list → parameter ("," parameter)*
parameter      → IDENT ":" type

variable_decl  → "let" IDENT (":" type)? "=" expr ";"
               | "const" IDENT ":" type "=" expr ";"

assignment     → IDENT "=" expr | IDENT "[" expr "]" "=" expr | expr "." IDENT "=" expr

block          → "{" statement* "}"

if_stmt        → "if" "(" expr ")" block ("else" block)?
while_stmt     → "while" "(" expr ")" block
for_stmt       → "for" "(" (variable_decl | expr)? ";" expr? ";" expr? ")" block
return_stmt    → "return" expr?

expr           → atom (binop atom)* | IDENT "=" expr
atom           → INT | STRING | BOOL | IDENT ("(" argument_list? ")")? | "(" expr ")"

type           → "int" | "string" | "bool" | "void" | type "[]" | IDENT
```
