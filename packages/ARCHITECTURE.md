# Ajeeb Backend Ecosystem Architecture

## Overview

The Ajeeb backend ecosystem provides production-grade packages for building
web services, APIs, and database-backed applications. All packages build on
the existing Ajeeb compiler, runtime, and Parth package manager — no new
language syntax is added.

## Core Design Principles

1. **No new syntax** — Everything uses existing structs, enums, functions,
   modules, and method calls.
2. **Flat function API** — Since Ajeeb lacks namespaced function calls
   (`http::server()`), all public functions use a prefix convention:
   `http_server()`, `json_stringify()`, `log_info()`.
3. **Struct-based state** — Server instances, connections, requests, and
   responses are `struct` values with fields.
4. **Function-name callbacks** — Instead of closures, handlers are registered
   as string function names that the server dispatches to by name.
5. **Runtime primitives via built-in functions** — TCP sockets and SQLite
   access are added as C runtime functions + Rust evaluator builtins.

## Package Graph

```
                 ┌─────────────┐
                 │  ajeeb-log  │
                 │  (no deps)  │
                 └──────┬──────┘
                        │ depends
                        ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ ajeeb-std/json │──▶│ ajeeb-json │  │ ajeeb-http  │
│ (stdlib)    │  │ (wraps      │  │ (server +   │
│             │  │  std::json) │  │  client)    │
└─────────────┘  └──────┬──────┘  └──────┬──────┘
                        │ depends        │ depends
                        ▼                ▼
                 ┌─────────────┐  ┌─────────────┐
                 │ ajeeb-db    │  │ TCP builtins│
                 │ (SQLite)    │  │ (runtime)   │
                 └─────────────┘  └─────────────┘

ajeeb-log    ◄── used by all packages for diagnostics
ajeeb-json   ◄── used by ajeeb-http for JSON API responses
ajeeb-http   ◄── uses ajeeb-json + TCP builtins + ajeeb-log
ajeeb-db     ◄── uses ajeeb-log, standalone DB access
```

## Runtime Additions (new built-in functions)

No new syntax — only new functions added to eval.rs and ajeeb_runtime.c:

| Function | Signature | Description |
|----------|-----------|-------------|
| `tcp_listen(port)` | Int → Int | Create TCP listen socket, return fd |
| `tcp_accept(fd)` | Int → Int | Accept connection, return client fd |
| `tcp_read(fd, max)` | Int, Int → String | Read up to max bytes from socket |
| `tcp_write(fd, data)` | Int, String → Void | Write string to socket |
| `tcp_close(fd)` | Int → Void | Close socket |
| `sqlite_open(path)` | String → Int | Open SQLite DB, return handle |
| `sqlite_close(handle)` | Int → Void | Close SQLite DB |
| `sqlite_exec(handle, sql)` | Int, String → Int | Execute SQL, return result code |
| `sqlite_query(handle, sql)` | Int, String → Array | Execute query, return rows |
| `sqlite_last_error(handle)` | Int → String | Get last error message |
| `now_ms()` | → Int | Current epoch time in milliseconds |

## Package Layout Convention

Each package follows Parth convention:

```
packages/<name>/
├── parth.das          # Package manifest
├── README.md          # Documentation
├── mod.ajb            # Main module (auto-loaded by import)
├── src/
│   ├── internal.ajb   # Internal helpers (optional)
│   └── ...
├── tests/
│   ├── test_*.ajb     # Test files
│   └── ...
└── examples/
    └── example_*.ajb  # Usage examples
```

## Public API Design

### Ajeeb JSON (`ajeeb-json`)
```
json_response(status, body)     → [status_code, headers, body_str]
json_response_ok(body)          → [200, ["Content-Type: application/json"], body]
json_response_error(msg, code)  → [code, [...], json_error_body]
json_parse_request(data)        → parse HTTP body as JSON
```

### Ajeeb HTTP (`ajeeb-http`)
```
http_new()                      → HttpServer { routes, port, ... }
http_route(server, method, path, handler_fn_name)
http_static_dir(server, prefix, dir)
http_listen(server, port)
http_parse_request(raw)         → HttpRequest { method, path, headers, body }
http_response(status, headers, body) → HttpResponse
http_send(client_fd, response)
http_read_request(client_fd)    → HttpRequest
```

### Ajeeb Log (`ajeeb-log`)
```
log_info(message)
log_warn(message)
log_error(message)
log_debug(message)
log_set_level(level)            // "debug", "info", "warn", "error"
log_set_output(file_path)       // redirect to file
```

### Ajeeb DB (`ajeeb-db`)
```
db_open(path)                   → DbHandle
db_close(handle)
db_exec(handle, sql)            → Int (result code)
db_query(handle, sql)           → Array of Row structs
db_escape_string(s)             → sanitized string
```

## Implementation Status (✅ Complete)

All four packages are implemented, tested, and documented:

### ✅ Runtime Additions
- TCP built-ins: `tcp_listen`, `tcp_accept`, `tcp_read`, `tcp_write`, `tcp_close`
- SQLite built-ins: `sqlite_open`, `sqlite_close`, `sqlite_exec`, `sqlite_query`, `sqlite_last_error`
- Utility built-ins: `now_ms`, `call_fn`, `assert_eq`, `assert_neq`, `assert_contains`, `arr_len`
- C runtime (`ajeeb_runtime.c`): POSIX TCP sockets, SQLite stubs, `clock_gettime`

### ✅ Package: ajeeb-json
- JSON response builders (`json_response_ok`, `json_response_error`, `json_response`)
- JSON object helpers (`json_pair`, `json_obj`, `json_results`)
- `parth.das` manifest, `mod.ajb`, tests, examples, README

### ✅ Package: ajeeb-log
- Level-based filtering (`log_set_level`: debug/info/warn/error/fatal)
- File output redirection (`log_set_output`)
- `parth.das` manifest, `mod.ajb`, tests, examples, README

### ✅ Package: ajeeb-http
- HTTP server with struct-based state (`HttpServer`, `Route`, `HttpRequest`)
- Route registration (`http_route`) and dispatch (`http_listen`)
- HTTP request parsing (`http_parse_request`) and response building (`http_send`)
- Handler dispatch via `call_fn` built-in (dynamic function calling by name)
- `parth.das` manifest, `mod.ajb`, tests, examples, README

### ✅ Package: ajeeb-db
- Database connection management (`db_open`, `db_close`)
- Query execution (`db_exec`, `db_query`)
- SQL injection prevention (`db_escape_string`, `db_escape_table`)
- Query builders (`db_select`, `db_insert`)
- `parth.das` manifest, `mod.ajb`, tests, examples, README

### ✅ Test Results
```
ajeeb-json: 7/7 PASS
ajeeb-log:  7/7 PASS
ajeeb-http: 7/7 PASS
ajeeb-db:   7/7 PASS
e2e:        5/5 PASS
stdlib:     46/46 PASS
```

### 🚀 Next: Parth Build Integration
- Parth resolves `parth.das` dependencies and concatenates all `.ajb` sources into `build/combined.ajb`
- Native compilation with `gcc build/output.c runtime/ajeeb_runtime.c -o build/ajeeb_native`
- SQLite: `-lsqlite3 -DUSE_SQLITE3`

## Dependency Graph (Parth)

```ini
# ajeeb-json/parth.das
[dependencies]
# none (uses std::json from stdlib)

# ajeeb-log/parth.das  
[dependencies]
# none

# ajeeb-http/parth.das
[dependencies]
ajeeb-json = "^1.0.0"
ajeeb-log = "^1.0.0"

# ajeeb-db/parth.das
[dependencies]
ajeeb-json = "^1.0.0"
ajeeb-log = "^1.0.0"
```

## Target Syntax Evolution

Once the language gains namespaced calls and closures, the API will
transition naturally:

```ajeeb
// Future (requires :: and closures):
import http;
let app = http::server();
app.get("/", fn(req) { return json::response({ "msg": "hi" }); });
app.listen(8080);
```

But the current API (implemented here) works TODAY with no new syntax:

```ajeeb
import http;
import json;
import log;

fn index_handler(req) {
    return json_response_ok(json_parse("{\"msg\":\"hi\"}"));
}

fn main() {
    log_info("Starting server...");
    let app = http_new();
    http_route(app, "GET", "/", "index_handler");
    http_listen(app, 8080);
}
```

## Testing

Each package ships with test files under `tests/` that use the existing
test patterns (function calls + print-based output). Tests are run via:

```
parth run tests/test_<name>.ajb
```

## Roadmap

| Phase | Packages | Timeline |
|-------|----------|----------|
| 1 | Runtime networking, ajeeb-json, ajeeb-log | Now |
| 2 | ajeeb-http (basic server) | Now |
| 3 | ajeeb-db (SQLite) | Now |
| 4 | ajeeb-http (middleware, static files, SSL) | Future |
| 5 | ajeeb-db (Postgres adapter) | Future |
| 6 | Namespaced calls + closures (compiler) | Future |
