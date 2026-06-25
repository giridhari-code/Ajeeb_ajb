# PARTH M1 AUDIT — Package Management Parity

## Overview

Three implementations of parth exist. The Ajeeb (.ajb) implementation is significantly simpler than the Rust versions. M1 bridges this gap for add/remove/resolution/version-matching/tree.

---

## 1. ADD Command

### Rust (ajeebc) — `commands/deps.rs:9-73` (65 LOC)

```
cmd_add(spec):
  1. Parse "name[@version]" — default "*"
  2. Read parth.das via config::read_config()
  3. Check duplicate → early return if exists
  4. find_local_package(name) — searches 4 paths:
     ./packages/<name>/, ~/.parth/packages/<name>/, ../packages/<name>/, <ajeeb_root>/packages/<name>/
  5. If local: link_local_package() — copies to ~/.parth/packages/<name>/<version>/
  6. If not local: download_package() — same 4-path search (remote is stub)
  7. resolve_and_cache(all_deps) — full PubGrub resolution
  8. On success: config::update_deps() — rewrites [dependencies] section
  9. On failure: print error, exit
```

**Edge cases:** Duplicate dep, missing parth.das, resolve failure rollback.

### Ajeeb M0 — `resolver.ajb:131-163` (33 LOC)

```
addDep(name, version):
  1. readFile("parth.das") → src
  2. parseDasConfig(src) → buf
  3. Iterate deps: if name exists → "pehle se hai!", return
  4. Construct "name = \"version\"\n"
  5. writeFile(dasPath, str_concat(src, newLine))
```

**Missing:** No `@`-parsing, no resolution, no local/cache lookup, no validation, **NOT WIRED in CLI dispatch**.

---

## 2. REMOVE Command

### Rust (ajeebc) — `commands/deps.rs:75-93` (19 LOC)

```
cmd_remove(name):
  1. Check parth.das exists
  2. read_config_basic() → deps list
  3. Filter: deps.filter(|d| d.name != name)
  4. update_deps(path, &new_deps) — rewrites [dependencies]
  5. read_lock() → remove entry → write_lock()
```

**Edge cases:** Non-existent dep (silently succeeds), missing lock file.

### Ajeeb M0 — `resolver.ajb:165-210` (46 LOC)

```
removeDep(name):
  1. readFile("parth.das") → src
  2. Line-by-line: if line starts with "name =" → skip (removed=1)
  3. Reconstruct newSrc without matching lines
  4. writeFile(dasPath, newSrc)
  5. If removed==0 → print error
```

**Missing:** No lock file cleanup, **NOT WIRED in CLI dispatch**.

---

## 3. Dependency Resolution

### Rust (ajeebc) — `resolver.rs:155-349` (195 LOC) + helpers

**Algorithm:** PubGrub-style backtracking BFS resolver.

```
resolve_and_cache(deps):
  1. Load local index (~/.parth/index)
  2. Load existing lock file
  3. BFS queue seeded with all direct deps
  4. While queue not empty:
     a. Pop dep from front
     b. If already resolved to compatible version → skip
     c. If conflict → BACKTRACK:
        - Pop decision_trail to find choice point
        - Remove all non-pivot packages from resolved
        - try_alternate_version() → re-queue transitive deps
     d. resolve_version():
        - Lock file (pinned) → remote index → local index
        - Sort candidates newest-first, return first match
     e. ensure_package() — verify cache + checksum
     f. read_package_deps() — read transitive deps from cached parth.das
     g. Pin existing-lock transitive deps with "=" prefix
     h. Push Decision to trail
     i. Enqueue unresolved transitive deps
  5. Clean stale lock entries
  6. Write parth.lock
  7. Return resolved list in order
```

**Data structures:**
- `Decision { package, version, dependencies, level }` — backtracking trail
- `LockFile = HashMap<String, LockEntry>` — lock cache
- `RegistryIndex = HashMap<String, HashMap<String, String>>` — pkg→ver→checksum
- `VecDeque<PkgDep>` — BFS queue
- `tried: HashMap<String, Vec<(Version, String)>>` — tried versions per package

**Helpers:** `compilation_order` (topo sort, 37 LOC), `read_lock`/`write_lock` (90 LOC), `print_tree` (23 LOC), `why` (18 LOC), `check_outdated` (26 LOC).

### Ajeeb M0 — `resolver.ajb:64-129` (66 LOC)

**Algorithm:** Flat linear scan, no recursion.

```
resolveAll():
  1. For each dep in parth.das:
     a. findPackageLocal(name) → 2 fixed paths
     b. findPackageCache(name) → ~/.parth/cache/<name>/
     c. fetchPackage(name, version) → git clone (version IGNORED)
  2. Count missing, return error if any
```

**Missing:** No transitive resolution, no conflict detection, no backtracking, no lock integration, no cycle detection.

---

## 4. Version Matching

### Rust (ajeebc) — `types.rs:112-327` (216 LOC)

**Version struct:** `{ major, minor, patch, pre: Vec<String>, build: Vec<String> }`

**Parsing** (18 LOC): `MAJOR.MINOR.PATCH[-pre][+build]`, 2-part allowed (defaults patch=0).

**Ordering** (19 LOC): major > minor > patch. Pre-release < release (`1.0.0-alpha < 1.0.0`). Numeric pre-release IDs compared numerically, string IDs lexicographically.

**Constraint enum** (10 variants):

| Variant | Syntax | Semantics |
|---------|--------|-----------|
| `Any` | `*` | Matches all |
| `Exact` | `1.2.3` or `=1.2.3` | Exact match |
| `Caret` | `^1.2.3` | `>=1.2.3, <2.0.0` (major lock; `^0.2.3` = minor lock) |
| `Tilde` | `~1.2.3` | `>=1.2.3, <1.3.0` (major+minor lock) |
| `Gte` | `>=1.2.3` | Greater or equal |
| `Gt` | `>1.2.3` | Strictly greater |
| `Lte` | `<=1.2.3` | Less or equal |
| `Lt` | `<1.2.3` | Strictly less |
| `And` | `>=1.0.0 <2.0.0` | Both must match |
| `Or` | `^1.0.0 \|\| ^2.0.0` | Either matches |

**Parsing** (35 LOC): Handles `||`, `&&`, implicit AND (space-separated), then `parse_single()` strips prefix operators.

**Matching** (20 LOC): Full `matches(version)` with caret/tilde special logic.

### Ajeeb M0 — **NOT IMPLEMENTED**

Zero version matching logic. Version string treated as opaque text. `fetchPackage` ignores version entirely.

---

## 5. Dependency Tree Display

### Rust (ajeebc) — `resolver.rs:469-512` (44 LOC)

```
print_tree(deps):  — formatted tree with box-drawing chars
print_dep_tree(name, version, deps, prefix, visited):  — recursive DFS with cycle detection
why(name):  — explain why a package is included (path from root)
check_outdated():  — compare lock versions against registry
```

### Ajeeb M0 — **NOT IMPLEMENTED**

---

## 6. Manifest Updates

### Rust (ajeebc) — `config.rs:162-201` (40 LOC)

```
update_deps(path, deps):
  1. Read all lines
  2. Find [dependencies] section
  3. Strip lines containing "=" within that section
  4. Insert new dep lines
  5. If no [dependencies] section exists, append one at end
  6. Write file back
```

### Ajeeb M0 — **Implemented inline** in addDep/removeDep

addDep appends raw string. removeDep does line-by-line filter. No section-aware rewriting.

---

## Summary Table

| Feature | Rust LOC | Ajeeb LOC | Gap | Complexity |
|---------|----------|-----------|-----|------------|
| `add` | 65 | 33 (unwired) | CLI wiring, `@`-parse, validation | Low |
| `remove` | 19 | 46 (unwired) | CLI wiring, lock cleanup | Low |
| Resolution (flat) | — | 66 | Sufficient for M1 | — |
| Resolution (transitive) | 195 | 0 | Full implementation needed | High |
| Version parsing | 18 | 0 | Full implementation needed | Medium |
| Version comparison | 19 | 0 | Full implementation needed | Medium |
| Constraint parsing | 35 | 0 | Full implementation needed | Medium |
| Constraint matching | 20 | 0 | Full implementation needed | Medium |
| Tree display | 44 | 0 | Implementation needed | Low |
| Why query | 18 | 0 | Not in M1 scope | — |
| Outdated check | 26 | 0 | Not in M1 scope | — |
| Topo sort | 37 | 0 | Needed for build order | Medium |
| Lock read/write | 90 | 48 (simple) | Not in M1 scope | — |
| Config update | 40 | inline | Sufficient | — |

---

## Rewrite Order (per feature)

| # | Feature | Depends On | LOC Estimate (Ajeeb) |
|---|---------|-----------|----------------------|
| 1 | CLI wiring for add/remove | Nothing | ~20 |
| 2 | Semver parsing | Nothing | ~40 |
| 3 | Version comparison (Ord) | Semver parsing | ~30 |
| 4 | Constraint parsing | Semver parsing | ~50 |
| 5 | Constraint matching | Version comparison, Constraint parsing | ~40 |
| 6 | `add` with version validation | Constraint parsing, Config update | ~15 (modify existing) |
| 7 | `remove` with lock cleanup | Lock read/write | ~10 (modify existing) |
| 8 | Transitive resolution (BFS) | Constraint matching, findPackage | ~80 |
| 9 | Dependency tree display | Transitive resolution | ~40 |
| 10 | Topo sort (build order) | Transitive resolution | ~40 |
