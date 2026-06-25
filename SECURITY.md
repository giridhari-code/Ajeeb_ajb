# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Ajeeb, please report it responsibly.

**Do NOT create a public GitHub issue for security vulnerabilities.**

Instead, please email: [security@ajeeb-lang.dev](mailto:security@ajeeb-lang.dev)

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 1 week
- **Fix or mitigation**: Within 30 days (depending on severity)

## Scope

The following are in scope:
- Compiler vulnerabilities (code execution, crashes)
- Runtime vulnerabilities (memory safety, buffer overflows)
- Package manager vulnerabilities (supply chain attacks)
- Dependency vulnerabilities

The following are out of scope:
- Denial of service (DoS) attacks
- Social engineering
- Physical attacks

## Security Features

### Package Signing

Parth supports Ed25519 package signing:
- Packages can be signed by maintainers
- Signatures are verified on install
- Key generation via `parth keygen`

### Sandboxing

- Compiler runs in user space
- No privileged operations
- File system access limited to project directory

### Memory Safety

- C runtime uses bounds checking
- Arena allocator prevents leaks
- Reference counting for dynamic allocations

## Dependencies

We regularly audit dependencies:
- Rust crates: checked via `cargo audit`
- C libraries: minimal dependencies (libc, libdl, libm)
- No unsafe dependencies without review

## Updates

Security updates are released as:
- **Critical**: Immediate patch release
- **High**: Within 1 week
- **Medium**: Within 30 days
- **Low**: Next minor release

## Contact

For security inquiries: [security@ajeeb-lang.dev](mailto:security@ajeeb-lang.dev)

For general issues: [GitHub Issues](https://github.com/giridhari-code/Ajeeb_ajb/issues)
