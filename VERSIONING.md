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

### 1. Determine Version Bump

Analyze changes since last release to determine the appropriate version bump:

- **Breaking changes** (require MAJOR bump):
  - Disk format changes that break compatibility
  - IPC protocol changes requiring daemon updates
  - Cryptographic algorithm changes
  - Removal of public APIs
  - Changes to CLI command structure

- **New features** (require MINOR bump):
  - New CLI commands or options
  - New daemon functionality
  - New mother device features
  - Additional cryptographic ciphersuites
  - New hardware wallet integrations

- **Bug fixes** (require PATCH bump):
  - Security fixes
  - Bug fixes that don't change APIs
  - Performance improvements
  - Documentation updates

### 2. Update Version

Use the provided version bump script:

```bash
# For patch release (bug fixes)
./scripts/bump-version.sh patch

# For minor release (new features)
./scripts/bump-version.sh minor

# For major release (breaking changes)
./scripts/bump-version.sh major

# For pre-release versions
./scripts/bump-version.sh minor alpha  # Creates X.Y.0-alpha.1
./scripts/bump-version.sh patch beta   # Creates X.Y.Z-beta.1
./scripts/bump-version.sh patch rc     # Creates X.Y.Z-rc.1
```

This script will:
- Update version in `Cargo.toml`
- Update version in `Cargo.lock`
- Prompt you to update `CHANGELOG.md`

### 3. Update CHANGELOG

Before creating a release, update `CHANGELOG.md` with all changes since the last release:

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Changes to existing functionality

### Deprecated
- Features marked for removal

### Removed
- Removed features

### Fixed
- Bug fixes

### Security
- Security fixes
```

### 4. Commit and Tag

```bash
# Commit version bump and changelog
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: bump version to X.Y.Z"

# Create annotated tag
git tag -a vX.Y.Z -m "Release version X.Y.Z"

# Push changes and tag
git push origin main
git push origin vX.Y.Z
```

### 5. GitHub Release

When a tag matching `v*` is pushed, the `.github/workflows/release.yml` workflow automatically:
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
