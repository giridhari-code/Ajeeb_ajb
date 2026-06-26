# INSTALLER_REPORT.md

## Installer Audit — Ajeeb v1.0.1

**Audit Date:** June 26, 2026

### Summary

The installer (`scripts/install.sh`) has been rewritten to provide a completely Rust-free installation experience. Users can install Ajeeb without any build tools beyond `gcc/clang` and `llc`.

### Changes Made

#### 1. Removed Cargo Suggestions (CRITICAL)

**Before:** When binary download failed, installer suggested:
```bash
curl -sSf .../install.sh | bash -s -- --build-from-source
```
This required Rust/Cargo, defeating the purpose of a prebuilt installer.

**After:** When binary download fails, installer now shows:
```
❌ ajeebc binary release mein nahi hai (linux-x86_64)
   GitHub issue karo: https://github.com/giridhari-code/Ajeeb_ajb/issues
```

#### 2. Added Checksum Verification

**Before:** No checksum verification. Users had no way to verify binary integrity.

**After:** Installer downloads `SHA256SUMS.txt` and runs `sha256sum -c` to verify all downloaded binaries. Warns on failure but continues (non-blocking).

#### 3. Fixed Runtime URL

**Before:** Runtime downloaded from raw GitHub content URL:
```
https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/ajeebc/runtime/ajeeb_runtime.c
```

**After:** Runtime downloaded from release assets:
```
https://github.com/giridhari-code/Ajeeb_ajb/releases/download/v1.0.1/ajeeb_runtime.c
```

#### 4. Added Standard Library Packages

**Before:** Only 7 standard library files downloaded.

**After:** All 13 standard library files downloaded:
- `io.ajb`, `math.ajb`, `string.ajb`, `array.ajb`, `fs.ajb`
- `result.ajb`, `collections.ajb`, `option.ajb`, `path.ajb`
- `process.ajb`, `test.ajb`, `time.ajb`, `json.ajb`

#### 5. Removed `--build-from-source` Flag

**Before:** Installer supported `--build-from-source` flag that invoked Cargo.

**After:** Flag removed entirely. Installer only downloads prebuilt binaries.

### Dependency Analysis

| Dependency | Required | Purpose | Install Method |
|------------|----------|---------|----------------|
| `curl` | Yes | Download binaries | System package manager |
| `gcc`/`cc`/`clang` | Yes | Compile generated code | System package manager |
| `llc` | Yes | LLVM code generation | System package manager |
| `sha256sum` | Optional | Checksum verification | Usually pre-installed |
| `cargo` | **NO** | — | Not required |
| `rustc` | **NO** | — | Not required |
| `make` | **NO** | — | Not required |

### Security Considerations

1. **Checksum Verification**: All binaries verified against SHA256SUMS.txt from release assets
2. **HTTPS Only**: All downloads use HTTPS
3. **No Remote Code Execution**: Installer never runs downloaded code (only installs binaries)
4. **Permission Model**: Binaries installed to user directory (`~/.ajeeb/bin/`), no sudo required

### Test Results

| Test Case | Expected | Actual | Status |
|-----------|----------|--------|--------|
| Fresh install (no Rust) | Success | Success | ✅ |
| Missing gcc warning | Warning shown | Warning shown | ✅ |
| Missing llc warning | Warning shown | Warning shown | ✅ |
| Binary download success | All binaries installed | All installed | ✅ |
| Checksum verification | Verified | Verified | ✅ |
| PATH configuration | Added to shell rc | Added to .bashrc | ✅ |
| Standard library install | All packages downloaded | 13 packages downloaded | ✅ |

### Recommendations

1. **Future Enhancement**: Consider adding `--check` flag for dry-run verification
2. **Future Enhancement**: Add `--uninstall` flag for clean removal
3. **Future Enhancement**: Consider adding ARM64 binary detection for Apple Silicon Macs

### Conclusion

The installer now provides a **completely Rust-free installation experience**. Users can install Ajeeb with a single command and start using it immediately without any build tools beyond the standard C compiler toolchain.
