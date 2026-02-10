# Cargo Build Optimization Guide

## Current Configuration

The `.cargo/config.toml` configures:

### 1. Parallel Compilation (16 cores)
```toml
[build]
jobs = 16
```
Cargo uses multithreaded compilation by default, this makes it explicit.

### 2. Incremental Compilation
```toml
incremental = true
```
Enabled by default for debug. Recompiles only changed code.

### 3. Split Debug Info
```toml
[profile.dev]
split-debuginfo = "unpacked"
```
Reduces linking time by separating debug information.

## Performance

Current build times (16 cores):
- **Incremental rebuild**: ~4-5 seconds
- **Clean debug build**: ~40-60 seconds  
- **Clean release build**: ~2-3 minutes

## Further Optimizations

### Install Mold Linker (Recommended)
```bash
sudo apt install mold
```
Reduces linking time by 3-10x. Uncomment the mold config in `.cargo/config.toml`.

### Install LLD Linker
```bash
sudo apt install lld
```
Alternative faster linker (2-5x improvement).

### Use Sccache
```bash
cargo install sccache
export RUSTC_WRAPPER=sccache
```
Caches compiled dependencies across projects.

### Cargo Watch
```bash
cargo install cargo-watch
cargo watch -x build  # Auto-rebuild on changes
```

## References
- [Cargo Config](https://doc.rust-lang.org/cargo/reference/config.html)
- [Mold Linker](https://github.com/rui314/mold)
- [Fast Rust Builds](https://endler.dev/2020/rust-compile-times/)
