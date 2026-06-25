# M2.2 Package Cache — Implementation Plan

## Goal

Add a local versioned package cache that stores resolved packages by name+version, with integrity checks and lockfile integration. Packages are cached after first resolution so subsequent builds skip redundant lookups.

## Current State

- `findPkgPath(name)` checks 3 locations: `packages/<name>/`, `../packages/<name>/`, `/root/.parth/cache/<name>/`
- `fetchPackage(name, version)` exists but is dead code (never called in M1 resolver)
- Lockfile v2 stores `[name]\nversion = "X.Y.Z"` entries
- No checksums, no version-specific caching, no cache invalidation

## Cache Layout

```
/root/.parth/cache/
├── <package-name>/
│   ├── <version>/
│   │   ├── parth.das          # Package config (copied from source)
│   │   └── src/               # Package source files (if fetched)
│   └── latest                 # Text file containing current version string
```

- **Version-specific dirs** prevent conflicts between `1.0.0` and `2.0.0`
- **`latest` file** tracks which version was most recently resolved (for quick lookup)
- **No SHA256** (Ajeeb can't compute it) — integrity checked via `parth.das` parseability + file existence

## New Functions

| Function | Purpose |
|----------|---------|
| `cacheHas(name, version): int` | Check if `cache/<name>/<version>/parth.das` exists |
| `cacheGetPath(name, version): string` | Return path to cached `parth.das` |
| `cacheStore(name, version, dasPath): int` | Copy `parth.das` into versioned cache dir |
| `cacheGetLatest(name): string` | Read `cache/<name>/latest` file, return version |
| `cacheSetLatest(name, version): int` | Write version to `cache/<name>/latest` |
| `cacheInvalidate(name): int` | Delete `cache/<name>/` and recreate empty |
| `cacheIsCorrupted(name, version): int` | Check if cached `parth.das` is parseable |

## Resolver Integration

### In `resolveAllLockAware` (before `findPkgPath` call):

```
// After determining resolvedVer, before findPkgPath:
if (cacheHas(nameStr, resolvedVer) == 1) {
    pkgPath = cacheGetPath(nameStr, resolvedVer)
    // print "📦 cache hit"
} else {
    pkgPath = findPkgPath(nameStr)
    if (len(pkgPath) > 0) {
        cacheStore(nameStr, actualVer, pkgPath)
        // print "💾 cached"
    }
}
```

### Cache lookup order (new `findPkgPathCached`):

1. `packages/<name>/parth.das` (local, always authoritative)
2. `../packages/<name>/parth.das` (sibling local)
3. `/root/.parth/cache/<name>/<version>/parth.das` (versioned cache)
4. `/root/.parth/cache/<name>/parth.das` (legacy flat cache)

## Lockfile Integration

- Lockfile pins `name → version`
- Cache uses `name/version/` directories
- On resolve: lockfile provides version → cache lookup uses it
- After resolve: newly resolved packages written to versioned cache
- `generate-lockfile` command also populates cache

## Corruption Handling

- `cacheIsCorrupted`: tries `parseDasConfig` on cached `parth.das`; if parse fails, returns 1
- On corruption: skip cache entry, fall through to `findPkgPath`, re-cache on success
- `cacheInvalidate`: deletes entire `<name>/` cache dir (manual command via `parth cache clear <name>`)

## CLI Commands

- `parth cache list` — list all cached packages with versions
- `parth cache clear <name>` — invalidate a package's cache
- `parth cache status` — show cache stats (total packages, total size)

## Test Plan (Suite 13: Package Cache)

| Test | Description |
|------|-------------|
| 1 | `cacheStore` + `cacheHas` round-trip |
| 2 | `cacheGetLatest` / `cacheSetLatest` |
| 3 | `cacheIsCorrupted` detects bad parth.das |
| 4 | `cacheInvalidate` clears cache dir |
| 5 | Resolver uses cache hit (prints 📦) |
| 6 | Resolver writes to cache after fresh resolve (prints 💾) |
| 7 | `cache list` output format |
| 8 | `cache clear` removes package dir |

## Files Modified

| File | Changes |
|------|---------|
| `parth/parth_m1.ajb` | Add cache functions, integrate with resolver, add test suite, add CLI commands |
| `parth/build.sh` | No changes needed (rebuilds from monolith) |

## Verification

1. `bash parth/build.sh` — rebuild parth_m2
2. `parth_m2 test-all` — all suites pass (including new Suite 13)
3. Manual: resolve a project, check `/root/.parth/cache/` has versioned entries
4. Manual: delete a local package, verify resolver falls back to cache
5. Manual: corrupt a cached `parth.das`, verify corruption detected
