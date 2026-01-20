# Versioning Policy

Sigil follows [Semantic Versioning 2.0.0](https://semver.org/) for all releases.

## Version Format

All versions follow the format: `MAJOR.MINOR.PATCH` (e.g., `1.2.3`)

- **MAJOR**: Incremented for incompatible API changes or breaking changes to disk format, IPC protocol, or cryptographic operations
- **MINOR**: Incremented for backwards-compatible functionality additions
- **PATCH**: Incremented for backwards-compatible bug fixes

### Pre-release Versions

Pre-release versions may be tagged with identifiers:
- `X.Y.Z-alpha.N`: Early testing releases, unstable
- `X.Y.Z-beta.N`: Feature-complete but not production-ready
- `X.Y.Z-rc.N`: Release candidates, production-ready pending final testing

Examples: `0.2.0-alpha.1`, `1.0.0-beta.2`, `1.0.0-rc.1`

## Current Version

The current version is defined in the root `Cargo.toml` under `[workspace.package]`:

```toml
[workspace.package]
version = "0.1.0"
```

## Release Workflow

Sigil uses **automated semantic versioning** that automatically bumps the version when changes are merged to the `main` branch.

### Automated Version Bumping

When a PR is merged to `main`, the `.github/workflows/auto-version.yml` workflow automatically:

1. **Analyzes commit messages** using [Conventional Commits](https://www.conventionalcommits.org/) format
2. **Determines the version bump type**:
   - **Breaking changes** (MAJOR bump): Commits with `!` suffix or `BREAKING CHANGE:` in body
     - Examples: `feat!: change disk format`, `fix!: update IPC protocol`
   - **New features** (MINOR bump): Commits starting with `feat:`
     - Example: `feat(daemon): add disk expiration warnings`
   - **Bug fixes** (PATCH bump): Commits starting with `fix:`, `perf:`, or `refactor:`
     - Example: `fix(mother): correct presignature generation`, `perf(core): optimize validation`
3. **Updates version** in `Cargo.toml` and `Cargo.lock`
4. **Updates CHANGELOG.md** with the new version entry
5. **Creates and pushes a git tag** (e.g., `v0.2.0`)
6. **Triggers the release workflow** to build and publish artifacts

### Commit Message Format

To ensure proper version bumping, use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>[optional scope][optional !]: <description>

[optional body]

[optional footer(s)]
```

**Types that trigger version bumps:**
- `feat`: New feature (MINOR bump, or MAJOR if `!` suffix)
- `fix`: Bug fix (PATCH bump, or MAJOR if `!` suffix)
- `perf`: Performance improvement (PATCH bump)
- `refactor`: Code refactoring (PATCH bump)
- Any type with `!` suffix: Breaking change (MAJOR bump)

**Other types** (won't trigger automatic bumps by themselves):
- `docs`: Documentation changes
- `style`: Code style changes
- `test`: Test changes
- `chore`: Maintenance tasks
- `ci`: CI/CD changes

**Examples:**

```bash
# PATCH bump (0.1.0 -> 0.1.1)
fix(daemon): correct disk detection timeout
fix: resolve memory leak in presignature cache

# MINOR bump (0.1.0 -> 0.2.0)
feat(mother): add Trezor hardware wallet support
feat: implement disk expiration warnings

# MAJOR bump (0.1.0 -> 1.0.0) - Only when project is >= 1.0.0
feat!: change disk format to v2
fix!: update IPC protocol with incompatible changes

# MAJOR bump converted to MINOR for pre-1.0 (0.1.0 -> 0.2.0)
feat!: breaking change in 0.x.y converts to MINOR bump
```

**Note on Pre-1.0 Versions**: While the project is in `0.x.y` phase, breaking changes (marked with `!`) will bump the MINOR version instead of MAJOR, following semantic versioning guidelines for initial development.

### Manual Version Bumping (Advanced)

For special cases like pre-release versions, you can still use the manual bump script:

```bash
# For pre-release versions
./scripts/bump-version.sh minor alpha  # Creates X.Y.0-alpha.1
./scripts/bump-version.sh patch beta   # Creates X.Y.Z-beta.1
./scripts/bump-version.sh patch rc     # Creates X.Y.Z-rc.1
```

After running the script manually:
```bash
# Commit version bump and changelog
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: bump version to X.Y.Z-alpha.1"

# Create annotated tag
git tag -a vX.Y.Z-alpha.1 -m "Release version X.Y.Z-alpha.1"

# Push changes and tag
git push origin main
git push origin vX.Y.Z-alpha.1
```

### CHANGELOG Management

The CHANGELOG.md is automatically updated by the auto-version workflow. However, for better release notes, you should add meaningful entries to the `[Unreleased]` section as you develop:

```markdown
## [Unreleased]

### Added
- New hardware wallet support for Trezor
- Disk expiration warning system

### Fixed
- Memory leak in presignature cache
- Disk detection timeout issues

### Security
- Updated cryptographic dependencies
```

When the version is bumped, the unreleased changes will become part of that version's history.

### GitHub Release

When a tag matching `v*` is pushed (either automatically or manually), the `.github/workflows/release.yml` workflow automatically:
- Creates a GitHub release
- Builds Linux binaries
- Uploads release artifacts
- Publishes crates to crates.io (for stable releases only)

The release workflow will mark releases as pre-release if the tag contains `alpha`, `beta`, or `rc`.

## Version Compatibility

### Disk Format Compatibility

- **MAJOR version changes**: May introduce incompatible disk formats. Users must reconcile and refill disks with the new format
- **MINOR/PATCH version changes**: Must maintain backwards compatibility with existing disk formats

### IPC Protocol Compatibility

- **MAJOR version changes**: May introduce incompatible IPC protocol changes. Daemon and CLI must be upgraded together
- **MINOR/PATCH version changes**: Must maintain backwards compatibility

### zkVM Proof Compatibility

- **MAJOR version changes**: May change zkVM program logic or proof format
- **MINOR/PATCH version changes**: Should maintain proof format compatibility

## Special Considerations for 0.x.y Versions

While Sigil is in initial development (version `0.x.y`):
- The API is not considered stable
- MINOR version bumps (`0.x.0`) may include breaking changes
- Users should expect potential incompatibilities between minor versions
- Once the project reaches `1.0.0`, strict semantic versioning guarantees will apply

## Version History

See [CHANGELOG.md](CHANGELOG.md) for a detailed history of all releases.

## Questions?

For questions about versioning or releases, please open an issue on GitHub.
