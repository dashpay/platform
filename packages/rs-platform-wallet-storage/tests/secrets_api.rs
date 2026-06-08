//! Type-shape + boundary guards for the `secrets` API
//! (SEC-REQ-4.1 / 4.4 / 4.5).
//!
//! Compiled only with `--features secrets`. Uses a tempdir-backed
//! `EncryptedFileStore` (always available under `secrets`).

#![cfg(feature = "secrets")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::{Error as KeyringError, Result as KeyringResult};
use platform_wallet_storage::secrets::{
    EncryptedFileStore, SecretBytes, SecretStore, SecretStoreError, SecretString, WalletId,
    SERVICE_PREFIX,
};

fn vault_path(dir: &Path) -> PathBuf {
    dir.join("vault.pwsvault")
}

fn open(dir: &Path) -> EncryptedFileStore {
    EncryptedFileStore::open(vault_path(dir), SecretString::new("test-pass")).unwrap()
}

fn service(w: WalletId) -> String {
    format!("{SERVICE_PREFIX}{}", w.to_hex())
}

/// `CredentialApi::get_secret` returns `Vec<u8>` per upstream — we
/// re-wrap it via `SecretBytes::new` at the consumer seam with no named
/// intermediate `Vec` binding. This binding only compiles when the
/// re-wrap type is exactly `SecretBytes`.
#[test]
fn get_secret_rewraps_into_zeroizing_at_consumer_seam() {
    let dir = tempfile::tempdir().unwrap();
    let s = open(dir.path());
    let w = WalletId::from([1; 32]);
    let entry = s.build(&service(w), "seed", None).unwrap();
    entry.set_secret(b"abc").unwrap();
    let wrapped: SecretBytes = SecretBytes::new(entry.get_secret().unwrap());
    assert_eq!(wrapped.expose_secret(), b"abc");
}

/// The secrets module is reachable and the store is object-safe
/// behind `Arc<dyn CredentialStoreApi + Send + Sync>` (SEC-REQ-4.5
/// positive build guard).
#[test]
fn secrets_tree_builds_and_is_object_safe() {
    let dir = tempfile::tempdir().unwrap();
    let s: Arc<dyn CredentialStoreApi + Send + Sync> = Arc::new(open(dir.path()));
    let w = WalletId::from([9; 32]);
    let entry: KeyringResult<_> = s.build(&service(w), "bip39_mnemonic", None);
    entry.unwrap().set_secret(b"x").unwrap();
    let e2 = s.build(&service(w), "bip39_mnemonic", None).unwrap();
    assert_eq!(e2.get_secret().unwrap(), b"x");
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

/// The bridged `keyring_core::Error` carries no secret in `Display`
/// (SEC-REQ-2.0.1 / 3.3 / CWE-209). `{:?}` is the dangerous shape
/// (it can echo `BadEncoding(Vec<u8>)` /
/// `BadDataFormat(Vec<u8>, _)`); the file backend never constructs
/// those variants with secret bytes, and our consumers must not
/// `{:?}`-print `keyring_core::Error` either (see `secrets_guard`).
#[test]
fn error_display_is_static_and_secret_free() {
    // Resident-vault model: a wrong passphrase is detected at open()
    // (verify-token check), not on get(). The exclusive vault lock is
    // released when the first store drops, so the wrong-pass reopen
    // can attempt acquisition. The typed distinction survives
    // losslessly on both the SPI and the SecretStore public paths.
    let dir = tempfile::tempdir().unwrap();
    let w = WalletId::from([4; 32]);
    {
        let store = open(dir.path());
        let entry = store.build(&service(w), "seed", None).unwrap();
        entry.set_secret(b"PLAINTEXTNEEDLE").unwrap();

        let inv = store.build(&service(w), "../bad", None).unwrap_err();
        match inv {
            KeyringError::Invalid(attr, _) => assert_eq!(attr, "user"),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    let typed = EncryptedFileStore::open(vault_path(dir.path()), SecretString::new("wrong-pass"))
        .expect_err("wrong pass must fail open");
    assert!(matches!(typed, SecretStoreError::WrongPassphrase));
    let rendered = format!("{typed}");
    assert!(!rendered.contains("PLAINTEXTNEEDLE"));
    assert!(!rendered.contains("wrong-pass"));

    // The same wrong-pass open through the public `SecretStore` keeps
    // the typed variant intact.
    let typed = SecretStore::file(vault_path(dir.path()), SecretString::new("wrong-pass"))
        .expect_err("wrong pass must fail open via SecretStore");
    assert!(matches!(typed, SecretStoreError::WrongPassphrase));

    // The SPI projection: a typed `SecretStoreError::WrongPassphrase`
    // converted to `KeyringError` rides in `NoStorageAccess` with the
    // typed error boxed as the source, recoverable losslessly. The
    // projection itself never sees the bytes, so a plaintext leak is
    // impossible at this seam.
    let spi: KeyringError = SecretStoreError::WrongPassphrase.into();
    let spi_rendered = format!("{spi}");
    assert!(!spi_rendered.contains("PLAINTEXTNEEDLE"));
    assert!(!spi_rendered.contains("wrong-pass"));
    match &spi {
        KeyringError::NoStorageAccess(src) => {
            assert!(matches!(
                src.downcast_ref::<SecretStoreError>(),
                Some(SecretStoreError::WrongPassphrase)
            ));
        }
        other => panic!("expected NoStorageAccess, got {other:?}"),
    }
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
