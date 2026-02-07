# Contributing to Scryfall Cache Microservice

Thank you for your interest in contributing! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct:

- Be respectful and inclusive
- Welcome newcomers and help them learn
- Focus on what is best for the community
- Show empathy towards other community members
- Be patient with questions and issues

## How to Contribute

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates.

**Good bug reports include:**
- Clear, descriptive title
- Exact steps to reproduce
- Expected behavior
- Actual behavior
- Your environment (OS, Rust version, Docker version)
- Relevant logs or error messages
- Screenshots if applicable

**Bug report template:**
```markdown
**Describe the bug**
A clear description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Start services with '...'
2. Send request '....'
3. See error

**Expected behavior**
What you expected to happen.

**Actual behavior**
What actually happened.

**Environment:**
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.85.0]
- Docker version: [e.g., 24.0.0]

**Logs**
```
[paste relevant logs here]
```

**Additional context**
Any other context about the problem.
```

### Suggesting Features

We love feature suggestions! Please:

1. Check if the feature is already in BACKLOG.md
2. Create an issue with the "enhancement" label
3. Describe the problem your feature would solve
4. Describe your proposed solution
5. Consider alternatives
6. Provide examples of how it would work

### Pull Requests

#### Before Starting
1. Check existing PRs to avoid duplicate work
2. For large changes, create an issue first to discuss
3. Fork the repository
4. Create a feature branch from `main`

#### Development Process

1. **Set up development environment**
```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/scryfall-cache-microservice
cd scryfall-cache-microservice

# Add upstream remote
git remote add upstream https://github.com/ORIGINAL_OWNER/scryfall-cache-microservice

# Create feature branch
git checkout -b feature/your-feature-name
```

2. **Write code**
- Follow Rust style guide
- Write tests for new functionality
- Update documentation
- Keep commits focused and atomic

3. **Test your changes**
```bash
# Run tests
cargo test

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Build
cargo build --release

# Test with Docker
docker-compose up -d
curl http://localhost:8080/health
```

4. **Commit your changes**
```bash
git add .
git commit -m "feat: add new feature"
```

Use conventional commit format:
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Adding tests
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Maintenance tasks

5. **Push and create PR**
```bash
git push origin feature/your-feature-name
```

Then create a pull request on GitHub.

#### Pull Request Guidelines

**Your PR should:**
- Have a clear, descriptive title
- Reference any related issues
- Include a description of what changed and why
- Include tests for new functionality
- Update documentation if needed
- Pass all CI checks
- Be focused on a single concern

**PR template:**
```markdown
## Description
Brief description of the changes.

## Related Issues
Fixes #123
Related to #456

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex code
- [ ] Documentation updated
- [ ] No new warnings
- [ ] Tests added/updated
- [ ] All tests passing
```

## Development Guidelines

### Code Style

We follow Rust community standards:

**Use rustfmt:**
```bash
cargo fmt
```

**Use clippy:**
```bash
cargo clippy -- -D warnings
```

**Naming conventions:**
- Functions: `snake_case`
- Types: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`

**Error handling:**
- Use `Result<T, E>` for recoverable errors
- Use `anyhow::Result` for application errors
- Provide context with `.context()`
- Don't use `unwrap()` in production code

**Documentation:**
- Add rustdoc comments for public APIs
- Include examples in documentation
- Document error conditions
- Explain non-obvious code

### Testing Guidelines

**Write tests for:**
- All new features
- Bug fixes
- Edge cases
- Error conditions

**Test organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = ...;

        // Act
        let result = function(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

**Integration tests:**
```rust
// tests/integration/api_test.rs
use scryfall_cache::*;

#[tokio::test]
async fn test_api_endpoint() {
    // Test implementation
}
```

### Documentation Guidelines

**Update when:**
- Adding new features
- Changing behavior
- Adding configuration options
- Modifying APIs

**Documentation files:**
- `README.md` - Overview and getting started
- `DEVELOPMENT.md` - Development details
- `BACKLOG.md` - Future features
- `TODO.md` - Immediate tasks
- `CHANGELOG.md` - Version history

### Commit Guidelines

**Good commit messages:**
```
feat: add GraphQL API endpoint

- Implement GraphQL schema
- Add resolver for card queries
- Add tests for GraphQL endpoint
- Update documentation

Closes #123
```

**Bad commit messages:**
```
fix stuff
WIP
update
asdf
```

**Commit message structure:**
```
<type>(<scope>): <subject>

<body>

<footer>
```

### Review Process

**All PRs require:**
1. Passing CI checks
2. At least one approval
3. No unresolved comments
4. Up-to-date with main branch

**Reviewers will check:**
- Code quality
- Test coverage
- Documentation
- Performance impact
- Security implications
- API compatibility

## First-Time Contributors

Welcome! Here are good issues to start with:

**Good first issues:**
- Documentation improvements
- Adding tests
- Fixing typos
- Small bug fixes
- Adding examples

**How to find them:**
- Look for `good-first-issue` label
- Check TODO.md for quick wins
- Ask in discussions

**Getting help:**
- Join our Discord
- Comment on the issue
- Ask in GitHub Discussions
- Tag maintainers in your PR

## Project Structure

```
src/
├── main.rs           # Application entry point
├── config.rs         # Configuration
├── models/           # Data models
├── db/              # Database layer
├── api/             # REST API
├── query/           # Query parsing
├── cache/           # Cache management
├── scryfall/        # Scryfall integration
└── utils/           # Utilities
```

## Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test '*'

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Doc tests
cargo test --doc
```

## Debugging

```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Run with debugger
rust-gdb target/debug/scryfall-cache

# Check for memory leaks
valgrind target/debug/scryfall-cache
```

## Performance Testing

```bash
# Build with optimizations
cargo build --release

# Benchmark
cargo bench

# Profile
cargo flamegraph
```

## Release Process

(For maintainers)

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create release branch
4. Run full test suite
5. Create release tag
6. Build Docker images
7. Publish release notes
8. Deploy to production

## Questions?

- **GitHub Discussions**: General questions
- **GitHub Issues**: Bug reports, feature requests
- **Discord**: Real-time chat
- **Email**: [maintainer email]

## Recognition

Contributors are recognized in:
- CHANGELOG.md
- README.md contributors section
- GitHub contributors page

Thank you for contributing! 🎉
