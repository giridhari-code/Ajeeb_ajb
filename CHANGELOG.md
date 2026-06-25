# Changelog

All notable changes to Ajeeb will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.0] - 2026-06-25

### Added

#### Compiler
- Self-hosting bootstrap chain (Gen0 → Gen1 → Gen2)
- LLVM backend (primary, production-ready)
- C fallback backend (for platforms without LLVM)
- Interpreter mode (--interpret flag)
- Multi-file imports with proper scoping
- Forward declarations for imported functions
- Type checking with trait bounds
- Pattern matching (match expressions)
- Generics with type parameters
- Classes, structs, enums, traits
- Closures and first-class functions
- Dynamic arrays with built-in functions
- String operations (concat, substring, indexOf, etc.)
- Mathematical operations (abs, max, min, pow, etc.)
- File I/O operations (readFile, writeFile, etc.)
- 65+ built-in functions

#### Parth (Package Manager)
- Project initialization (parth init)
- Build system (parth build)
- Test runner (parth test)
- Interpreter mode (parth run)
- Self-hosting verification (parth bootstrap)
- Package registry support
- Dependency resolution
- Workspace support
- Lockfile support
- Cache management
- Package signing (Ed25519)
- Security audit

#### MIR Interpreter
- Direct AST execution
- No LLVM required
- Fast development cycle

#### Standard Library
- io.ajb — Input/output operations
- math.ajb — Mathematical functions
- string.ajb — String utilities
- array.ajb — Array utilities
- fs.ajb — File system operations
- result.ajb — Result/Option types
- collections.ajb — Data structures (Stack, Queue)

#### Documentation
- Language specification
- Compiler architecture
- LLVM backend documentation
- MIR documentation
- Parth documentation
- Bootstrap guide

### Changed
- Default backend is now LLVM (was C)
- Compiler produces 99% smaller binaries (15MB → 142KB)
- Bootstrap uses pre-built binaries (no Rust required for normal development)
- Repository reorganized for v1.0 release

### Fixed
- Import handler savePos clobbered by nextTok
- Forward declarations missing for imported functions
- LLVM codegen string == does pointer comparison
- Parth parser slot mapping
- LLVM runtime strSet missing null-termination
- Goto instruction argument order
- While loop exit block overlap with inner if-else

### Removed
- Duplicate ajeebBootstrap/ directory (saved 1.2 GB)
- Nested target/ directories (saved ~1 GB)
- Generated test files
- Flag-like generated files

## [0.2.0] - 2026-06-20

### Added
- Initial self-hosting support
- Basic LLVM backend
- C fallback backend
- Package manager (Parth)
- Standard library

### Fixed
- Various compilation bugs
- Memory corruption issues
- Type checking errors

## [0.1.0] - 2026-06-15

### Added
- Initial release
- Basic compiler (Rust-based)
- Lexer and parser
- Interpreter mode
