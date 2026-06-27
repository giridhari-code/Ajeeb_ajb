# Release Pipeline Audit Report

**Date:** June 26, 2026
**Workflow:** `.github/workflows/release.yml`

---

## Root Causes (Why the workflow was failing)

### BUG 1 (CRITICAL): Wrong ajeebc binary path
**Lines affected:** 49, 55, 70 (linux-x86_64); 119, 129 (linux-aarch64); 177, 183, 198 (macos-arm64); 246, 256 (macos-x86_64); 304 (windows)

**Problem:** The build step copies the ajeebc binary to `ajeebc/build/ajeebc` (relative to repo root), but the bootstrap check and collect artifacts steps referenced `build/ajeebc` (root-level).

**Impact:** In a fresh CI checkout:
- `build/ajeebc` does NOT exist (it's in `.gitignore`)
- Bootstrap check fails with "No such file or directory"
- Artifact collection fails, release has no binaries

**Fix:** Changed all references from `build/ajeebc` to `ajeebc/build/ajeebc`

### BUG 2 (CRITICAL): Wrong stdlib path
**Lines affected:** 74, 133, 202, 260, 307

**Problem:** Workflow referenced `packages/ajeeb-std/*.ajb` but standard library files are at `ajeeb-lang/std/*.ajb`

**Impact:** Standard library not included in release

**Fix:** Changed all references to `ajeeb-lang/std/*.ajb`

### BUG 3: piri/build.sh missing --emit-llvm-only
**Line affected:** 43 of piri/build.sh

**Problem:** Without `--emit-llvm-only`, ajeebc attempts full compilation pipeline

**Impact:** Redundant compilation, potential build conflicts

**Fix:** Added `--emit-llvm-only` flag

### BUG 4: Release body referenced removed flag
**Line affected:** 361-362 of release.yml

**Problem:** Release notes mentioned `--build-from-source` which was removed from installer

**Impact:** Users get error following release instructions

**Fix:** Removed build-from-source section from release notes

### BUG 5: AJEEBC_PATH env var unused
**Lines affected:** 36, 106, 163, 232

**Problem:** Workflow set `AJEEBC_PATH` env var but `piri/build.sh` doesn't use it

**Impact:** Misleading configuration (build.sh uses hardcoded relative path)

**Fix:** Removed unused env var from workflow

---

## Files Modified

| File | Changes |
|------|---------|
| `.github/workflows/release.yml` | Fixed all ajeebc paths, stdlib paths, removed --build-from-source, removed unused env vars |
| `piri/build.sh` | Added `--emit-llvm-only` flag |

---

## Diff Summary

### release.yml
- 30 lines changed (20 insertions, 10 deletions)
- All `build/ajeebc` references → `ajeebc/build/ajeebc`
- All `packages/ajeeb-std/*.ajb` → `ajeeb-lang/std/*.ajb`
- Removed `AJEEBC_PATH` env vars
- Removed `--build-from-source` from release notes

### piri/build.sh
- 1 line changed
- Added `--emit-llvm-only` flag to ajeebc invocation

---

## Verification Results

| Check | Status |
|-------|--------|
| All file paths exist in repo | ✅ |
| Installer URLs match release assets | ✅ |
| SHA256SUMS includes all binaries | ✅ |
| Action versions are current | ✅ |
| YAML syntax is valid | ✅ |
| No deprecated actions | ✅ |
| Permissions set correctly | ✅ |
| All build jobs have consistent structure | ✅ |

---

## Why the fix works

1. **Path consistency:** All references to the ajeebc binary now point to `ajeebc/build/ajeebc`, which is where the build step actually places it.

2. **Stdlib inclusion:** The release now correctly includes all standard library files from `ajeeb-lang/std/`.

3. **Clean pipeline:** The `--emit-llvm-only` flag ensures piri builds don't conflict with the LLVM pipeline.

4. **User experience:** Release notes no longer reference removed features.

---

## Ready to push: YES
