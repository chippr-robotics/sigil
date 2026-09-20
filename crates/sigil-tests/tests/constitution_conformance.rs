//! Constitution conformance.
//!
//! `.specify/memory/constitution.md` states seven principles. A constitution
//! that cannot fail a build is the same category of object as a `Security
//! Audit` job with `continue-on-error: true` — which is what issue #53 found
//! and #57 fixed. These tests make the principles executable.
//!
//! Each test names the principle it enforces and explains, in its failure
//! message, what went wrong and why it matters. A future contributor who trips
//! one should not have to read the constitution to understand what they broke.
//!
//! Coverage as of this file (see `specs/README.md` for the live table):
//!
//! | Principle | Here? |
//! | --- | --- |
//! | I. One TCB | ✅ |
//! | II. No signature without physical consent | ⬜ requires signer tests — backlog item 1 |
//! | III. Default deny at the network edge | ✅ (no HTTP stack at all) |
//! | IV. Key material off convenience transports | ✅ (partial: no HTTP stack to carry it) |
//! | V. Supported install is auditable | ✅ |
//! | VI. No outbound path from the air-gapped side | ✅ |
//! | VII. Least privilege on the local machine | ✅ elsewhere (`sigil-daemon` ipc tests) |

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Repository root, derived from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/sigil-tests should be two levels below the repo root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Every file in the repository with one of the given extensions, skipping
/// build output and VCS metadata.
fn files_with_extensions(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    fn walk(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if matches!(
                    name.as_ref(),
                    "target" | ".git" | "node_modules" | ".dart_tool"
                ) {
                    continue;
                }
                walk(&path, extensions, out);
            } else if path
                .extension()
                .map(|e| extensions.contains(&e.to_string_lossy().as_ref()))
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }

    let mut out = Vec::new();
    walk(root, extensions, &mut out);
    out.sort();
    out
}

// ============================================================================
// Principle I — One TCB, and It Contains a Floppy Disk
// ============================================================================

/// The workspace's `default-members` must equal `members`.
///
/// Since the removal of `sigil-bridge` every crate here is in the TCB, so a
/// bare `cargo build` produces no binary that is outside it. If an out-of-TCB
/// crate is ever added it must be absent from `default-members` — and the
/// companion test below then requires it to be `publish = false` as well.
#[test]
fn p1_default_members_equals_members() {
    let root = repo_root();
    let manifest: toml::Value = read(&root.join("Cargo.toml"))
        .parse()
        .expect("root Cargo.toml must be valid TOML");

    let workspace = manifest
        .get("workspace")
        .expect("root manifest must have a [workspace] table");

    let list = |key: &str| -> Vec<String> {
        workspace
            .get(key)
            .unwrap_or_else(|| panic!("[workspace] must declare `{key}`"))
            .as_array()
            .unwrap_or_else(|| panic!("`{key}` must be an array"))
            .iter()
            .map(|v| v.as_str().expect("entry must be a string").to_string())
            .collect()
    };

    let mut members = list("members");
    let mut default_members = list("default-members");
    members.sort();
    default_members.sort();

    let out_of_tcb: Vec<_> = members
        .iter()
        .filter(|m| !default_members.contains(m))
        .cloned()
        .collect();

    assert!(
        out_of_tcb.is_empty(),
        "Constitution Principle I: these crates are workspace members but not in \
         `default-members`, which marks them out of TCB: {out_of_tcb:?}.\n\n\
         That may be correct — an out-of-TCB convenience surface belongs outside \
         the default build. If so, update this test and `specs/README.md`, and \
         make sure each such crate is `publish = false` (see \
         p1_out_of_tcb_crates_are_unpublished).\n\n\
         If it was not deliberate, add them back to `default-members`."
    );

    assert_eq!(
        members, default_members,
        "Constitution Principle I: `default-members` must list every member while \
         the whole workspace is in the TCB."
    );
}

/// Any crate deliberately excluded from `default-members` must also be
/// unpublishable, so it cannot be installed as though it were supported.
#[test]
fn p1_out_of_tcb_crates_are_unpublished() {
    let root = repo_root();
    let manifest: toml::Value = read(&root.join("Cargo.toml"))
        .parse()
        .expect("root Cargo.toml must be valid TOML");
    let workspace = manifest.get("workspace").expect("[workspace] table");

    let read_list = |key: &str| -> Vec<String> {
        workspace
            .get(key)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };

    let members = read_list("members");
    let default_members = read_list("default-members");

    for member in members.iter().filter(|m| !default_members.contains(m)) {
        let manifest_path = root.join(member).join("Cargo.toml");
        let crate_manifest: toml::Value = read(&manifest_path)
            .parse()
            .unwrap_or_else(|e| panic!("{} must be valid TOML: {e}", manifest_path.display()));

        let publishable = crate_manifest
            .get("package")
            .and_then(|p| p.get("publish"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(true);

        assert!(
            !publishable,
            "Constitution Principle I: `{member}` is out of TCB (absent from \
             `default-members`) but does not declare `publish = false`. An \
             out-of-TCB component must not be installable as though it were part \
             of the supported signing system."
        );
    }
}

// ============================================================================
// Principle III / IV — nothing terminates HTTP; key material stays off it
// ============================================================================

/// No crate may depend on an HTTP server framework.
///
/// `SECURITY.md` standing invariant 3 says nothing in this repository
/// terminates HTTP or listens on a network socket. That is currently true by
/// construction — `sigil-bridge` was deleted — and this keeps it true. There is
/// no network-facing signing endpoint to authenticate because there is no
/// network-facing endpoint.
///
/// It also does most of Principle IV's work: key material cannot travel over a
/// convenience transport that does not exist.
#[test]
fn p3_no_crate_depends_on_an_http_server() {
    const HTTP_SERVERS: &[&str] = &[
        "axum",
        "warp",
        "actix-web",
        "rocket",
        "tide",
        "poem",
        "salvo",
        "hyper",
        "tiny_http",
        "tower-http",
    ];

    let root = repo_root();
    let mut offenders = Vec::new();

    for entry in std::fs::read_dir(root.join("crates")).expect("crates/ must exist") {
        let dir = entry.expect("readable dir entry").path();
        let manifest_path = dir.join("Cargo.toml");
        if !manifest_path.exists() {
            continue;
        }

        let manifest: toml::Value = read(&manifest_path)
            .parse()
            .unwrap_or_else(|e| panic!("{} must be valid TOML: {e}", manifest_path.display()));

        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(table) = manifest.get(section).and_then(toml::Value::as_table) else {
                continue;
            };
            for name in table.keys() {
                if HTTP_SERVERS.contains(&name.as_str()) {
                    offenders.push(format!(
                        "{}: {section}.{name}",
                        dir.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "Constitution Principles III and IV: an HTTP server dependency appeared \
         in the workspace: {offenders:?}.\n\n\
         Nothing in this repository terminates HTTP. Sigil's boundary is the \
         physically inserted disk, and an HTTP surface in a key-custody repo is \
         how the issue #53 finding came to exist — an unauthenticated \
         `POST /api/sign` on 0.0.0.0 with wildcard CORS, serving a client that \
         had never been built.\n\n\
         If a remote interface is genuinely wanted, it belongs out of TCB in a \
         separate repository, and this is a constitutional amendment."
    );
}

// ============================================================================
// Principle V — The Supported Install Is Auditable
// ============================================================================

/// `scripts/install.sh` must refuse to run when piped from a shell.
///
/// Reproduces `curl … | sudo bash` locally: feed the script to `bash` on stdin
/// and require a non-zero exit. An operator of a key-custody product should be
/// able to read what is about to run as root before it runs.
#[test]
fn p5_install_script_refuses_pipe_execution() {
    let script = repo_root().join("scripts/install.sh");
    assert!(script.exists(), "scripts/install.sh must exist");

    let file = std::fs::File::open(&script).expect("install.sh must be readable");

    let output = Command::new("bash")
        .stdin(Stdio::from(file))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("bash must be available to run this test");

    assert!(
        !output.status.success(),
        "Constitution Principle V: `scripts/install.sh` ran to completion when \
         piped into bash. It must refuse, because a piped script is one the \
         operator has not read.\n\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Refusing to run from a pipe"),
        "Constitution Principle V: install.sh exited non-zero when piped, but \
         without the expected refusal message. It may be failing for an \
         unrelated reason, which would make this test pass by accident.\n\n\
         stderr:\n{stderr}"
    );
}

/// `install.sh` must still work when executed as a real file, so the refusal
/// above is a targeted guard and not a script that is simply broken.
#[test]
fn p5_install_script_guard_admits_real_file_execution() {
    let root = repo_root();
    let script = read(&root.join("scripts/install.sh"));

    let guard_start = script
        .find("refuse_pipe_execution() {")
        .expect("install.sh must define refuse_pipe_execution()");
    let guard_end = script[guard_start..]
        .find("\n}")
        .map(|i| guard_start + i + 2)
        .expect("refuse_pipe_execution() must be a closed function");

    let harness = format!(
        "{}\nrefuse_pipe_execution\necho GUARD_ADMITTED_FILE\n",
        &script[guard_start..guard_end]
    );

    let dir = std::env::temp_dir().join(format!("sigil-p5-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let harness_path = dir.join("guard.sh");
    std::fs::write(&harness_path, harness).expect("write harness");

    let output = Command::new("bash")
        .arg(&harness_path)
        .output()
        .expect("bash must be available");

    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("GUARD_ADMITTED_FILE"),
        "Constitution Principle V: the pipe guard rejected a script executed from \
         a real file on disk. The guard must block pipes only; blocking normal \
         execution makes the supported install impossible.\n\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// No document may present a pipe-to-shell command as an instruction.
///
/// Matches lines that *are* such a command, so prose describing the policy —
/// "there is no `curl | sudo bash` one-liner" — does not trip it.
#[test]
fn p5_no_document_instructs_piping_into_a_shell() {
    let root = repo_root();
    let mut offenders = Vec::new();

    for path in files_with_extensions(&root, &["md", "html", "txt", "rst"]) {
        for (number, line) in read(&path).lines().enumerate() {
            let trimmed = line
                .trim()
                .trim_start_matches("<code>")
                .trim_start_matches('$');
            let trimmed = trimmed.trim();
            if !trimmed.starts_with("curl") && !trimmed.starts_with("wget") {
                continue;
            }
            let pipes_to_shell = trimmed.contains("| bash")
                || trimmed.contains("| sh")
                || trimmed.contains("| sudo bash")
                || trimmed.contains("| sudo sh");
            if pipes_to_shell {
                offenders.push(format!(
                    "{}:{}: {trimmed}",
                    path.strip_prefix(&root).unwrap_or(&path).display(),
                    number + 1
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "Constitution Principle V: documentation instructs an operator to pipe \
         network content into a shell:\n  {}\n\n\
         The supported install is `cargo install --locked` from a pinned tag, or \
         a cloned checkout. Asking operators of a key custody product to execute \
         unread, unpinned code as root is not a supported install path.",
        offenders.join("\n  ")
    );
}

// ============================================================================
// Principle VI — No Outbound Path From the Air-Gapped Side
// ============================================================================

/// Knowledge-base and sync tooling must not reappear in this repository.
///
/// A Logseq skill lived here until issue #53. Its `sigil-mother-node` example
/// instructed operators to merge air-gapped mother device material into a
/// networked, indexed knowledge graph — an exfiltration path from the side of
/// the air gap that must not have one, documented as a feature.
///
/// Denylisted by skill directory name, which is how it would come back.
#[test]
fn p6_no_knowledge_base_sync_tooling() {
    const DENYLIST: &[&str] = &[
        "logseq", "obsidian", "notion", "roam", "anytype", "dendron", "foam", "mem", "tana",
    ];

    let skills = repo_root().join(".claude/skills");
    if !skills.exists() {
        return;
    }

    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(&skills).expect(".claude/skills must be readable") {
        let entry = entry.expect("readable dir entry");
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if DENYLIST.iter().any(|d| name.contains(d)) {
            offenders.push(name.to_string());
        }
    }

    assert!(
        offenders.is_empty(),
        "Constitution Principle VI: knowledge-base or sync tooling reappeared in \
         `.claude/skills/`: {offenders:?}.\n\n\
         Tooling that indexes, syncs, merges or mirrors mother-device material \
         into a networked system does not live in this repository. Documentation \
         showing an operator how to do so is an exfiltration tutorial regardless \
         of intent. If you want it, it belongs in a separate repository that \
         holds no key material."
    );
}

// ============================================================================
// Security Requirements — CI is a gate, not an author
// ============================================================================

/// Every GitHub Actions workflow definition.
fn workflow_files(root: &Path) -> Vec<PathBuf> {
    let dir = root.join(".github/workflows");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        panic!(
            "{} does not exist. CI is part of the TCB story; if the workflows \
             moved, these tests have to move with them.",
            dir.display()
        );
    };

    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
        })
        .collect();
    files.sort();

    assert!(
        !files.is_empty(),
        "No workflow files found under {}.",
        dir.display()
    );
    files
}

/// No workflow may push to `main` or `staging`.
///
/// `auto-version.yml` did exactly this until this change: it bumped the
/// version, committed, and pushed to `main` under `secrets.GITHUB_TOKEN`. Six
/// runs, all green, all of them putting a commit on the default branch that no
/// person had read.
///
/// It also silently did nothing useful. GitHub does not trigger workflows from
/// events created by a workflow's own `GITHUB_TOKEN`, so the tags it pushed
/// never started `release.yml` — four tags, zero releases.
///
/// Automation that wants to change this repository opens a pull request.
#[test]
fn ci_never_pushes_to_an_integration_branch() {
    const PROTECTED: &[&str] = &["main", "staging", "master"];

    // Actions whose entire purpose is to commit and push on the runner's
    // behalf. A denylist is how this rule comes back after being removed from
    // the shell scripts.
    const PUSHING_ACTIONS: &[&str] = &[
        "git-auto-commit-action",
        "github-push-action",
        "add-and-commit",
        "auto-commit-action",
    ];

    let root = repo_root();
    let mut offenders = Vec::new();

    for path in workflow_files(&root) {
        let display = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();

        for (number, line) in read(&path).lines().enumerate() {
            let trimmed = line.trim();

            if let Some((_, refspec)) = trimmed.split_once("git push") {
                // Tokenise the refspec so `git push -u origin "$BRANCH"` — a
                // branch this job created — is not confused with
                // `git push origin HEAD:main`.
                let targets_protected = refspec
                    .split(|c: char| c.is_whitespace() || c == ':')
                    .map(|token| token.trim_matches(|c| c == '"' || c == '\'' || c == '`'))
                    .any(|token| PROTECTED.contains(&token));

                if targets_protected {
                    offenders.push(format!("{display}:{}: {trimmed}", number + 1));
                }
            }

            if trimmed.starts_with("uses:") || trimmed.starts_with("- uses:") {
                if let Some(action) = PUSHING_ACTIONS.iter().find(|a| trimmed.contains(*a)) {
                    offenders.push(format!("{display}:{}: uses {action}", number + 1));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "Security Requirements: a workflow pushes to an integration branch:\n  \
         {}\n\n\
         A bot commit on `main` or `staging` is code that reached a release \
         branch without having been read by anyone. Automation that wants to \
         change this repository proposes the change as a pull request from a \
         branch it creates, and a person merges it. See \
         `.github/workflows/release-prep.yml` for the shape.",
        offenders.join("\n  ")
    );
}

/// No workflow step may be exempted from failing.
///
/// `continue-on-error: true` was on the `Security Audit` job (issue #53: 22
/// RUSTSEC advisories reported, exit zero) and on every step of the release
/// workflow's `publish` job — which could not have succeeded under any
/// circumstances, and said so in green for four releases running.
///
/// A step that cannot fail is not evidence. If a failure is tolerable, the
/// tolerance belongs in the thing being checked — `.cargo/audit.toml` names
/// each accepted advisory and why — not in a flag that swallows every failure
/// including the ones nobody has seen yet.
#[test]
fn no_workflow_step_reports_success_on_failure() {
    let root = repo_root();
    let mut offenders = Vec::new();

    for path in workflow_files(&root) {
        let display = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();

        for (number, line) in read(&path).lines().enumerate() {
            let trimmed = line.trim().trim_start_matches("- ").trim();

            // Only the YAML key counts. The workflows explain in comments why
            // this flag was removed, and saying so must not trip the check.
            let Some(value) = trimmed.strip_prefix("continue-on-error:") else {
                continue;
            };

            if value.trim() != "false" {
                offenders.push(format!("{display}:{}: {trimmed}", number + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "Security Requirements: a workflow step is exempted from failing:\n  \
         {}\n\n\
         `continue-on-error: true` turns a check into a report nobody reads. \
         If specific failures are acceptable, enumerate them where they occur \
         (as `.cargo/audit.toml` does for advisories) so that the next, \
         unenumerated failure is still red.",
        offenders.join("\n  ")
    );
}

// ============================================================================
// Meta — the constitution itself
// ============================================================================

/// The constitution must exist and still state its non-negotiable principles.
///
/// Cheap tripwire: silently gutting the document would otherwise make every
/// test above pass while meaning nothing.
#[test]
fn constitution_is_present_and_states_its_principles() {
    let text = read(&repo_root().join(".specify/memory/constitution.md"));

    for heading in [
        "### I. One TCB",
        "### II. No Signature Without Physical Consent",
        "### III. Default Deny at the Network Edge",
        "### IV. Key Material Does Not Travel Over Convenience Transports",
        "### V. The Supported Install Is Auditable",
        "### VI. The Air-Gapped Side Has No Outbound Path",
        "### VII. Least Privilege on the Local Machine",
    ] {
        assert!(
            text.contains(heading),
            "The constitution no longer contains `{heading}`. Amending it is \
             allowed — it is a living document — but amendment requires a version \
             bump there and a CHANGELOG note, and the conformance tests here must \
             be updated in the same change."
        );
    }

    assert!(
        text.contains("NON-NEGOTIABLE"),
        "The constitution's non-negotiable markers are gone."
    );
}
