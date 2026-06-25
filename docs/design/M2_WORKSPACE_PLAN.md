# M2.4 Workspace Support — Implementation Plan

## Goal

Add workspace support to Parth, enabling multiple packages to live in one repository with shared dependencies, a single lockfile, and coordinated builds.

## Design

### Workspace Manifest Format

A workspace root has a `parth.das` with a `[workspace]` section:

```ini
[workspace]
name = "my-workspace"
members = "packages/*"

[dependencies]
shared-util = "^1.0.0"
shared-json = "^2.0.0"
```

Member packages have their own `parth.das`:

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
  parth.lock             # Shared lockfile for all members
  packages/
    pkg-a/
      parth.das          # [package] + [dependencies]
      src/main.ajb
    pkg-b/
      parth.das
      src/main.ajb
```

### Resolution Rules

1. **Shared deps**: `[dependencies]` in workspace root apply to all members
2. **Member deps**: Each member's `[dependencies]` are local to that package
3. **Merge**: Resolution merges workspace deps + member deps (member wins on conflict)
4. **Lockfile**: Single `parth.lock` at workspace root covers all packages
5. **Cache**: Existing cache system is reused unchanged

### New CLI Commands

| Command | Description |
|---------|-------------|
| `parth workspace init <name>` | Create workspace root + packages/ dir |
| `parth workspace add <pkg>` | Add member package |
| `parth workspace build [pkg]` | Build all members or specific one |
| `parth workspace list` | List member packages |
| `parth workspace resolve` | Resolve all workspace dependencies |

### Backward Compatibility

- A `parth.das` with `[package]` (not `[workspace]`) is a single-package project
- All existing commands (`build`, `add`, `remove`, etc.) continue to work unchanged
- Workspace commands are new and don't interfere with single-package mode

## Buffer Slot Layout

### Existing slots (unchanged):
- Slots 0-14: package config fields
- Slot 10: depCount
- Slots 200+: dependency entries
- Slot 750+: lockfile entries

### New slots for workspace:
- Slot 800: workspace mode flag (0 = single package, 1 = workspace)
- Slot 801: member count
- Slots 810+: member paths (2 slots each: pathOff, pathLen, max 20 members = 80 slots)

## Implementation Steps

### Phase 1: Parser Extension
1. Add `[workspace]` section recognition (section code = 5)
2. Parse `members` field from `[workspace]` section
3. Add `isWorkspace(buf)` check
4. Add `getWorkspaceMembers(buf, src)` to return member paths

### Phase 2: Workspace Functions
1. `workspaceInit(name)` — create workspace root with [workspace] + packages/ dir
2. `workspaceAddMember(memberPath)` — add member to workspace members list
3. `workspaceListMembers(buf, src)` — return list of member package paths
4. `workspaceReadMemberDas(memberPath)` — parse member's parth.das, merge with workspace deps
5. `workspaceResolve(buf, src)` — resolve all workspace dependencies
6. `workspaceBuild(buf, src, member)` — build one or all members

### Phase 3: CLI Integration
1. Add `workspace` command dispatcher
2. Route `workspace init/add/build/list/resolve` subcommands
3. Add `testWorkspace()` test suite

### Phase 4: Test Suite
1. Test workspace init creates correct structure
2. Test workspace list shows members
3. Test workspace resolve merges deps
4. Test workspace build compiles all members
5. Test backward compat (single-package still works)

## Test Plan

```ajb
function testWorkspace(): int {
    set passed: int = 0;
    set failed: int = 0;
    
    // Test 1: workspace init creates structure
    // Test 2: workspace add member
    // Test 3: workspace list members
    // Test 4: workspace resolve with shared + member deps
    // Test 5: workspace build all
    // Test 6: workspace build specific member
    // Test 7: backward compat — single-package project unchanged
    
    // Report results
    return (failed > 0) ? 1 : 0;
}
```

## Files to Modify

1. `parth/parth_m1.ajb` — main source (add workspace functions + CLI + tests)
2. No other files need modification (self-contained in monolith)

## Risks

- **File size**: parth_m1.ajb is ~3958 lines; adding workspace could push to ~4500. Still under 140K limit.
- **Complexity**: Workspace dep merging is the most complex part; must not break single-package resolution.
- **Test isolation**: Workspace tests create filesystem state; must clean up properly.

## Verification

1. All existing 14 test suites pass
2. New workspace test suite passes
3. Bootstrap check passes (if Rust compiler available)
4. Single-package `parth build` still works
