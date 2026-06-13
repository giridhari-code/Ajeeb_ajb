# ajeeb-web — Web Framework for Ajeeb

A production-style web framework built in pure Ajeeb using only existing language features (no new syntax).

## Features

- **Router**: Register routes with GET, POST, PUT, DELETE
- **Route Groups**: Prefix-based route grouping
- **Middleware Chain**: Request filtering with short-circuit support
- **Request Context**: Response-building methods (JSON, HTML, Text)
- **JSON Serialization**: Built-in `web_json_stringify` for tagged arrays
- **Error Handling**: Custom error handler registration
- **Dependency Injection**: Lazy singleton service container
- **Static File Serving**: Serve files from directory prefixes
- **Configuration Loading**: File-based config reader
- **Testing Utilities**: Mock request/response helpers

## Quick Start

```ajeeb
import mod;

function handle_home(method: String, path: String, body: String): Array {
    return web_ok("{\"message\":\"Hello!\"}");
}

function main() {
    let app = web_new();
    app.get("/", "handle_home");
    web_listen(app, 8080);
}
```

## API Reference

### App Creation

```ajeeb
let app = web_new();  // Create a new App instance
```

### Route Registration

```ajeeb
app.get("/path", "handler_fn_name");
app.post("/path", "handler_fn_name");
app.put("/path", "handler_fn_name");
app.delete("/path", "handler_fn_name");
```

Or directly:

```ajeeb
web_route(app, "GET", "/path", "handler_fn_name");
```

### Route Groups

```ajeeb
let api = web_group(app, "/api/v1");
web_group_route(api, "GET", "/items", "handle_items");
// Registers route at /api/v1/items
```

### Handler Functions

Handlers receive 3 or 4 arguments: `(method, path, body[, ctx])`.

```ajeeb
// Without context
function handle_get(method: String, path: String, body: String): Array {
    return web_ok("{\"ok\":true}");
}

// With context (for response methods)
function handle_with_ctx(method: String, path: String, body: String, ctx): Array {
    return ctx.json(["obj", "message", "hello"], 200);
}
```

### Middleware

```ajeeb
app.use("middleware_fn_name");

function logger(method: String, path: String, body: String, ctx): Array {
    // Return [] to continue, or a response array to short-circuit
    return [];
}
```

### Response Helpers

```ajeeb
web_ok(body)           // 200 JSON
web_created(body)      // 201 JSON
web_bad_request(body)  // 400 JSON
web_not_found(body)    // 404 JSON
web_server_error(body) // 500 JSON
```

### JSON

```ajeeb
web_json_stringify(["obj", "key", "value"])    // {"key":"value"}
web_json_str("key", "value")                   // {"key":"value"}
web_json_obj("k1", "v1", "k2", "v2")           // {"k1":"v1","k2":"v2"}
web_json_error("Not Found", 404)               // {"error":"Not Found","code":"404"}
```

### Context Methods

```ajeeb
ctx.ok(["obj", "key", "val"])     // 200 JSON
ctx.json(data, 201)                // Custom status JSON
ctx.text("plain text")             // 200 text/plain
ctx.html("<h1>Title</h1>")         // 200 text/html
```

### Error Handling

```ajeeb
// Direct field assignment (method doesn't persist due to scope cloning)
app.error_handler = "my_error_handler";

function my_error_handler(method: String, path: String, body: String, ctx, status: Int, resp_body: String): Array {
    return [status, ["Content-Type: application/json"], resp_body];
}
```

### Dependency Injection

```ajeeb
let container = di_container();
di_register(container, "db", "factory_fn");
let svc = di_resolve(container, "db");  // Cached singleton
```

### Static Files

```ajeeb
app.static("/static", "./public");

// Serves ./public/index.html for GET /static/index.html
```

### Testing

```ajeeb
let resp = web_test(app, "GET", "/path", "body");
web_assert_status(resp, 200);
web_assert_body_contains(resp, "expected text");
```

### HTTP Server

```ajeeb
web_listen(app, 8080);  // Start TCP server
```

## Limitations

- **Scalar field mutations via methods are lost** due to scope cloning. Use direct field assignment in the same scope, or array-based mutations (which persist via `Rc<RefCell>`).
- **No `:param` path patterns** — exact matching only.
- **JSON serializer always quotes values** — no native number type detection.

## Running Tests

```bash
cargo run --bin ajeeb_compiler -- packages/ajeeb-web/run_tests.ajb
```

## Example

```bash
cargo run --bin ajeeb_compiler -- packages/ajeeb-web/example_web.ajb
```
