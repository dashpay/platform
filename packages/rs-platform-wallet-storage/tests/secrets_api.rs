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
    MAX_PASSPHRASE_LEN, MAX_SECRET_LEN, SERVICE_PREFIX,
};

fn vault_path(dir: &Path) -> PathBuf {
    // `open` refuses a group/other-writable parent dir; a umask-0002
    // tempdir lands at 0o775, so tighten it to 0o700 first.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .expect("tighten vault parent dir to 0o700 so open() passes the perm check");
    }
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

/// SECRETS.md (the on-disk vault is "explicitly attacker-controllable",
/// defenses must "fail closed", error doc: "malformed vault file ...
/// truncated header"). A garbage / truncated / empty / non-UTF-8 vault
/// fed through the FULL `EncryptedFileStore::open` integration path
/// (`read_vault_at` -> `Vec::with_capacity(len)` -> `format::deserialize`)
/// must surface a clean typed `MalformedVault` and NEVER panic. The
/// `format.rs` unit tests exercise `deserialize` in isolation; they do
/// not prove the file-open seam (perms check, size cap, allocation,
/// take()) is wired to the same clean-error outcome.
#[cfg(unix)]
#[test]
fn garbage_vault_file_fails_closed_at_open_no_panic() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let cases: &[(&str, &[u8])] = &[
        ("empty", b""),
        ("ascii-garbage", b"this is not a vault at all"),
        ("truncated-json", b"{\"version\":1,\"kdf\":{\"id\":1,"),
        ("non-utf8", &[0xff, 0xfe, 0x00, 0x01, 0x80, 0x80]),
        ("json-but-not-a-vault", b"{\"hello\":\"world\"}"),
    ];
    for (name, bytes) in cases {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        fs::write(&path, bytes).unwrap();
        // Match the resident-vault perm precondition so the failure is
        // attributable to parsing, not to the (separately tested) perm
        // refusal.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("garbage vault must fail to open");
        assert!(
            matches!(err, SecretStoreError::MalformedVault),
            "case `{name}`: expected MalformedVault, got {err:?}"
        );
        // The clean error must not echo the offending input bytes.
        let rendered = format!("{err}");
        assert!(
            !rendered.contains("not a vault") && !rendered.contains("hello"),
            "case `{name}`: error leaked input bytes: {rendered}"
        );
    }
}

/// SECRETS.md: an unknown/rolled-forward `format_version` is refused
/// fail-closed through the file-open seam, distinct from a malformed
/// body. The format.rs unit test proves the parser; this proves the
/// `open()` path preserves the distinction end to end.
#[cfg(unix)]
#[test]
fn unknown_version_vault_is_refused_at_open() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = vault_path(dir.path());
    // A structurally JSON document whose `version` is in the future:
    // the lax probe reads it, then the version gate rejects it before
    // any KDF/AEAD work.
    fs::write(&path, br#"{"version":999,"extra":"tolerated-by-probe"}"#).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

    let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
        .expect_err("unknown version must fail to open");
    assert!(
        matches!(err, SecretStoreError::VersionUnsupported { found: 999 }),
        "expected VersionUnsupported{{999}}, got {err:?}"
    );
}

/// The passphrase ceiling is part of the public contract, not an
/// internal detail: `MAX_PASSPHRASE_LEN` is re-exported, the boundary is
/// inclusive, and a violation surfaces as the typed
/// `PassphraseTooLong` carrying lengths only — never the passphrase.
#[test]
fn passphrase_cap_is_public_typed_and_leak_free() {
    let dir = tempfile::tempdir().unwrap();
    let path = vault_path(dir.path());

    // At the cap: accepted, and the vault it opens is fully usable.
    let store = EncryptedFileStore::open(&path, SecretString::new("p".repeat(MAX_PASSPHRASE_LEN)))
        .expect("a passphrase exactly at MAX_PASSPHRASE_LEN must be accepted");
    let w = WalletId::from([7; 32]);
    store
        .build(&service(w), "seed", None)
        .unwrap()
        .set_secret(b"still works")
        .unwrap();
    drop(store);

    // Past it: refused, typed, and the message carries no plaintext.
    let needle = "PLAINTEXTNEEDLE".repeat(MAX_PASSPHRASE_LEN / 15 + 1);
    let err = EncryptedFileStore::open(&path, SecretString::new(needle.clone()))
        .expect_err("a passphrase past MAX_PASSPHRASE_LEN must be refused");
    assert!(
        matches!(err, SecretStoreError::PassphraseTooLong { found, max }
            if found == needle.len() && max == MAX_PASSPHRASE_LEN),
        "got {err:?}"
    );
    let rendered = format!("{err} {err:?}");
    assert!(
        !rendered.contains("PLAINTEXTNEEDLE"),
        "error leaked the passphrase: {rendered}"
    );
}

/// The two public size ceilings stay put, pinned by value.
///
/// Both are load-bearing rows of the locked-memory budget documented at
/// `MAX_SECRET_LEN`, and both are public API — so a change to either is a
/// change to what callers may store, not an implementation detail. Spelt
/// as literals rather than as page arithmetic: they are product ceilings
/// chosen to cover real secrets cheaply, and deriving them from a page
/// size is what previously tied them to one host's idea of a page.
///
/// The page size remains an assumption about the host, so the store
/// construction below asserts it holds here: a host with larger pages
/// cannot open a store at all, which is what stops the budget these
/// ceilings belong to from silently describing nobody.
#[test]
fn public_size_ceilings_stay_pinned() {
    assert_eq!(MAX_SECRET_LEN, 8176);
    assert_eq!(MAX_PASSPHRASE_LEN, 4080);

    let dir = tempfile::tempdir().unwrap();
    let _store = open(dir.path());
}
