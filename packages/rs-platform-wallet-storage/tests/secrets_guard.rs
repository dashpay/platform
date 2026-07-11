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

/// Whether `s` holds at least one non-comment occurrence of `needle`.
/// Strips full-line `//`/`///`/`*` comment lines before checking so a
/// doc-line mention of a sink or `expose_secret` is not a false hit.
fn contains_in_code(s: &str, needle: &str) -> bool {
    s.lines()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("//") || t.starts_with("*"))
        })
        .any(|l| l.contains(needle))
}

const MAX_STMT_LINES: usize = 12;

/// Scan one file body for any logging/formatting sink paired with
/// `expose_secret` in the same whole statement. `origin` labels the
/// offender lines.
///
/// Whole-statement windows: starting at each line, concatenate following
/// lines until the parentheses balance AND a `;` or `{` is seen (or a
/// small line cap is hit). A 3+-line `tracing::…(field = expose_secret(),
/// …)` call collapses into one window so the sink and `expose_secret`
/// land in the same string — a fixed 2-line window would split them.
fn scan_text(origin: &str, body: &str, offenders: &mut Vec<String>) {
    let lines: Vec<&str> = body.lines().collect();
    for start in 0..lines.len() {
        let mut joined = String::new();
        let mut depth: i32 = 0;
        let mut end = start;
        while end < lines.len() && end - start < MAX_STMT_LINES {
            let line = lines[end];
            joined.push_str(line);
            joined.push(' ');
            // Track paren balance over code only (ignore comment text).
            let code = line.trim_start();
            if !(code.starts_with("//") || code.starts_with("*")) {
                for c in line.chars() {
                    match c {
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        _ => {}
                    }
                }
            }
            let balanced = depth <= 0;
            let terminated =
                line.contains(';') || line.trim_end().ends_with('{') || code.is_empty();
            end += 1;
            if balanced && terminated {
                break;
            }
        }

        if !contains_in_code(&joined, "expose_secret") {
            continue;
        }
        for sink in SINKS {
            if contains_in_code(&joined, sink) && contains_in_code(&joined, "expose_secret") {
                offenders.push(format!(
                    "{origin}:{}: `{sink}` paired with `expose_secret` — {}",
                    start + 1,
                    lines[start].trim()
                ));
                break;
            }
        }
    }
}

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
        scan_text(&p.display().to_string(), &body, offenders);
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

/// Non-vacuous proof: a 3-line `tracing::error!(...)` call that splays the
/// `expose_secret()` argument onto its own line is caught by the
/// whole-statement window. A fixed 2-line window would have paired the
/// sink line only with the field line and missed the leaking argument,
/// so this case guards against silently regressing the scan to that
/// narrower form.
#[test]
fn widened_scan_catches_three_line_leak() {
    let planted = r#"
fn leak() {
    tracing::error!(
        wallet_id = %wid,
        secret = %s.expose_secret(),
    );
}
"#;
    let mut offenders = Vec::new();
    scan_text("planted", planted, &mut offenders);
    assert!(
        !offenders.is_empty(),
        "widened scan must catch a 3-line tracing call leaking expose_secret"
    );

    // The same leak split across only two lines is the case the narrow
    // window would have caught — keep it caught too.
    let two_line = "    tracing::error!(field = 1,\n        secret = %s.expose_secret());";
    let mut two_off = Vec::new();
    scan_text("planted-2line", two_line, &mut two_off);
    assert!(!two_off.is_empty(), "two-line leak must still be caught");

    // A clean 3-line call with no secret must NOT be flagged (no false
    // positive from over-eager statement joining).
    let clean = "    tracing::error!(\n        wallet_id = %wid,\n        label = %label,\n    );";
    let mut clean_off = Vec::new();
    scan_text("planted-clean", clean, &mut clean_off);
    assert!(clean_off.is_empty(), "clean call must not be flagged");
}

/// `keyring_core::Error` embeds raw `Vec<u8>` in
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
        "`{{:?}}` debug-format paired with `keyring_core::Error` \
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
