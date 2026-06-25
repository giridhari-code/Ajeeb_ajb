# M2.3 Registry Support — Implementation Plan

## Goal

Add local filesystem-based registry support to Parth, enabling package publishing, discovery, and installation from a shared registry directory. Designed to be extensible to HTTP later.

## Current State

- **Resolver**: BFS with lockfile pinning, finds packages in `packages/`, `../packages/`, cache
- **Cache**: Versioned directory layout at `/root/.parth/cache/<name>/<version>/`
- **Lockfile**: v2 format with `[name]\nversion = "X.Y.Z"`
- **Package acquisition**: `fetchPackage` exists (git clone) but is dead code
- **No native HTTP**: all network via `exec(curl)` or `exec(git)`

## Design Constraints

- No native HTTP client — use `exec(curl)` for remote, direct file I/O for local
- No JSON parser — use `parth.das`-compatible key=value format
- No SHA-256 — use file size + line count as lightweight integrity check
- No Ed25519 signing — defer to future milestone
- Reuse existing cache layout and lockfile format

## Registry Layout

```
/root/.parth/registry/
├── config.toml              # Registry config (URL, auth token)
├── index/
│   └── <package-name>       # Version list file
│       ├── versions         # One version per line: "1.0.0\n1.1.0\n"
│       └── <version>/
│           ├── parth.das    # Package metadata
│           └── src/         # Package source files
└── auth.json                # Auth token (optional, for remote)
```

### Local Registry (MVP)

For local/shared filesystem registries:
- `/root/.parth/registry/` is the registry root
- `index/<name>/versions` lists available versions
- `index/<name>/<version>/parth.das` stores package metadata
- `index/<name>/<version>/src/` stores package source
- No HTTP needed — direct file copy

### Remote Registry (Future)

For HTTP-based registries:
- `config.toml` stores `registry = "https://registry.parth.dev"`
- `parth install` uses `curl` to fetch packages
- `parth publish` uses `curl` to upload packages

## New Functions

### Registry Config

| Function | Purpose |
|----------|---------|
| `regGetConfig(key): string` | Read registry config value |
| `regSetConfig(key, value): int` | Write registry config value |
| `regGetRoot(): string` | Return registry root path |

### Package Index

| Function | Purpose |
|----------|---------|
| `regHasPackage(name): int` | Check if package exists in registry |
| `regHasVersion(name, version): int` | Check if specific version exists |
| `regListVersions(name): string` | Return CSV of available versions |
| `regAddVersion(name, version): int` | Add version to index |
| `regRemoveVersion(name, version): int` | Remove version from index |

### Package Storage

| Function | Purpose |
|----------|---------|
| `regPublish(name, version, srcDir): int` | Publish package to registry |
| `regFetch(name, version, destDir): int` | Fetch package from registry to dest |
| `regGetMetadata(name, version): string` | Read package's parth.das from registry |

### Registry-Aware Resolution

| Function | Purpose |
|----------|---------|
| `findPkgPathReg(name, version): string` | Search with registry fallback |
| `regInstall(name, version): int` | Install package from registry to local |

## CLI Commands

| Command | Description |
|---------|-------------|
| `parth publish` | Publish current package to registry |
| `parth install <name[@ver]>` | Install package from registry |
| `parth search <query>` | Search registry for packages |
| `parth registry list` | List all packages in registry |
| `parth registry info <name>` | Show package info from registry |

## Integration with Existing Systems

### Resolver Integration

`findPkgPathReg(name, version)` lookup order:
1. `packages/<name>/parth.das` (local)
2. `../packages/<name>/parth.das` (sibling)
3. `/root/.parth/cache/<name>/<version>/parth.das` (cache)
4. `/root/.parth/registry/index/<name>/<version>/parth.das` (registry)

### Cache Integration

- `regFetch` copies package to cache after fetching from registry
- Cache acts as local mirror of registry packages

### Lockfile Integration

- Lockfile records `name → version` (unchanged)
- Registry provides version → source mapping
- `parth install` resolves + fetches + updates lockfile

## Test Plan (Suite 14: Registry)

| Test | Description |
|------|-------------|
| 1 | `regHasPackage` / `regHasVersion` |
| 2 | `regListVersions` returns CSV |
| 3 | `regPublish` stores package in registry |
| 4 | `regFetch` copies package from registry |
| 5 | `findPkgPathReg` finds registry package |
| 6 | `parth publish` command works |
| 7 | `parth install` command works |
| 8 | `parth search` command works |
| 9 | `parth registry list` command works |
| 10 | `parth registry info` command works |
| 11 | Publish + install round-trip |
| 12 | Version conflict detection |

## Files Modified

| File | Changes |
|------|---------|
| `parth/parth_m1.ajb` | Add registry functions, CLI commands, test suite |

## Verification

1. `bash parth/build.sh` — rebuild parth_m2
2. `parth_m2 test-all` — all suites pass (including new Suite 14)
3. Manual: `parth publish` a test package, verify it appears in registry
4. Manual: `parth install` a package from registry, verify it's in cache
5. Manual: `parth search` finds published packages
