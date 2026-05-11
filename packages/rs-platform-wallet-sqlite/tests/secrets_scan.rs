#![allow(clippy::field_reassign_with_default)]

//! SEC-006 — schema-file substring scan for forbidden secret-material
//! tokens.
//!
//! The persister never stores mnemonics / seeds / private keys (see
//! SECRETS.md). This test grep-scans every file under `src/schema/`
//! and `migrations/` for ASCII substrings associated with secret
//! material. A new column or migration that smuggles in `private`,
//! `mnemonic`, `seed`, or `xpriv` breaks the test.
//!
//! The check is intentionally string-level: it does not parse SQL or
//! Rust. A column literally named `private_X` is the kind of mistake
//! we want to catch; legitimate uses of these words inside doc
//! comments are allow-listed via `tests/secrets_allowlist`.

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
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
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
    scan_dir(&manifest.join("src/schema"), &mut offenders);
    scan_dir(&manifest.join("migrations"), &mut offenders);
    assert!(
        offenders.is_empty(),
        "forbidden secret-material tokens found in schema files (see SECRETS.md):\n{}",
        offenders.join("\n")
    );
}
