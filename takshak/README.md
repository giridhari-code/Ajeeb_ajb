# Takshak — Ajeeb Backend Web Framework

Takshak is the official backend web framework for the Ajeeb programming language.
Write Ajeeb backend code and compile to a native HTTP server binary.

## Quick Start

```bash
# Build the hello server
cat src/server.ajb src/request.ajb src/response.ajb examples/hello.ajb > build/combined.ajb

# Compile via self-hosted compiler
cd /path/to/ajeeb_compiler
cargo run --bin ajeeb_compiler -- compiler/compiler.ajb takshak/build/combined.ajb takshak/build/output.c

# Compile with gcc
cd takshak
gcc -include runtime/takshak_runtime.h \
    build/output.c \
    ../runtime/ajeeb_runtime.c \
    runtime/takshak_runtime.c \
    -o build/server \
    -Wall -Wno-int-to-pointer-cast -Wno-pointer-to-int-cast -Wno-int-conversion

# Run
./build/server
```

## Example

```ajeeb
function handleHello(req: int, res: int): void {
    takshakResJson(res, "{\"message\": \"Namaste!\"}");
}

function main(): int {
    takshakInit();
    takshakGet("/hello", handleHello);
    takshakListen(3000);
    return 0;
}
```

## API

- `takshakInit()` — Initialize the server
- `takshakGet(path, handler)` — Register GET route
- `takshakPost(path, handler)` — Register POST route
- `takshakListen(port)` — Start event loop (blocking)
- `takshakReqPath(req)` — Get request path
- `takshakReqMethod(req)` — Get request method  
- `takshakReqBody(req)` — Get request body
- `takshakResSend(res, body)` — Send response
- `takshakResJson(res, json)` — Send JSON response
- `takshakResHtml(res, html)` — Send HTML response
- `takshakResStatus(res, code)` — Set HTTP status code
- `takshakResHeader(res, key, val)` — Set response header

## Status Codes

200, 201, 400, 404, 500 supported. Route wildcards (`/*`) for fallback handlers.
