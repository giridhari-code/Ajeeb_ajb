# Contributing to Ajeeb

Thank you for your interest in contributing to Ajeeb! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and inclusive in all interactions. We're building a fun, bilingual programming language — let's keep the community welcoming!

## How to Contribute

### Reporting Bugs

1. Check existing issues to avoid duplicates
2. Create a new issue with:
   - Clear title describing the problem
   - Steps to reproduce
   - Expected vs actual behavior
   - Ajeeb version and platform

### Suggesting Features

1. Check existing issues/discussions
2. Create a new issue with:
   - Clear description of the feature
   - Use cases
   - Implementation ideas (if any)

### Submitting Changes

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run tests: `cd ajeebc && make test`
5. Run bootstrap check: `bash tests/bootstrap_check.sh`
6. Commit with clear message
7. Push and create a Pull Request

## Development Setup

### Prerequisites

- Rust (for bootstrap compiler only)
- GCC or Clang
- LLVM (llc)

### Building

```bash
# Clone the repository
git clone https://github.com/giridhari-code/Ajeeb_ajb
cd Ajeeb_ajb

# Build the compiler (requires Rust, one-time only)
cd ajeebc
make rust

# Build native compiler (no Rust needed after first build)
make native

# Run tests
make test

# Verify self-hosting
make bootstrap
```

### Project Structure

```
ajeebc/           # Compiler source
├── compiler/     # Ajeeb compiler source
├── runtime/      # C runtime
├── tests/        # Test files
└── crates/       # Rust crates
parth/            # Package manager
parthi/           # MIR interpreter
tests/            # Integration tests
docs/             # Documentation
```

## Coding Standards

### Ajeeb Code

- Use clear, descriptive variable names
- Add comments for complex logic
- Follow existing code style
- Include error messages in Hinglish

### Rust Code

- Follow Rust conventions
- Add documentation comments
- Run `cargo clippy` for linting
- Run `cargo fmt` for formatting

### C Code

- Follow existing style (K&R-ish)
- Add comments for complex sections
- Ensure memory safety

## Testing

### Running Tests

```bash
# Core tests
cd ajeebc && make test

# Bootstrap check
bash tests/bootstrap_check.sh

# Parth commands
parth test
parth bootstrap
```

### Writing Tests

- Add test files to `ajeebc/tests/` or `tests/`
- Name tests descriptively
- Include expected output in comments
- Test both success and error cases

## Documentation

- Update README.md for user-facing changes
- Update docs/ for technical changes
- Add examples for new features
- Update CHANGELOG.md

## Pull Request Guidelines

1. **One feature per PR** — keep changes focused
2. **Clear description** — explain what and why
3. **Tests included** — add or update tests
4. **Documentation updated** — update relevant docs
5. **All tests pass** — run `make test` and `bootstrap_check.sh`
6. **No breaking changes** — unless discussed in advance

## Release Process

1. Update version in relevant files
2. Update CHANGELOG.md
3. Create release notes
4. Tag the release
5. Create GitHub release with binaries

## Questions?

Feel free to open an issue for any questions about contributing!

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
