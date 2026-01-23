# Contributing to Sigil

Thank you for considering contributing to Sigil! This document provides guidelines for contributing to the project.

## Code of Conduct

Please be respectful and constructive in all interactions. This project follows standard open source community guidelines.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/sigil.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests and linters
6. Commit your changes
7. Push to your fork
8. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Linux (for full functionality)
- System dependencies: `libudev-dev`

### Building

```bash
# Install system dependencies (Ubuntu/Debian)
sudo apt-get install libudev-dev

# Build the project
cargo build

# Build with optimizations
cargo build --release

# Build with optional features (e.g., Ledger support)
cargo build --features "sigil-mother/ledger"
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p sigil-core

# Run integration tests
cargo test --test '*'

# Run with logging
RUST_LOG=debug cargo test
```

### Code Quality

The CI pipeline includes automated code quality checks with auto-fix capabilities:

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --workspace --all-targets -- -D warnings

# Apply automatic clippy fixes
cargo clippy --fix --workspace --all-targets --allow-dirty --allow-staged

# Check documentation
cargo doc --workspace --no-deps
```

#### CI Auto-Fix Behavior

**For Direct Pushes (main, feature branches):**
- The CI will automatically run `cargo fmt` and `cargo clippy --fix` to fix issues
- Fixed code is automatically committed and pushed back to the branch
- You'll see auto-fix commits from `github-actions[bot]`

**For Pull Requests:**
- The CI detects formatting and clippy issues but doesn't commit directly
- A detailed comment is posted on your PR with:
  - Commands to run locally to fix the issues
  - A diff showing the suggested changes
  - Additional guidance for issues requiring manual intervention
- You should apply the fixes locally and push to your PR branch

#### Applying CI Suggestions Locally

When the CI detects issues in your PR, follow these steps:

1. **For formatting issues:**
   ```bash
   cargo fmt --all
   git add -A
   git commit -m "style: apply formatting fixes"
   git push
   ```

2. **For clippy warnings:**
   ```bash
   # Auto-fix what can be fixed automatically
   cargo clippy --fix --workspace --all-targets --allow-dirty --allow-staged
   
   # Check if any issues remain
   cargo clippy --workspace --all-targets -- -D warnings
   
   # Commit the changes
   git add -A
   git commit -m "fix: apply clippy fixes"
   git push
   ```

3. **For issues requiring manual fixes:**
   - Review the remaining clippy warnings in the CI output
   - Make necessary code changes manually
   - Test your changes with `cargo test`
   - Commit and push

## Pull Request Guidelines

### Before Submitting

1. **Test your changes**: Run the full test suite
2. **Format your code**: Run `cargo fmt --all` (CI will auto-fix on merge, but it's faster to do it locally)
3. **Pass linting**: Run `cargo clippy --fix --workspace --all-targets --allow-dirty --allow-staged` to auto-fix issues, then verify with `cargo clippy --workspace --all-targets -- -D warnings`
4. **Update documentation**: If you're changing public APIs or adding features
5. **Add tests**: For new functionality
6. **Update CHANGELOG.md**: Add your changes to the `[Unreleased]` section (optional but recommended for better release notes)
7. **Use conventional commits**: Ensure your commit messages follow the format described above for proper automatic versioning

**Note on Automated Versioning:**
- Your commit messages control version bumping automatically
- Use `feat:` for new features, `fix:` for bug fixes, `feat!:` or `fix!:` for breaking changes
- Version will be bumped automatically when merged to main
- No need to manually update version numbers in Cargo.toml

**Note on CI Auto-Fix:**
- The CI pipeline will attempt to automatically fix formatting and clippy issues
- For PRs, it will comment with suggested fixes instead of committing directly
- For direct pushes to branches, fixes are automatically committed
- It's still recommended to run these checks locally before pushing to minimize CI iterations

### PR Description

Please include:
- **Summary**: Brief description of the changes
- **Motivation**: Why is this change needed?
- **Changes**: Detailed list of modifications
- **Testing**: How you tested the changes
- **Related Issues**: Link to any related issues

### Example PR Description

```markdown
## Summary
Add support for Trezor hardware wallets in mother device

## Motivation
Users requested alternative hardware wallet support beyond Ledger

## Changes
- Added Trezor integration in sigil-mother
- Updated documentation with Trezor setup instructions
- Added Trezor feature flag

## Testing
- Tested with Trezor Model T
- All existing tests pass
- Added unit tests for Trezor integration

## Related Issues
Closes #123
```

## Versioning and Releases

Sigil follows [Semantic Versioning 2.0.0](https://semver.org/) with **automated version bumping**. See [VERSIONING.md](VERSIONING.md) for detailed information.

### Automated Version Bumping

Version bumps happen automatically when PRs are merged to `main`, based on your commit messages:

- **Breaking changes** (`type!:` or `BREAKING CHANGE:`) → MAJOR bump (or MINOR for 0.x.y)
- **New features** (`feat:`) → MINOR bump
- **Bug fixes** (`fix:`, `perf:`, `refactor:`) → PATCH bump

### Understanding Version Bumps

When contributing, write your commit messages to reflect how your changes should affect versioning:

#### Breaking Changes (MAJOR version bump, or MINOR for 0.x.y versions)
Use `type!:` syntax or add `BREAKING CHANGE:` in the commit body for:
- Changes to disk format that break compatibility
- IPC protocol changes requiring daemon updates
- Removal of public APIs
- Changes to CLI command structure
- Cryptographic algorithm changes

Examples:
```bash
feat!: change disk format to v2
fix!: update IPC protocol structure
```

#### New Features (MINOR version bump)
Use `feat:` for:
- New CLI commands or options
- New daemon functionality
- Additional cryptographic ciphersuites
- New hardware wallet integrations

Examples:
```bash
feat(mother): add Trezor support
feat: implement disk expiration warnings
```

#### Bug Fixes (PATCH version bump)
Use `fix:`, `perf:`, or `refactor:` for:
- Security fixes
- Bug fixes without API changes
- Performance improvements

Examples:
```bash
fix(daemon): correct disk detection timeout
perf(core): optimize signature validation
```

#### No Version Bump
Use `docs:`, `test:`, `style:`, `chore:`, or `ci:` for changes that don't affect the released code.

### Changelog Entries

Add your changes to `CHANGELOG.md` under the `[Unreleased]` section:

```markdown
## [Unreleased]

### Added
- Your new feature description

### Changed
- Your changes to existing functionality

### Fixed
- Your bug fixes
```

Categories:
- **Added**: New features
- **Changed**: Changes to existing functionality
- **Deprecated**: Features marked for removal
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security fixes

## Commit Message Guidelines

**Important**: Sigil uses **automated semantic versioning** based on commit messages. Your commit messages directly control version bumps!

We follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>[optional scope][optional !]: <description>

[optional body]

[optional footer]
```

### Types

**Types that trigger automatic version bumps:**
- `feat`: New feature (triggers MINOR version bump)
- `fix`: Bug fix (triggers PATCH version bump)
- `perf`: Performance improvement (triggers PATCH version bump)
- `refactor`: Code refactoring (triggers PATCH version bump)
- Any type with `!` suffix: Breaking change (triggers MAJOR version bump)

**Other types** (documentation, no version bump):
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `test`: Adding or updating tests
- `chore`: Maintenance tasks (like dependency updates)
- `ci`: CI/CD changes

### Breaking Changes

To indicate a breaking change, add `!` after the type/scope:

```
feat!: change disk format to v2
fix!: update IPC protocol with incompatible changes
feat(daemon)!: remove deprecated API
```

Or add `BREAKING CHANGE:` in the commit footer (after a blank line):

```
feat: new authentication system

BREAKING CHANGE: The old authentication method is no longer supported.
Users must migrate to the new system.
```

**Note**: The `BREAKING CHANGE:` footer must appear after a blank line following the commit body, according to the [Conventional Commits specification](https://www.conventionalcommits.org/).

### Examples

```bash
# PATCH bump (0.1.0 -> 0.1.1)
fix(daemon): correct disk detection timeout
perf(core): optimize presignature validation
refactor(cli): simplify error handling

# MINOR bump (0.1.0 -> 0.2.0)
feat(mother): add Trezor hardware wallet support
feat: implement disk expiration warnings

# MAJOR bump (for projects >= 1.0.0, MINOR for 0.x.y)
feat!: change disk format to v2
fix!: update IPC protocol with incompatible changes

# No version bump
docs: update README with Ledger setup instructions
test(core): add tests for disk format validation
ci: update GitHub Actions workflow
style: apply rustfmt formatting
```

### Version Bump on Merge

When your PR is merged to `main`:
1. The auto-version workflow analyzes all commit messages
2. Determines the highest version bump needed (major > minor > patch)
3. Automatically updates version files, CHANGELOG, and creates a release tag
4. Triggers the release workflow to build and publish

**Pro tip**: Squash your commits when merging PRs to ensure clean, semantic commit messages that properly control versioning.

## Code Style

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting (run `cargo fmt`)
- Use `clippy` for linting (run `cargo clippy`)
- Write idiomatic Rust code
- Add documentation comments for public APIs

### Documentation

- Document all public APIs with `///` doc comments
- Include examples in documentation when appropriate
- Update README.md for significant changes
- Keep docs/ directory synchronized with code changes

## Security

### Reporting Security Issues

**Do not open public issues for security vulnerabilities.**

Please report security issues to the maintainers privately. See [SECURITY.md](SECURITY.md) for details.

### Security Considerations

When contributing, keep these security principles in mind:

- **Timing-safe operations**: Use constant-time comparisons for sensitive data
- **Memory zeroization**: Clear sensitive data from memory after use
- **Input validation**: Validate all inputs, especially from disks and IPC
- **Error handling**: Don't leak sensitive information in error messages
- **Cryptographic best practices**: Follow established cryptographic standards

## Areas for Contribution

Here are some areas where contributions are particularly welcome:

### High Priority

- Additional hardware wallet integrations (Trezor, etc.)
- Performance optimizations
- Security audits and improvements
- Documentation improvements
- Test coverage expansion

### Features

- Support for additional blockchains
- Enhanced reconciliation reporting
- Improved CLI user experience
- Additional zkVM proof types
- Multi-language support

### Infrastructure

- CI/CD improvements
- Release automation
- Packaging (Debian, RPM, etc.)
- Docker support
- Cross-platform support

## Questions?

If you have questions about contributing, please:

1. Check existing documentation
2. Search closed issues and PRs
3. Open a new issue with the "question" label

Thank you for contributing to Sigil!
