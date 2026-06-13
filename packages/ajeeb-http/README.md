# ajeeb-http

HTTP server with routing, request parsing, and response building for Ajeeb.

## Installation

Add to `parth.das`:
```toml
[dependencies]
ajeeb-http = "^1.0.0"
ajeeb-json = "^1.0.0"
ajeeb-log = "^1.0.0"
```

## API

```ajeeb
// Create a new HTTP server instance
function http_new(): HttpServer

// Register a route with method, path, and handler function name
function http_route(server: HttpServer, method: String, path: String, handler: String)

// Start the HTTP server (enters accept loop)
function http_listen(server: HttpServer, port: Int)

// Stop the server
function http_stop(server: HttpServer)

// Parse raw HTTP request into HttpRequest struct
function http_parse_request(raw: String): HttpRequest

// Build and send an HTTP response over a client socket
function http_send(client_fd: Int, response: Array)

// Get HTTP status text for standard codes
function http_status_text(code: Int): String
```

## Handler Signature

Route handlers receive 3 arguments and return a response array:

```ajeeb
function my_handler(method: String, path: String, body: String): Array {
    return [200, ["Content-Type: application/json"], "{\"status\":\"ok\"}"];
}
```

## Structs

```ajeeb
struct Route {
    method: String;
    path: String;
    handler: String;
}

struct HttpServer {
    port: Int;
    routes: Route[];
    running: Bool;
}

struct HttpRequest {
    method: String;
    path: String;
    headers: Array;
    body: String;
}
```

## Test

```bash
cargo run --bin ajeeb_compiler -- packages/ajeeb-http/tests/test_http.ajb
```
