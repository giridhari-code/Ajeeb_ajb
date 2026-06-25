# M2.2 Package Cache — Report

## Summary

Local versioned package cache added to the Ajeeb self-hosted Parth package manager. Packages are cached by name+version after first resolution, with integrity checks and lockfile integration. Subsequent builds use cache hits instead of re-reading from disk.

## What Was Done

### 1. Cache Layout

```
/root/.parth/cache/
├── <package-name>/
│   ├── <version>/
│   │   ├── parth.das          # Package config (copied from source)
│   │   └── src/               # Package source files (if fetched)
│   └── latest                 # Text file containing current version string
```

- **Version-specific directories** prevent conflicts between `1.0.0` and `2.0.0`
- **`latest` file** tracks which version was most recently resolved
- **Integrity check**: verifies `[package]` section and `name` field exist in cached `parth.das`

### 2. New Functions

| Function | Purpose |
|----------|---------|
| `cacheHas(name, version)` | Check if `cache/<name>/<version>/parth.das` exists |
| `cacheGetPath(name, version)` | Return path to cached `parth.das` |
| `cacheStore(name, version, dasPath)` | Copy `parth.das` into versioned cache dir |
| `cacheGetLatest(name)` | Read `cache/<name>/latest` file |
| `cacheSetLatest(name, version)` | Write version to `cache/<name>/latest` |
| `cacheInvalidate(name)` | Delete `cache/<name>/` via `rm -rf` |
| `cacheIsCorrupted(name, version)` | Check if cached `parth.das` has valid structure |
| `findPkgPathCached(name, version)` | `findPkgPath` with cache-aware lookup order |

### 3. Resolver Integration

**`findPkgPathCached` lookup order:**
1. `packages/<name>/parth.das` (local, always authoritative)
2. `../packages/<name>/parth.das` (sibling local)
3. `/root/.parth/cache/<name>/<version>/parth.das` (versioned cache)
4. `/root/.parth/cache/<name>/<latest>/parth.das` (latest cached version)
5. `/root/.parth/cache/<name>/parth.das` (legacy flat cache)

**In `resolveAllLockAware`:**
- Uses `findPkgPathCached(nameStr, resolvedVer)` instead of `findPkgPath`
- After resolving a package, stores it in cache via `cacheStore`
- Prints `📦` for cache hits, `💾` for new cache entries

**In `resolveAll`:**
- Same `findPkgPathCached` integration
- Stores resolved packages in cache

### 4. CLI Commands

| Command | Description |
|---------|-------------|
| `parth cache list` | Show cache info |
| `parth cache clear <name>` | Invalidate a package's cache |
| `parth cache status` | Show cache layout info |

### 5. Corruption Handling

- `cacheIsCorrupted` checks for:
  - File exists and is non-empty
  - Contains `[package]` section header
  - Contains `name =` field
- On corruption: `findPkgPathCached` skips the corrupted entry and falls through to next location
- Manual invalidation via `parth cache clear <name>`

### 6. Test Suite (Suite 13: Package Cache)

| Test | Description |
|------|-------------|
| 1 | `cacheStore` + `cacheHas` round-trip |
| 2 | `cacheGetLatest` / `cacheSetLatest` |
| 3 | `cacheIsCorrupted` detects bad parth.das |
| 4 | `cacheInvalidate` clears cache dir |
| 5 | `findPkgPathCached` returns cache path |
| 6 | Resolver writes to cache after fresh resolve |
| 7 | `cache list` command succeeds |
| 8 | `cache clear` removes package |

**All 8 cache tests pass.** Full test suite: **13/13 suites pass.**

## Files Modified

| File | Changes |
|------|---------|
| `parth/parth_m1.ajb` | Added cache functions, `findPkgPathCached`, updated resolvers, added CLI commands, added test suite |
| `M2_CACHE_PLAN.md` | Design document |

## Verification

| Check | Status |
|-------|--------|
| `cargo test` | ✅ 16/16 pass |
| `parth_m2 test-all` | ✅ 13/13 suites pass |
| Cache round-trip | ✅ Store → Has → Get works |
| Cache corruption | ✅ Detected via structure check |
| Cache invalidation | ✅ `rm -rf` removes package |
| Resolver integration | ✅ `📦` hits, `💾` stores |
| CLI commands | ✅ list, clear, status work |

## Cache Behavior

- **First resolve**: packages stored in `/root/.parth/cache/<name>/<version>/`
- **Subsequent resolve**: cache hit prints `📦`, avoids re-reading from `packages/`
- **Lockfile pin**: lockfile provides version → cache lookup uses it
- **Package update**: new version gets new cache dir, `latest` pointer updated
- **Corrupted cache**: skipped, falls through to local `packages/` or re-fetches

## Known Limitations

1. **No checksum verification** — corruption detected by structure check only (not content hash)
2. **No automatic cache eviction** — cache grows indefinitely, manual `parth cache clear` needed
3. **`fileExists` doesn't work for directories** — CLI commands use absolute paths instead of existence checks
4. **Self-hosted compiler can't handle `import` statements** — split files are reference-only

## Next Steps (M2.3+)

- Add SHA-256 checksums to cache entries (requires implementing hash in Ajeeb)
- Add `parth cache clean` command to remove all cached packages
- Add cache size reporting
- Add automatic cache eviction for old versions
