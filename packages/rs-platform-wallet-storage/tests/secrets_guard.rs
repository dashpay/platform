//! Positive secret-leak guard for `src/secrets/` (SEC-REQ-4.5.1).
//!
//! `tests/secrets_scan.rs` deliberately exempts `src/secrets/`, so this
//! module needs its own string-level guard: no `tracing::*` /
//! `println!` / `eprintln!` / `format!`-family call may take an
//! `expose_secret()` result as an argument. Same spirit as
//! `secrets_scan.rs` — it does not parse Rust; a leaking line that
//! pairs a logging/formatting macro with `expose_secret` on the same
//! logical statement is the mistake we catch.
//!
//! Compiled only with `--features secrets` (the tree does not exist
//! otherwise); a no-op assertion keeps the default build green.

#![cfg(feature = "secrets")]

use std::path::Path;

/// Logging / formatting sinks that must never receive plaintext.
const SINKS: &[&str] = &[
    "tracing::trace!",
    "tracing::debug!",
    "tracing::info!",
    "tracing::warn!",
    "tracing::error!",
    "trace!(",
    "debug!(",
    "info!(",
    "warn!(",
    "error!(",
    "println!(",
    "eprintln!(",
    "print!(",
    "eprint!(",
    "format!(",
    "write!(",
    "writeln!(",
    "panic!(",
    "dbg!(",
];

fn scan(dir: &Path, offenders: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            scan(&p, offenders);
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let Ok(body) = std::fs::read_to_string(&p) else {
            continue;
        };
        // Join continuations: a leaking call may wrap across lines.
        for (idx, window) in body.lines().collect::<Vec<_>>().windows(2).enumerate() {
            let joined = format!("{} {}", window[0], window[1]);
            if !joined.contains("expose_secret") {
                continue;
            }
            // The `expose_secret` definitions/doc lines in `secret.rs`
            // and intentional debug-redaction tests are not sinks.
            if window.iter().any(|l| {
                let t = l.trim_start();
                t.starts_with("//") || t.starts_with("///") || t.starts_with("*")
            }) && !SINKS.iter().any(|s| joined.contains(s))
            {
                continue;
            }
            for sink in SINKS {
                if joined.contains(sink) && joined.contains("expose_secret") {
                    offenders.push(format!(
                        "{}:{}: `{sink}` paired with `expose_secret` — {}",
                        p.display(),
                        idx + 1,
                        window[0].trim()
                    ));
                }
            }
        }
    }
}

#[test]
fn no_secret_sink_in_secrets_module() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    scan(&manifest.join("src/secrets"), &mut offenders);
    assert!(
        offenders.is_empty(),
        "secret material may be reaching a log/format sink:\n{}",
        offenders.join("\n")
    );
}

/// Smythe EDIT-2 — `keyring_core::Error` embeds raw `Vec<u8>` in
/// `BadEncoding` / `BadDataFormat`; `Display` is safe but `{:?}` is
/// dangerous. Forbid `{:?}` debug-formatting of any binding the seam
/// code holds as a `keyring_core::Error` inside `src/secrets/`.
///
/// String-level scan: it flags `{:?}` paired with `KeyringError` /
/// `keyring_core::Error` on the same source line. The unit-test files
/// for the bridge necessarily print the error in assert messages —
/// those tests live in this `tests/` tree, not under `src/secrets/`.
#[test]
fn no_debug_format_of_keyring_error_in_secrets_module() {
    const DEBUG_TOKENS: &[&str] = &["{:?}", "{e:?}", "{err:?}", "{:#?}"];
    const ERROR_NAMES: &[&str] = &["KeyringError", "keyring_core::Error"];

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    visit(&manifest.join("src/secrets"), &mut offenders);
    assert!(
        offenders.is_empty(),
        "Smythe EDIT-2: `{{:?}}` debug-format paired with `keyring_core::Error` \
         in src/secrets/ (BadEncoding/BadDataFormat embed raw Vec<u8>):\n{}",
        offenders.join("\n")
    );

    fn visit(dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                visit(&p, out);
                continue;
            }
            if p.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let Ok(body) = std::fs::read_to_string(&p) else {
                continue;
            };
            for (idx, line) in body.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("*") {
                    continue;
                }
                let has_dbg = DEBUG_TOKENS.iter().any(|t| line.contains(t));
                let has_err = ERROR_NAMES.iter().any(|n| line.contains(n));
                if has_dbg && has_err {
                    out.push(format!("{}:{}: {}", p.display(), idx + 1, line.trim()));
                }
            }
        }
    }
}
