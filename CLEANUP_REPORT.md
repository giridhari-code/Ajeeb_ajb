# Cleanup Report

## Date: June 25, 2026

## Summary
Reorganized the repository structure for v1.0 release. Moved 71 files to logical directories, removed generated/duplicate files, and verified all builds and tests pass.

## Directory Structure (Before → After)

### Root Level
**Before:** 97 entries (68 .md files, scripts, test files, generated files)
**After:** 14 entries (clean, organized)

```
ajeeb_compiler/
├── AGENTS.md              # Agent instructions
├── README.md              # Project documentation
├── ajeebc/                # Main compiler source
├── ajeeb-lang/            # Language specification
├── build/                 # Build artifacts (gitignored)
├── compiler/              # Ajeeb compiler source
├── docs/                  # Documentation
│   ├── design/            # Design docs & roadmaps (13 files)
│   ├── reports/           # Reports & audits (58 files)
│   └── *.md               # Core documentation (6 files)
├── parth/                 # Package manager (Ajeeb)
├── parthi/                # MIR interpreter
├── runtime/               # C runtime
├── scripts/               # Helper scripts
│   ├── install.sh
│   └── install.ps1
├── src/                   # Source examples
├── takshak/               # Web framework
└── tests/                 # Test files
    ├── hello-world/       # Example project
    ├── *.ajb              # Test files
    └── bootstrap_check.sh # Verification script
```

## Files Moved

### Reports → docs/reports/ (58 files)
```
B1_PROGRESS_REPORT.md, BACKEND_CONTROLLER_REPORT.md, BLOCK_EMISSION_AUDIT.md,
BOOTSTRAP_PARITY.md, BOOTSTRAP_VERIFICATION.md, C_FINAL_REPORT.md,
C1_REPORT.md, CARGO_REMOVAL_REPORT.md, CHANGELOG_v0.2.md,
COMPILER_SCALE_REPORT.md, E1_REPORT.md, E2_REPORT.md, E3_REPORT.md,
FEATURE_PARITY.md, FEATURE_VERIFICATION.md, FINAL_AUDIT.md,
FUNCTION_EMISSION_REPORT.md, FWDDECL_AUDIT.md, HOTPATH_REPORT.md,
IDENTIFIER_CORRUPTION_REPORT.md, INTERPRETER_PARITY.md, KNOWN_ISSUES.md,
LLVM_PARITY.md, LLVM_PARITY_MATRIX.md, M1_1_REPORT.md, M1_2_REPORT.md,
M1_3_FIX_REPORT.md, M1_3_HANG_REPORT.md, M1_4_REPORT.md, M2_1_REPORT.md,
M2_2_REPORT.md, M2_3_REPORT.md, M2_4_REPORT.md, M2_5_REPORT.md,
MEMORY_CORRUPTION_REPORT.md, METHOD_DISPATCH_REPORT.md, PARTH_AUDIT.md,
PARTH_M0_REPORT.md, PARTH_M1_AUDIT.md, PERFORMANCE_REPORT.md,
PURE_AJEEB_GAP_ANALYSIS.md, RELEASE_AUDIT.md, ROOT_BLOCKER.md,
RUNTIME_DECLARATION_AUDIT.md, SELF_HOSTED_HOTPATH.md, STAGE_A_AUDIT.md,
STAGE_A_BLOCKERS_REPORT.md, STAGE_A_COMPLETE.md, STAGE_A_FINAL_GAP_REPORT.md,
STAGE_A_REMAINING_WORK.md, STAGE_C_COMPLETE.md, STAGE_C_PROGRESS.md,
STAGE_D_COMPLETE.md, STAGE_E_COMPLETE.md, STAGE_F_COMPLETE.md,
STAGE_G_COMPLETE.md, STRUCT_IMPLEMENTATION_REPORT.md, STRUCT_USAGE_REPORT.md
```

### Design/Plan → docs/design/ (13 files)
```
M2_CACHE_PLAN.md, M2_LOCKFILE_PLAN.md, M2_REGISTRY_PLAN.md,
M2_SIGNING_PLAN.md, M2_WORKSPACE_PLAN.md, PARTH_M1_PLAN.md,
PARTH_REWRITE_PLAN.md, PURE_AJEEB.md, PURE_AJEEB_ROADMAP.md,
ROADMAP_v0.3.md, STAGE_A_PLAN.md, STAGE_C_PLAN.md, ajeeblangdoc.md
```

### Scripts → scripts/ (2 files)
```
install.sh, install.ps1
```

### Test Files → tests/ (5 entries)
```
debug_entry.ajb, leetcode.ajb, max_subarray.ajb,
max_subarray_c.ajb, hello-world/
```

## Files Removed

| File | Reason |
|------|--------|
| `output.c` | Generated stub file |
| `ajeebc/output.c` | Generated file |
| `ajeebc/test_output.c` | Generated file |
| `ajeebc/hello.ajb` | Test file ( belongs in tests/) |
| `ajeebc/--skip-run` | Flag-like generated file |
| `ajeebc/-o` | Flag-like generated file |
| `ajeebc/--dry-run` | Flag-like generated file |
| `ajeebc/--gcc` | Flag-like generated file |
| `ajeebc/--interpret` | Flag-like generated file |
| `--skip-run` (root) | Flag-like generated file |
| `-o` (root) | Flag-like generated file |
| `parth/build/ap*.txt` | ~11,000 junk files |

## Paths Updated

| File | Change |
|------|--------|
| `README.md` | `install.sh` → `scripts/install.sh` |
| `.github/workflows/release.yml` | Updated curl URLs for install scripts |
| `AGENTS.md` | Updated quick start commands |
| `ajeebc/crates/parth/src/commands/build.rs` | Updated error messages |

## Verification

| Test | Status |
|------|--------|
| `make native` | ✅ Pass |
| `make test` | ✅ 6/6 pass |
| `bootstrap_check.sh` | ✅ Pass |
| `parth run` | ✅ Pass |
| `parth bootstrap` | ✅ Pass |

## Disk Space Saved
- Removed ~11,000 junk files from `parth/build/`
- Removed generated files from root and ajeebc/
- Total cleanup: ~50 MB of junk removed

## Conclusion
Repository is now organized for v1.0 release:
- Clean root directory (14 entries vs 97)
- Logical folder structure
- No generated/duplicate files
- All builds and tests pass
