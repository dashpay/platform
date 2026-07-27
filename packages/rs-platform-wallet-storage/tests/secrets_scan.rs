#![allow(clippy::field_reassign_with_default)]

//! Schema-file substring scan for forbidden secret-material tokens
//! (the load-bearing test for the NFR-10 / SECRETS.md boundary).
//!
//! The persister never stores mnemonics / seeds / private keys.
//! This test grep-scans every file under `src/sqlite/schema/` and
//! `migrations/` for ASCII substrings associated with secret material.
//! A new column, blob field, or comment that uses `private`,
//! `mnemonic`, `seed`, `xpriv`, or `secret` breaks the test, forcing
//! the author to rename or add an allow-list entry with rationale.
//!
//! Out of scope by design: files in `src/sqlite/` outside of
//! `schema/` (`persister.rs`, `backup.rs`, `buffer.rs`, `config.rs`,
//! `error.rs`, `migrations.rs`, `util/`) are NOT scanned. They never
//! define database columns and may legitimately reference the
//! forbidden tokens in doc comments. The future `src/secrets/`
//! submodule slot is exempt for the same reason.
//!
//! The check is intentionally string-level: it does not parse SQL or
//! Rust. A column literally named `private_X` is the kind of mistake
//! we want to catch; legitimate uses inside doc comments are
//! allow-listed via the `ALLOWLIST` constant below.

use std::path::Path;

const FORBIDDEN: &[&str] = &["private", "mnemonic", "seed", "xpriv", "secret"];

/// Doc-comment / identifier substrings we deliberately want to
/// permit even though they contain a forbidden token. Keep this list
/// tiny — each entry is a string that must appear verbatim in the
/// offending line for it to be ignored.
const ALLOWLIST: &[&str] = &[
    // `IdentityPublicKey` blob column carries only PUBLIC material;
    // the doc comment says so explicitly. Allow-listing the phrase
    // means future contributors can still surface the boundary.
    "PUBLIC material only",
    "No private bytes",
    "no private key",
    "private-key bytes",
    "public_key_blob",
    "public material",
    "do not derive private keys",
    "private keys are NOT",
];

fn line_is_allowlisted(line: &str) -> bool {
    ALLOWLIST.iter().any(|needle| line.contains(needle))
}

fn scan_dir(dir: &Path, offenders: &mut Vec<String>) {
    // Fail loudly on a missing/unreadable scan root — a directory
    // rename or build-script bug that points the scanner at a
    // non-existent path must NOT degrade this security guardrail into a
    // silent no-op.
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| {
        panic!(
            "secrets-scan: scan root `{}` is unreadable ({e}); this guardrail \
             requires every configured root to exist — update the call sites \
             or fix the path",
            dir.display()
        )
    });
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            scan_dir(&p, offenders);
            continue;
        }
        if !p
            .extension()
            .is_some_and(|e| e == "rs" || e == "sql" || e == "md")
        {
            continue;
        }
        // Skip the test file itself; it intentionally lists the
        // forbidden tokens.
        if p.file_name().and_then(|s| s.to_str()) == Some("secrets_scan.rs") {
            continue;
        }
        let body = match std::fs::read_to_string(&p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for (idx, line) in body.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            for needle in FORBIDDEN {
                if lower.contains(needle) && !line_is_allowlisted(line) {
                    offenders.push(format!(
                        "{}:{}: contains `{needle}` — {}",
                        p.display(),
                        idx + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
}

#[test]
fn no_secret_substrings_in_schema_or_migrations() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    // `src/sqlite/schema` (SQLite-backend column definitions and blob
    // encoders) and `migrations/` (refinery DDL) are the entire
    // persistence surface for non-secret material. `src/secrets/` is
    // exempt by design — that submodule legitimately mentions
    // `private`, `mnemonic`, `seed`; its own `secrets_guard.rs` test
    // covers it.
    scan_dir(&manifest.join("src/sqlite/schema"), &mut offenders);
    scan_dir(&manifest.join("migrations"), &mut offenders);
    assert!(
        offenders.is_empty(),
        "forbidden secret-material tokens found in schema files (see SECRETS.md):\n{}",
        offenders.join("\n")
    );
}
