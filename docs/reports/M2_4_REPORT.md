# M2.4 Workspace Support — Implementation Report

**Date:** 2026-06-24  
**Status:** ✅ Complete

## Summary

Added workspace support to Parth, enabling multiple packages to live in one repository with shared dependencies and coordinated builds.

## What Was Implemented

### Workspace Manifest Format

A workspace root has a `[workspace]` section in `parth.das`:

```ini
[workspace]
name = "my-workspace"
members = "packages/*"

[dependencies]
shared-util = "^1.0.0"
```

Member packages have standard `[package]` sections:

```ini
[package]
name = "pkg-a"
version = "0.1.0"

[dependencies]
local-dep = "^1.0.0"
```

### Directory Layout

```
workspace-root/
  parth.das              # [workspace] + [dependencies]
  parth.lock             # Shared lockfile
  packages/
    pkg-a/
      parth.das          # [package] + [dependencies]
      src/main.ajb
    pkg-b/
      parth.das
      src/main.ajb
```

### New CLI Commands

| Command | Description |
|---------|-------------|
| `parth workspace init <name>` | Create workspace root + packages/ dir |
| `parth workspace add <pkg>` | Add member package to workspace |
| `parth workspace list` | List member packages |
| `parth workspace resolve` | Resolve all workspace dependencies |
| `parth workspace build [pkg]` | Build all members or specific one |

### Buffer Slot Layout

| Slot | Purpose |
|------|---------|
| 800 | Workspace mode flag (0=single, 1=workspace) |
| 801 | Member count |
| 810+ | Member paths (2 slots each: off, len, max 20 members) |

### New Functions

| Function | Line | Description |
|----------|------|-------------|
| `isWorkspace(buf)` | 2449 | Check if workspace mode is enabled |
| `getMemberCount(buf)` | 2453 | Get number of workspace members |
| `getMemberPath(buf, src, idx)` | 2457 | Get member path by index |
| `parseWorkspaceConfig(path)` | 2465 | Parse workspace parth.das with member tracking |
| `cmdWorkspaceInit(name)` | 2547 | Initialize new workspace |
| `cmdWorkspaceAdd(name)` | 2566 | Add member package |
| `cmdWorkspaceList()` | 2606 | List workspace members |
| `cmdWorkspaceResolve()` | 2641 | Resolve workspace dependencies |
| `cmdWorkspaceBuild(arg)` | 2689 | Build workspace members |
| `testWorkspace()` | 4350 | Test suite (8 tests) |

## Test Results

### Workspace Test Suite (8 tests)

| Test | Description | Status |
|------|-------------|--------|
| 1 | workspace init creates structure | ✓ |
| 2 | workspace add member | ✓ |
| 3 | add second member | ✓ |
| 4 | parse workspace config | ✓ |
| 5 | workspace member paths | ✓ |
| 6 | single-package not workspace | ✓ |
| 7 | multi-member CSV parsing | ✓ |
| 8 | backward compatibility | ✓ |

### Full Test Suite

| Suite | Status |
|-------|--------|
| 1. Regression | ✓ |
| 2. SemVer | ✓ |
| 3. Dependency Tree | ✓ |
| 4. CSV Helpers | ✓ |
| 5. Config Parser | ✓ |
| 6. E2E: new | ✓ |
| 7. E2E: init | ✓ |
| 8. E2E: add/remove | ✓ |
| 9. E2E: build | ✓ |
| 10. E2E: run | ✓ |
| 11. E2E: tree | ✓ |
| 12. Lockfile v2 | ✓ |
| 13. Package Cache | ✓ |
| 14. Registry | ✓ |
| **15. Workspace** | **✓** |

**RESULT: ALL PASS (15/15 suites)**

### Other Tests

- Cargo tests: 16/16 pass
- ajeebc tests: 6/6 pass

## Key Design Decisions

1. **Separate parser**: `parseWorkspaceConfig()` vs `parseDasConfig()` — workspace parser reads `[workspace]` section and member paths, while standard parser resets workspace fields. This ensures single-package projects are never误detected as workspaces.

2. **Workspace fields in standard buffer**: Uses slots 800-890 in the same state buffer, avoiding need for separate data structures.

3. **CSV-based member storage**: Member paths stored as offset+length pairs in buffer slots, same pattern as dependency entries.

4. **Backward compatible**: Single-package projects work unchanged. `parseDasConfig()` resets workspace fields (800, 801) to 0.

5. **Member builds via exec**: `workspace build` runs `parth build` in each member directory, leveraging existing build system.

## Files Modified

1. `parth/parth_m1.ajb` — Added workspace functions, CLI commands, and test suite
   - Added `bw(buf, 800, 0)` and `bw(buf, 801, 0)` to `parseDasConfig()` for workspace reset
   - Added 10 new workspace functions (~200 lines)
   - Added workspace CLI dispatch (~30 lines)
   - Added testWorkspace function (~120 lines)

## Verification

1. All 15 test suites pass
2. All 16 cargo tests pass
3. All 6 ajeebc tests pass
4. Workspace CLI commands work end-to-end
5. Single-package projects unaffected
