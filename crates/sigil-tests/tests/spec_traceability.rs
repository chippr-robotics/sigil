//! Requirement traceability.
//!
//! A spec whose requirements point at nothing is prose with numbers on it.
//! These tests check that every `FR-xxx` in `specs/` is accounted for: named
//! in its spec's Coverage table, and mapped to a test that actually exists in
//! the workspace — or explicitly marked as not implemented, which is a
//! statement someone chose to make rather than an omission nobody noticed.
//!
//! See `specs/README.md`. This lives in `sigil-tests` so it runs in the
//! existing `Unit Tests` job; no new CI job is needed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/sigil-tests is two levels below the repo root")
        .to_path_buf()
}

/// Every `specs/<NNN>-<slug>/spec.md`.
fn spec_files() -> Vec<PathBuf> {
    let specs = repo_root().join("specs");
    let mut out = Vec::new();

    for entry in std::fs::read_dir(&specs).expect("specs/ must exist") {
        let dir = entry.expect("readable entry").path();
        if !dir.is_dir() {
            continue;
        }
        let spec = dir.join("spec.md");
        if spec.exists() {
            out.push(spec);
        }
    }

    out.sort();
    assert!(!out.is_empty(), "specs/ must contain at least one spec");
    out
}

/// Requirement identifiers appearing in a document, e.g. `FR-014`.
fn requirement_ids(text: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let bytes = text.as_bytes();

    for (index, _) in text.match_indices("FR-") {
        let digits: String = bytes[index + 3..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .map(|b| *b as char)
            .collect();
        if digits.len() >= 3 {
            ids.insert(format!("FR-{digits}"));
        }
    }

    ids
}

/// The Coverage section of a spec, where requirements are mapped to tests.
fn coverage_section(text: &str) -> &str {
    match text.find("\n## Coverage") {
        Some(start) => &text[start..],
        None => "",
    }
}

/// Every Rust source file in the workspace, so a named test can be located.
fn workspace_sources() -> String {
    fn walk(dir: &Path, out: &mut String) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if matches!(name.as_ref(), "target" | ".git") {
                    continue;
                }
                walk(&path, out);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    out.push_str(&text);
                    out.push('\n');
                }
            }
        }
    }

    let mut out = String::new();
    walk(&repo_root().join("crates"), &mut out);
    out
}

/// Every requirement must appear in its spec's Coverage table.
///
/// Catches the common failure: a requirement added during review that nobody
/// ever wired to a test, and which then reads as satisfied because it is
/// written down.
#[test]
fn every_requirement_appears_in_its_specs_coverage_table() {
    let mut missing = Vec::new();

    for spec in spec_files() {
        let text = std::fs::read_to_string(&spec).expect("readable spec");
        let coverage = coverage_section(&text);

        assert!(
            !coverage.is_empty(),
            "{} has no `## Coverage` section. Every spec must say which test \
             covers each requirement, or say explicitly that none does.",
            spec.display()
        );

        let in_coverage = requirement_ids(coverage);
        for id in requirement_ids(&text) {
            if !in_coverage.contains(&id) {
                missing.push(format!("{}: {id}", spec.display()));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "requirements with no row in their spec's Coverage table:\n  {}\n\n\
         Add a row naming the test that covers each, or a row saying it is not \
         implemented. A requirement that points at nothing reads as satisfied \
         and is not.",
        missing.join("\n  ")
    );
}

/// Every test named in a Coverage table must exist.
///
/// A coverage table is only worth having if its entries are real; a renamed or
/// deleted test would otherwise leave the spec claiming coverage it lost.
#[test]
fn every_test_named_in_a_coverage_table_exists() {
    let sources = workspace_sources();
    let mut missing = Vec::new();

    for spec in spec_files() {
        let text = std::fs::read_to_string(&spec).expect("readable spec");

        for line in coverage_section(&text).lines() {
            if !line.trim_start().starts_with('|') {
                continue;
            }

            // Test names are the backticked snake_case identifiers in the row.
            for candidate in line.split('`').skip(1).step_by(2) {
                let candidate = candidate.trim();
                let looks_like_a_test = candidate.len() > 8
                    && candidate
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                    && candidate.contains('_');

                if looks_like_a_test && !sources.contains(&format!("fn {candidate}")) {
                    missing.push(format!("{}: {candidate}", spec.display()));
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Coverage tables name tests that do not exist in the workspace:\n  {}\n\n\
         The test was renamed or removed and its spec still claims it. Update \
         the spec, or restore the test.",
        missing.join("\n  ")
    );
}

/// No spec outside Draft status may carry unresolved clarifications.
///
/// `NEEDS CLARIFICATION` is spec-kit's marker for a decision nobody made.
/// Carrying one into an approved spec means shipping against an unknown.
#[test]
fn no_approved_spec_has_unresolved_clarifications() {
    let mut offenders = Vec::new();

    for spec in spec_files() {
        let text = std::fs::read_to_string(&spec).expect("readable spec");
        let is_draft = text.contains("**Status**: Draft");

        if !is_draft && text.contains("NEEDS CLARIFICATION") {
            offenders.push(spec.display().to_string());
        }
    }

    assert!(
        offenders.is_empty(),
        "specs past Draft status still contain NEEDS CLARIFICATION:\n  {}\n\n\
         Resolve the question, or move the spec back to Draft. An approved spec \
         with an open question is an unknown someone will implement by guessing.",
        offenders.join("\n  ")
    );
}

/// Every spec must be listed in the backlog.
///
/// `specs/README.md` is what a reviewer checks against; a spec absent from it
/// is invisible to the process that is supposed to track it.
#[test]
fn every_spec_is_listed_in_the_backlog() {
    let backlog = std::fs::read_to_string(repo_root().join("specs/README.md"))
        .expect("specs/README.md must exist");

    let mut missing = Vec::new();
    for spec in spec_files() {
        let dir = spec
            .parent()
            .and_then(Path::file_name)
            .expect("spec directory")
            .to_string_lossy()
            .to_string();

        if !backlog.contains(&dir) {
            missing.push(dir);
        }
    }

    assert!(
        missing.is_empty(),
        "specs absent from specs/README.md: {missing:?}.\n\n\
         The backlog is what reviewers check against; add a row for each."
    );
}
