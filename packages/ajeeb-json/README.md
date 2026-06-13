# ajeeb-json

JSON response helpers for Ajeeb web services. Builds on `std::json` to provide convenient response formatting for HTTP APIs.

## Installation

Add to `parth.das`:
```toml
[dependencies]
ajeeb-json = "^1.0.0"
```

## API

```ajeeb
// Create a 200 JSON response from a tagged object
function json_response_ok(obj: Array): Array

// Create a 200 JSON response from a raw string body
function json_response_ok_str(body: String): Array

// Create a JSON error response with message and status code
function json_response_error(message: String, code: Int): Array

// Create a JSON response with custom status, headers, and body
function json_response(status: Int, headers: Array, body: String): Array

// Parse a JSON string from an HTTP request body
function json_parse_body(body: String): Array

// Build a tagged object from 4 key-value pairs
function json_obj(k1, v1, k2, v2, k3, v3, k4, v4): Array

// Build a simple {"key": "value"} tagged object
function json_pair(key: String, value: String): Array

// Wrap results array with a count
function json_results(results: Array, total: Int): Array
```

## Response Format

Responses are arrays in the format `[status_code, headers_array, body_string]`.

## Test

```bash
cargo run --bin ajeeb_compiler -- packages/ajeeb-json/tests/test_json.ajb
```
