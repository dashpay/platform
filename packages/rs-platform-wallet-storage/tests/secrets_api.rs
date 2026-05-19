//! Type-shape + boundary guards for the `secrets` API
//! (SEC-REQ-4.1 / 4.4 / 4.5, TC-082 parity).
//!
//! Compiled only with `--features secrets`. Uses `EncryptedFileStore`
//! (always available under `secrets`); `MemoryStore` is intentionally
//! unreachable here (SEC-REQ-2.3.1) — it is exercised only by the
//! crate's in-module unit tests.

#![cfg(feature = "secrets")]

use std::path::Path;

use platform_wallet_storage::secrets::{
    EncryptedFileStore, SecretBytes, SecretStore, SecretStoreError, SecretString, WalletId,
};

fn open(dir: &Path) -> EncryptedFileStore {
    EncryptedFileStore::open(dir, SecretString::new("test-pass")).unwrap()
}

/// `SecretStore::get` returns `Option<SecretBytes>`, never a bare
/// `Vec<u8>` (SEC-REQ-4.1). This binding only compiles if the type is
/// exactly that.
#[test]
fn get_returns_zeroizing_wrapper_not_vec() {
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    let w = WalletId::from([1; 32]);
    s.put(w, "seed", b"abc").unwrap();
    let got: Option<SecretBytes> = s.get(w, "seed").unwrap();
    assert_eq!(got.unwrap().expose_secret(), b"abc");
}

/// The secrets module is reachable, compiles, and round-trips through
/// `dyn SecretStore` (SEC-REQ-4.5 positive build guard).
#[test]
fn secrets_tree_builds_and_is_object_safe() {
    let dir = tempfile::tempdir().unwrap();
    let s: std::sync::Arc<dyn SecretStore> = std::sync::Arc::new(open(dir.path()));
    let w = WalletId::from([9; 32]);
    s.put(w, "bip39_mnemonic", b"x").unwrap();
    assert!(s.get(w, "bip39_mnemonic").unwrap().is_some());
}

/// No `Box<dyn Error>` in the `secrets` tree's public surface — TC-082
/// parity for the module the schema scanner does not cover.
#[test]
fn no_box_dyn_error_in_secrets_src() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/secrets");
    let mut offenders = Vec::new();
    walk(&dir, &mut offenders);
    assert!(
        offenders.is_empty(),
        "Box<dyn Error> found in secrets src:\n{}",
        offenders.join("\n")
    );

    fn walk(dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
                continue;
            }
            if p.extension().and_then(|x| x.to_str()) != Some("rs") {
                continue;
            }
            let Ok(body) = std::fs::read_to_string(&p) else {
                continue;
            };
            for (i, line) in body.lines().enumerate() {
                // The rule bans the *type* in code; prose explaining
                // the rule (doc/line comments) is not a violation.
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("*") {
                    continue;
                }
                let s = line.replace(' ', "");
                if s.contains("Box<dyn") && s.contains("Error") {
                    out.push(format!("{}:{}", p.display(), i + 1));
                }
            }
        }
    }
}

/// The error enum carries no secret in `Display` (SEC-REQ-2.0.1 /
/// 3.3 / CWE-209).
#[test]
fn error_display_is_static_and_secret_free() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(dir.path());
    let w = WalletId::from([4; 32]);
    store.put(w, "seed", b"PLAINTEXTNEEDLE").unwrap();
    let bad = EncryptedFileStore::open(dir.path(), SecretString::new("wrong-pass")).unwrap();
    let err = bad.get(w, "seed").unwrap_err();
    let rendered = format!("{err}");
    assert!(!rendered.contains("PLAINTEXTNEEDLE"));
    assert!(!rendered.contains("wrong-pass"));
    assert_eq!(rendered, "wrong passphrase");

    let inv = store.put(w, "../bad", b"x").unwrap_err();
    assert!(matches!(inv, SecretStoreError::InvalidLabel));
    assert_eq!(format!("{inv}"), "invalid label");
}

/// `SecretBytes`/`SecretString` `Debug` is redacted at the API
/// boundary (SEC-REQ-3.3).
#[test]
fn wrapper_debug_is_redacted() {
    let b = SecretBytes::from_slice(b"PLAINTEXTNEEDLE");
    assert!(!format!("{b:?}").contains("PLAINTEXT"));
    let s = SecretString::new("PLAINTEXTNEEDLE");
    assert!(!format!("{s:?}").contains("PLAINTEXT"));
}
