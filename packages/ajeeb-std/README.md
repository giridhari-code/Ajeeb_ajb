# Ajeeb Standard Library

`packages/ajeeb-std/` — Standard library for Ajeeb programming language.

## Modules

| File | Description |
|------|-------------|
| `io.ajb` | Input/output — print, read, file lines |
| `math.ajb` | Math — abs, pow, factorial, gcd, isPrime |
| `string.ajb` | String utilities — repeat, reverse, pad, join, count |
| `array.ajb` | Array utilities — sum, max, min, reverse, sort |
| `fs.ajb` | File system — exists, copy, mkdir |
| `result.ajb` | Result/Option types — ok, err, some, none |
| `collections.ajb` | Data structures — Stack, Queue |

## Use

```ajb
import io;
import math;
import string;

function main(): int {
    println(itoa(math::abs(-42)));
    return 0;
}
```
