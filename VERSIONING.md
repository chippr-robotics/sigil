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

The current version is defined once, in the root `Cargo.toml` under
`[workspace.package]`, and inherited by every crate:

```toml
[workspace.package]
version = "0.6.0"
```

The latest `v*` git tag is the version that was last *released*. When the two
differ, a bump has been prepared and its tag has not been pushed yet.


## Release Workflow

A release takes three deliberate steps, and no automation performs any of them
on its own. Nothing reaches `main` without being reviewed on `staging` first,
and nothing is published without a person pushing a tag.

```
  merge to staging
        │
        ▼
  release-prep.yml  ──▶  PR: "chore: release vX.Y.Z"  ──▶  merge to staging
   (proposes only)
        │
        ▼
  staging ──▶ main                                      (you merge)
        │
        ▼
  git push origin vX.Y.Z                                (you push the tag)
        │
        ▼
  release.yml  ──▶  verify, build, GitHub release
```

### Step 1 — the bump is proposed

On a push to `staging`, `.github/workflows/release-prep.yml`:

1. **Checks whether a release is already pending.** It proposes nothing unless
   the workspace version equals the latest `v*` tag. If they differ, a bump is
   already prepared and awaiting its tag.
2. **Analyzes commit messages** since that tag, using
   [Conventional Commits](https://www.conventionalcommits.org/) format.
3. **Determines the bump type**:
   - **Breaking changes** (MAJOR): commits with `!` suffix or a
     `BREAKING CHANGE:` footer — e.g. `feat!: change disk format`
   - **New features** (MINOR): commits starting with `feat:`
   - **Bug fixes** (PATCH): commits starting with `fix:`, `perf:`, `refactor:`
   - Anything else proposes nothing.
4. **Opens a pull request** against `staging` with the `Cargo.toml`,
   `Cargo.lock` and `CHANGELOG.md` changes on a `release/vX.Y.Z` branch.

It does not commit to `staging` or to `main`. CI holding write access to an
integration branch is forbidden by the constitution
(`.specify/memory/constitution.md`, Security Requirements) and asserted by
`crates/sigil-tests/tests/constitution_conformance.rs`.

> **The bump PR arrives with no CI.** GitHub does not trigger workflows from
> events created by a workflow's own `GITHUB_TOKEN`, so `ci.yml` does not run
> on a branch that workflow pushed. `release-prep.yml` runs
> `cargo check --workspace --locked --all-targets` itself and reports the
> result in the PR body. To run the full suite, use **Run workflow** on `CI`
> and select the release branch.

### Step 2 — staging reaches main

The version bump merges to `staging` like any other change and reaches `main`
through the normal `staging` → `main` pull request.

### Step 3 — you push the tag

```bash
git checkout main && git pull
git tag -a vX.Y.Z -m "Release version X.Y.Z"
git push origin vX.Y.Z
```

**The tag must be pushed with a human credential.** A tag pushed by a
workflow's `GITHUB_TOKEN` does not start `release.yml` — which is why `v0.2.0`
through `v0.5.0` exist in this repository and produced no releases at all. The
workflow that pushed them reported success six times.


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

# MAJOR bump with BREAKING CHANGE footer
feat: new authentication system

BREAKING CHANGE: The old authentication method is no longer supported.

# MAJOR bump converted to MINOR for pre-1.0 (0.1.0 -> 0.2.0)
feat!: breaking change in 0.x.y converts to MINOR bump
```

**Note on Pre-1.0 Versions**: While the project is in `0.x.y` phase, breaking changes (marked with `!`) will bump the MINOR version instead of MAJOR, following semantic versioning guidelines for initial development.

### Manual Version Bumping (Advanced)

For pre-release versions, or when you would rather not wait for the proposal,
run the bump script yourself:

```bash
./scripts/bump-version.sh minor          # X.Y.0
./scripts/bump-version.sh minor alpha    # X.Y.0-alpha.1
./scripts/bump-version.sh patch rc       # X.Y.Z-rc.1
```

Then open it as a pull request against `staging`, exactly as the workflow
would:

```bash
git checkout -b release/vX.Y.Z
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: bump version to X.Y.Z"
git push -u origin release/vX.Y.Z
```

Tag after it has merged and `staging` has reached `main`, per step 3 above.


### CHANGELOG Management

`release-prep.yml` adds the version heading and the compare links, and
nothing else — it writes "See commit history for changes in this release."
under it. That is a placeholder, not release notes. Add meaningful entries
to the `[Unreleased]` section as you develop, and they become that
version's history when the bump lands:

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

A pushed `v*` tag starts `.github/workflows/release.yml`, which:

1. **Validates the tag** — semver shape, and the version must match
   `Cargo.toml` at that tag.
2. **Verifies the tagged commit** — `cargo fmt --check`, `cargo clippy -D
   warnings` and the full test suite. A tag can point at any commit, so CI
   being green on `main` is not evidence about this one.
3. **Builds the Linux x86_64 binaries** — `sigil`, `sigil-daemon`,
   `sigil-mother`, `sigil-mother-tui`, `sigil-mcp` — and fails if any is
   missing.
4. **Publishes the GitHub release** with the tarball and a `.sha256` checksum
   beside it.

The release is marked pre-release if the tag contains `alpha`, `beta` or `rc`.

The same workflow can be re-run against an existing tag with **Run workflow**
on `Release`, which is the recovery path for a tag whose push did not trigger
it.

**Crates are not published to crates.io.** The job that claimed to do so could
not have worked — internal dependencies carry no version requirement, so
`cargo package` refuses them; `sigil-frost` was missing from the publish order;
and the name `sigil-cli` belongs to an unrelated crate. Every step carried
`continue-on-error: true`, so it reported success anyway. Publishing is on hold
until coverage and end-to-end assurance justify putting key-custody crates into
a public namespace, where a version cannot be withdrawn. Tracked in
`specs/README.md`. The supported install does not need crates.io:

```bash
cargo install --locked --git https://github.com/chippr-robotics/sigil --tag vX.Y.Z sigil-cli
```


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
