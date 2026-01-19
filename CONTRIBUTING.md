# Contributing to Sigil

Thank you for your interest in contributing to Sigil! This document provides guidelines for contributing to the project.

## Development Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/chippr-robotics/sigil.git
   cd sigil
   ```

2. **Build the project**:
   ```bash
   cargo build --release
   ```

3. **Run tests**:
   ```bash
   cargo test
   ```

## Development Workflow

### Working with Development Artifacts

When developing features that involve child disk images or agent shards, follow these guidelines:

1. **Create artifacts in the proper location**:
   ```bash
   # For testing - place in artifacts directory
   sigil-mother create-child \
     --presig-count 100 \
     --output artifacts/child_disks/my_test_disk.img \
     --agent-output artifacts/agent_shards/my_test_disk_agent_shares.json
   ```

2. **Temporary artifacts** (not for sharing):
   ```bash
   # Create in /tmp or other temporary location
   sigil-mother create-child \
     --presig-count 100 \
     --output /tmp/temp_disk.img \
     --agent-output /tmp/temp_agent_shares.json
   ```

3. **Naming conventions**: Follow the guidelines in [`artifacts/README.md`](artifacts/README.md):
   - Use descriptive names: `test_<purpose>_<description>.img`
   - Match agent shard names to disk names
   - Example: `test_basic_signing.img` → `test_basic_signing_agent_shares.json`

4. **When to commit artifacts**:
   - ✅ **DO commit** if:
     - Artifact is part of test suite
     - Needed for documentation/examples
     - Required for team collaboration
     - Demonstrates specific edge case
   
   - ❌ **DON'T commit** if:
     - Temporary test output
     - Personal development artifact
     - Can be easily regenerated
     - Large (> 5MB)

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Run clippy before committing: `cargo clippy`
- Write tests for new features
- Update documentation for API changes

### Git Workflow

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes**:
   - Write code
   - Add tests
   - Update documentation
   - Add artifacts if needed (in `artifacts/` directory)

3. **Commit your changes**:
   ```bash
   git add .
   git commit -m "Description of changes"
   ```

4. **Push and create PR**:
   ```bash
   git push origin feature/my-feature
   ```

### Artifact Organization

The repository uses this structure for development artifacts:

```
artifacts/
├── child_disks/        # Child disk images (.img)
├── agent_shards/       # Agent shard data (.json)
└── examples/           # Reference artifacts for docs
```

**Key points**:
- Artifacts in `artifacts/` are tracked by git
- Artifacts elsewhere are automatically ignored
- See [`artifacts/README.md`](artifacts/README.md) for detailed guidelines

### Testing

#### Unit Tests
```bash
cargo test
```

#### Integration Tests
```bash
cargo test --test '*'
```

#### Testing with Artifacts
When writing tests that use artifacts:

```rust
#[test]
fn test_disk_loading() {
    let disk_path = "artifacts/child_disks/test_basic_100_presigs.img";
    let disk = DiskFormat::from_file(disk_path).unwrap();
    // ... assertions
}
```

Ensure the artifact exists or create it in the test setup.

## Pull Request Guidelines

1. **Title**: Clear, concise description of changes
2. **Description**: 
   - What changed and why
   - Related issues
   - Testing performed
   - If artifacts added, explain their purpose
3. **Tests**: Include tests for new features
4. **Documentation**: Update relevant documentation
5. **Size**: Keep PRs focused and reasonably sized

## Security Considerations

⚠️ **Important**: 
- Never commit real key material or production keys
- All artifacts must use test/dummy keys only
- Mark test artifacts clearly in names and documentation
- Review the [SECURITY.md](SECURITY.md) before working with cryptographic code

## Questions?

- Open an issue for bugs or feature requests
- Check existing documentation in `docs/`
- Review `README.md` for architecture overview

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 License.
