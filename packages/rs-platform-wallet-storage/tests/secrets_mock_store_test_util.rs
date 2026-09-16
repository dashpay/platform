//! Downstream proof that the `test-util` feature reaches `SecretStore::file_mock`
//! from OUTSIDE the crate (#4111).
//!
//! The unit tests all run under `cfg(test)`, which satisfies the
//! `cfg(any(test, feature = "test-util"))` gate on its own — so a misspelt
//! feature name in that gate would still pass them while breaking the one
//! caller the feature exists for (a downstream suite, which never gets
//! `cfg(test)` for THIS crate). Compiling this file against the crate as an
//! ordinary dependency is that assertion; the round-trip below then proves the
//! mock store is a real, working store and not just a name that links.

#![cfg(all(feature = "secrets", feature = "test-util"))]

use platform_wallet_storage::secrets::{SecretBytes, SecretStore, SecretString, WalletId};

/// Tighten the umask-0002 tempdir (0o775) to 0o700 so it passes the vault's
/// parent-dir permission check, then return a vault path inside it.
fn secure_vault_path(dir: &std::path::Path) -> std::path::PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    dir.join("vault.pwsvault")
}

/// A downstream caller swaps `file` → `file_mock` at construction and every
/// later call is transparently cheap, with no other signature change. The
/// floor-params assertions live in the crate's unit tests (the encoded
/// `KdfParams` are `pub(crate)`); what this pins is the public surface a
/// consumer actually touches.
#[test]
fn file_mock_is_reachable_and_round_trips_from_downstream() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::file_mock(
        secure_vault_path(dir.path()),
        SecretString::new("test-password"),
    )
    .expect("mock vault opens");

    let wallet = WalletId::from([7u8; 32]);
    let pw = SecretString::new("object-pw");

    store
        .set_secret(
            &wallet,
            "seed",
            &SecretBytes::from_slice(b"SEED"),
            Some(&pw),
        )
        .unwrap();
    assert_eq!(
        store
            .get_secret(&wallet, "seed", Some(&pw))
            .unwrap()
            .unwrap()
            .expose_secret(),
        b"SEED"
    );

    store.reprotect(&wallet, "seed", Some(&pw), None).unwrap();
    assert_eq!(
        store.get(&wallet, "seed").unwrap().unwrap().expose_secret(),
        b"SEED"
    );
}
