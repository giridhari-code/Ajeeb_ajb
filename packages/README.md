# Ajeeb Standard Packages

Production-ready packages for the Ajeeb programming language ecosystem. These packages provide HTTP serving, JSON response formatting, structured logging, and database access.

## Packages

| Package | Version | Description |
|---------|---------|-------------|
| `ajeeb-json` | 1.0.0 | JSON response helpers for web services |
| `ajeeb-log` | 1.0.0 | Structured logging with levels and file output |
| `ajeeb-http` | 1.0.0 | HTTP server with routing and request parsing |
| `ajeeb-db` | 1.0.0 | Database abstraction layer (SQLite) |

## Architecture

```
ajeeb-http  ──→  ajeeb-json  ──→  std::json
    │                  │
    ↓                  ↓
ajeeb-log          ajeeb-db  ──→  sqlite_* built-ins
    │                  │
    ↓                  ↓
 println()        tcp_* built-ins
```

## Testing

Each package has standalone tests in `tests/`:

```bash
# Test individual packages
cargo run --bin ajeeb_compiler -- packages/ajeeb-json/tests/test_json.ajb
cargo run --bin ajeeb_compiler -- packages/ajeeb-log/tests/test_log.ajb
cargo run --bin ajeeb_compiler -- packages/ajeeb-http/tests/test_http.ajb
cargo run --bin ajeeb_compiler -- packages/ajeeb-db/tests/test_db.ajb

# End-to-end test
cargo run --bin ajeeb_compiler -- packages/__e2e__/test_e2e.ajb
```

## Usage with Parth

These packages are designed for the Parth build system. When Parth is ready:

```bash
parth build my_project/
```

Parth resolves `parth.das` dependencies, concatenates all sources into `build/combined.ajb`, and compiles with `ajeeb_compiler`.

## API Design

All APIs use flat function names (e.g., `http_server()` instead of `http::server()`) since Ajeeb does not yet support namespaced calls. Route handlers are registered by function name string (since closures don't exist yet).
