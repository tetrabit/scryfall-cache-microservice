# Security Scanning and Dependency Audit

This document describes the security scanning setup for the Scryfall Cache Microservice.

## Security Tools

### 1. cargo-audit
Checks dependencies for known security vulnerabilities from the RustSec Advisory Database.

**Installation:**
```bash
cargo install cargo-audit
```

**Usage:**
```bash
cargo audit
cargo audit --deny warnings  # Fail on any vulnerabilities
```

### 2. cargo-deny
Lints the dependency graph for security issues, license compliance, and banned crates.

**Installation:**
```bash
cargo install cargo-deny
```

**Usage:**
```bash
cargo deny check         # Check all
cargo deny check advisories  # Security advisories only
cargo deny check licenses    # License compliance only
cargo deny check bans        # Banned dependencies only
```

**Configuration:** See `deny.toml` in project root.

### 3. cargo-clippy
Static analysis tool for catching common mistakes and security anti-patterns.

**Installation:**
```bash
rustup component add clippy
```

**Usage:**
```bash
cargo clippy -- -D warnings -D clippy::all
```

### 4. Trivy (Optional)
Container image vulnerability scanner.

**Installation:**
```bash
# Linux
wget https://github.com/aquasecurity/trivy/releases/latest/download/trivy_Linux-64bit.tar.gz
tar zxvf trivy_Linux-64bit.tar.gz
sudo mv trivy /usr/local/bin/

# macOS
brew install aquasecurity/trivy/trivy
```

**Usage:**
```bash
# Build image first
docker build -t scryfall-cache:latest .

# Scan image
trivy image scryfall-cache:latest
trivy image --severity HIGH,CRITICAL scryfall-cache:latest
```

## Running Security Scans

### Quick Scan
```bash
./scripts/security-scan.sh
```

### Manual Checks

**1. Dependency vulnerabilities:**
```bash
cargo audit
```

**2. License compliance:**
```bash
cargo deny check licenses
```

**3. Security advisories:**
```bash
cargo deny check advisories
```

**4. Static analysis:**
```bash
cargo clippy -- -D warnings
```

**5. Container scan:**
```bash
docker build -t scryfall-cache:latest .
trivy image scryfall-cache:latest
```

## CI/CD Integration

Add to GitHub Actions workflow:

```yaml
name: Security Scan

on:
  pull_request:
  push:
    branches: [master]
  schedule:
    - cron: '0 0 * * *'  # Daily at midnight

jobs:
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
      
      - name: Install cargo-audit
        run: cargo install cargo-audit
      
      - name: Install cargo-deny
        run: cargo install cargo-deny
      
      - name: Run security audit
        run: cargo audit --deny warnings
      
      - name: Check advisories
        run: cargo deny check advisories
      
      - name: Check licenses
        run: cargo deny check licenses
      
      - name: Run Clippy
        run: cargo clippy -- -D warnings
```

## Configuration

### deny.toml

The `deny.toml` file configures `cargo-deny` behavior:

- **Allowed licenses:** MIT, Apache-2.0, BSD-2/3-Clause, ISC, CC0, Unlicense
- **Denied licenses:** GPL-3.0, AGPL-3.0 (copyleft)
- **Advisory handling:** Deny vulnerabilities, warn on unmaintained
- **Multiple versions:** Warn (allows major version bumps)

### License Policy

We only use permissive licenses compatible with commercial use:
- ✅ MIT, Apache-2.0, BSD variants
- ❌ GPL, AGPL (copyleft licenses)

### Vulnerability Policy

- **Critical/High vulnerabilities:** Must be fixed immediately
- **Medium vulnerabilities:** Fix within 7 days
- **Low vulnerabilities:** Fix within 30 days or document exception

## Handling Vulnerabilities

### 1. Update Dependencies
```bash
cargo update
cargo audit
```

### 2. Check for Patches
```bash
cargo outdated  # Install with: cargo install cargo-outdated
```

### 3. Document Exceptions
If a vulnerability can't be fixed immediately, document it:

```toml
# In deny.toml
[advisories]
ignore = [
    "RUSTSEC-YYYY-NNNN",  # Brief reason why ignored
]
```

### 4. Consider Alternatives
If a dependency has persistent security issues:
1. Find alternative crate
2. Vendor and patch locally
3. Rewrite the functionality

## Security Best Practices

1. **Keep dependencies updated:** Run `cargo update` regularly
2. **Minimize dependencies:** Fewer dependencies = smaller attack surface
3. **Pin versions:** Use exact versions in `Cargo.toml` for reproducibility
4. **Review new dependencies:** Check license, maintenance, security history
5. **Use workspace dependencies:** Share versions across workspace
6. **Enable security features:** Use `rustls` instead of `openssl`
7. **Regular scans:** Run security scans daily in CI
8. **Monitor advisories:** Subscribe to RustSec advisory notifications

## Resources

- [RustSec Advisory Database](https://rustsec.org/)
- [cargo-audit Documentation](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [cargo-deny Documentation](https://embarkstudios.github.io/cargo-deny/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
