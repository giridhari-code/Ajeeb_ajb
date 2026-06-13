# ajeeb-db

Database abstraction layer for Ajeeb. Currently supports SQLite via built-in functions.

## Installation

Add to `parth.das`:
```toml
[dependencies]
ajeeb-db = "^1.0.0"

[runtime]
sqlite = "bundled"
```

## API

```ajeeb
// Open a database connection
function db_open(path: String): Int

// Close a database connection
function db_close(handle: Int)

// Execute a non-query SQL statement
function db_exec(handle: Int, sql: String): DbResult

// Execute a query and return results
function db_query(handle: Int, sql: String): Array

// Escape a string for safe SQL usage
function db_escape_string(s: String): String

// Build a SELECT query
function db_select(table: String, where_clause: String): String

// Build an INSERT query from a tagged object
function db_insert(table: String, obj: Array): String

// Escape a table/column name
function db_escape_table(name: String): String
```

## Structs

```ajeeb
struct DbResult {
    success: Bool;
    error: String;
    rows: Array;
    affected: Int;
}
```

## Notes

- In interpreter mode, `sqlite_*` functions are stubs returning empty results
- Full SQLite requires native compilation with `-lsqlite3 -DUSE_SQLITE3`
- Always use `db_escape_string` for user-supplied values

## Test

```bash
cargo run --bin ajeeb_compiler -- packages/ajeeb-db/tests/test_db.ajb
```
