# ajeeb-log

Structured logging for Ajeeb applications. Supports six levels with filtering and optional file output.

## Installation

Add to `parth.das`:
```toml
[dependencies]
ajeeb-log = "^1.0.0"
```

## API

```ajeeb
// Set minimum log level: "debug", "info", "warn", "error", "fatal"
function log_set_level(level: String)

// Redirect log output to a file (empty string = stdout)
function log_set_output(path: String)

function log_debug(message: String)
function log_info(message: String)
function log_warn(message: String)
function log_error(message: String)
function log_fatal(message: String)
```

## Log Levels

| Level | Value | Description |
|-------|-------|-------------|
| DEBUG | 0 | Detailed debug information |
| INFO | 1 | General information (default) |
| WARN | 2 | Warning messages |
| ERROR | 3 | Error messages |
| FATAL | 4 | Fatal errors |

## Test

```bash
cargo run --bin ajeeb_compiler -- packages/ajeeb-log/tests/test_log.ajb
```
