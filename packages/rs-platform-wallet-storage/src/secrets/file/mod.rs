//! [`EncryptedFileStore`] — passphrase-encrypted on-disk vault.
//!
//! One vault file per `wallet_id` (path namespaced by `wallet_id`
//! hex). Argon2id KDF + XChaCha20-Poly1305 AEAD, AAD-bound to
//! `(format_version, wallet_id, label)`, written atomically at mode
//! 0600. Implements the upstream `keyring_core::api::CredentialStoreApi`
//! SPI; per-`(service, user)` credentials implement `CredentialApi`.
//!
//! ## Threat coverage
//!
//! Covers **A1** (other local user), **A4** (lost laptop / cold
//! backup), **A6** (synced backup of the vault file): the at-rest file
//! is Argon2id + AEAD, useless without the passphrase. Does **not**
//! cover **A3** (passphrase / derived key resident while unlocked), a
//! weak operator passphrase (KDF raises cost, does not eliminate the
//! risk — accepted, AR-2), or **A5** if the derived key / plaintext is
//! swapped or core-dumped while unlocked (best-effort mitigated by
//! zeroize + mlock, not eliminated).

mod crypto;
pub(crate) mod error;
mod format;

use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use keyring_core::api::{Credential, CredentialApi, CredentialPersistence, CredentialStoreApi};
use keyring_core::{Entry, Error as KeyringError, Result as KeyringResult};

use crypto::{KdfParams, SALT_LEN};
use error::FileStoreError;
use format::{Entry as VaultEntry, Header};

use super::secret::{SecretBytes, SecretString};
use super::validate::{validated_label, WalletId};

/// Upstream service-prefix for vault entries. The full `service`
/// string is `SERVICE_PREFIX + hex(wallet_id)`, mapping each wallet
/// to its own keyring "service" namespace.
pub const SERVICE_PREFIX: &str = "dash.platform-wallet-storage/";

/// Vendor / id tags published through `CredentialStoreApi`.
const VENDOR: &str = "dash.platform-wallet-storage";
const STORE_ID: &str = "encrypted-file-store-v1";

/// A passphrase-encrypted file-backed credential store.
///
/// The passphrase is held in a [`SecretString`] for the store's
/// lifetime so each operation can re-derive the per-vault key; it is
/// never written anywhere and is zeroized when the store drops
/// (SEC-REQ-2.2.13). The derived AEAD key is recomputed per operation
/// and dropped (zeroized) immediately after use — it is never retained
/// on the struct.
pub struct EncryptedFileStore {
    inner: Arc<EncryptedFileStoreInner>,
}

/// Reference-counted backing so credentials returned from
/// [`CredentialStoreApi::build`] hold a clone of the store without
/// keeping the public handle alive.
struct EncryptedFileStoreInner {
    dir: PathBuf,
    passphrase: SecretString,
}

impl EncryptedFileStore {
    /// Open (or prepare to create) a vault store rooted at `dir`,
    /// unlocked by `passphrase`. `dir` is created if missing.
    pub fn open(dir: impl AsRef<Path>, passphrase: SecretString) -> Result<Self, FileStoreError> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;
        Ok(Self {
            inner: Arc::new(EncryptedFileStoreInner { dir, passphrase }),
        })
    }

    /// Re-encrypt every entry for `wallet_id` under a fresh salt +
    /// fresh per-entry nonces, then atomically replace the vault. No
    /// `.bak` retains old key material (SEC-REQ-2.2.12). Replaces this
    /// store's passphrase atomically on success.
    pub fn rekey(
        &mut self,
        wallet_id: WalletId,
        new_passphrase: SecretString,
    ) -> Result<(), FileStoreError> {
        // The store must hold a unique reference so the swap is
        // observable to every outstanding credential consistently. A
        // live credential clones the inner `Arc` in `build()`, a
        // caller-reachable state, so this is a recoverable typed error,
        // not a panic — but still fail-loud: never a silent stale-handle
        // rekey.
        let Some(inner) = Arc::get_mut(&mut self.inner) else {
            return Err(FileStoreError::Busy);
        };
        inner.rekey(wallet_id, new_passphrase)
    }

    /// Store `bytes` under `(wallet_id, label)`, returning the typed
    /// [`FileStoreError`] (lossless — no `keyring_core::Error` seam).
    /// The public [`SecretStore`](crate::secrets::SecretStore) file arm
    /// delegates here so the structural error distinction survives.
    pub(crate) fn put_bytes(
        &self,
        wallet_id: &WalletId,
        label: &str,
        bytes: &[u8],
    ) -> Result<(), FileStoreError> {
        self.inner.put(wallet_id, label, bytes)
    }

    /// Retrieve the plaintext under `(wallet_id, label)`, or `None` if
    /// absent, returning the typed [`FileStoreError`].
    pub(crate) fn get_bytes(
        &self,
        wallet_id: &WalletId,
        label: &str,
    ) -> Result<Option<Vec<u8>>, FileStoreError> {
        self.inner.get(wallet_id, label)
    }

    /// Delete the entry under `(wallet_id, label)`; `Ok(false)` if it was
    /// already absent. Returns the typed [`FileStoreError`].
    pub(crate) fn delete_bytes(
        &self,
        wallet_id: &WalletId,
        label: &str,
    ) -> Result<bool, FileStoreError> {
        self.inner.delete(wallet_id, label)
    }

    #[cfg(test)]
    pub(crate) fn test_vault_path(&self, wallet_id: &WalletId) -> PathBuf {
        self.inner.vault_path(wallet_id)
    }

    #[cfg(test)]
    pub(crate) fn test_read_vault(
        &self,
        path: &Path,
    ) -> Result<Option<(Header, Vec<VaultEntry>)>, FileStoreError> {
        self.inner.read_vault(path)
    }

    #[cfg(test)]
    pub(crate) fn test_write_vault(
        &self,
        path: &Path,
        header: &Header,
        entries: &[VaultEntry],
    ) -> Result<(), FileStoreError> {
        self.inner.write_vault(path, header, entries)
    }
}

impl EncryptedFileStoreInner {
    fn vault_path(&self, wallet_id: &WalletId) -> PathBuf {
        self.dir.join(format!("{}.pwsvault", wallet_id.to_hex()))
    }

    /// Build a fresh header for a brand-new vault: random salt, default
    /// Argon2 params, and a passphrase-verification token sealed under
    /// the freshly derived key (SEC-REQ-2.2.x; the token is the
    /// mixed-key-corruption guard).
    fn new_header(
        &self,
        wallet_id: &WalletId,
        passphrase: &SecretString,
    ) -> Result<(Header, SecretBytes), FileStoreError> {
        let mut salt = [0u8; SALT_LEN];
        crypto::random_bytes(&mut salt)?;
        let params = KdfParams::default_target();
        let key = crypto::derive_key(passphrase.expose_secret().as_bytes(), &salt, params)?;
        let v_aad = format::verify_aad(format::FORMAT_VERSION, wallet_id.as_bytes());
        let (verify_nonce, verify_ct) = crypto::seal(&key, &v_aad, format::VERIFY_CONSTANT)?;
        Ok((
            Header {
                params,
                salt,
                verify_nonce,
                verify_ct,
            },
            key,
        ))
    }

    /// Derive the key from the supplied passphrase and verify it
    /// against the header's token *before* any entry is touched. A
    /// wrong passphrase fails the token's AEAD tag (constant-time) and
    /// yields `WrongPassphrase` with no plaintext, so a mismatched key is
    /// rejected before any entry is touched (SEC-REQ-2.2.x).
    fn derive_and_verify(
        &self,
        wallet_id: &WalletId,
        header: &Header,
    ) -> Result<SecretBytes, FileStoreError> {
        let key = crypto::derive_key(
            self.passphrase.expose_secret().as_bytes(),
            &header.salt,
            header.params,
        )?;
        let v_aad = format::verify_aad(format::FORMAT_VERSION, wallet_id.as_bytes());
        match crypto::open(&key, &header.verify_nonce, &v_aad, &header.verify_ct) {
            Ok(_) => Ok(key),
            Err(FileStoreError::Decrypt) => Err(FileStoreError::WrongPassphrase),
            Err(e) => Err(e),
        }
    }

    /// Read + parse a vault file, or `None` if it does not exist.
    /// Refuses a pre-existing file with looser-than-0600 perms
    /// (SEC-REQ-2.2.10).
    fn read_vault(&self, path: &Path) -> Result<Option<(Header, Vec<VaultEntry>)>, FileStoreError> {
        match fs::metadata(path) {
            Ok(meta) => {
                check_perms(&meta)?;
                let bytes = fs::read(path)?;
                Ok(Some(format::deserialize(&bytes)?))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Atomically replace the vault, cross-platform (SEC-REQ-2.2.10/.11).
    ///
    /// Stages into a `NamedTempFile` in the SAME directory (so `persist`
    /// cannot fail cross-volume), tightens perms to 0600 on Unix before
    /// any byte is written, then: `write_all` → `sync_all` →
    /// `persist(path)` → Unix parent-dir fsync. The destination is never
    /// pre-removed, so a crash leaves either the old or the new vault,
    /// never an absent one. On `persist` failure the temp drops and
    /// self-cleans — no manual remove racing it. The temp holds only
    /// ciphertext+header, never plaintext.
    fn write_vault(
        &self,
        path: &Path,
        header: &Header,
        entries: &[VaultEntry],
    ) -> Result<(), FileStoreError> {
        let serialized = format::serialize(header, entries);
        // `persist` is atomic-replace only within one filesystem, so the
        // temp MUST share the destination's parent dir (mirrors
        // sqlite/backup.rs).
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let write = || -> Result<(), FileStoreError> {
            let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
            // tempfile creates the file private-to-owner on every OS; on
            // Unix we additionally pin 0600 (belt-and-suspenders). On
            // Windows the private-by-default ACL is sufficient for v1.
            set_restrictive_perms(tmp.as_file())?;
            tmp.as_file_mut().write_all(&serialized)?;
            tmp.as_file().sync_all()?;
            tmp.persist(path).map_err(|e| e.error)?;
            // Windows: directory durability relies on NTFS metadata
            // journaling; no dir-fsync primitive exists there.
            #[cfg(unix)]
            {
                let d = fs::File::open(parent)?;
                d.sync_all()?;
            }
            Ok(())
        };
        write().inspect_err(|e| {
            // Operators must see a failed durable write — paths are
            // caller-supplied non-secret (FileStoreError::Io doc); Display
            // only, never the secret.
            tracing::warn!(error = %e, "failed to write vault file");
        })
    }

    fn rekey(
        &mut self,
        wallet_id: WalletId,
        new_passphrase: SecretString,
    ) -> Result<(), FileStoreError> {
        let path = self.vault_path(&wallet_id);
        let Some((old_header, old_entries)) = self.read_vault(&path)? else {
            self.passphrase = new_passphrase;
            return Ok(());
        };
        let old_key = self.derive_and_verify(&wallet_id, &old_header)?;
        let (new_header, new_key) = self.new_header(&wallet_id, &new_passphrase)?;

        let mut new_entries = Vec::with_capacity(old_entries.len());
        for e in &old_entries {
            let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), &e.label);
            // `derive_and_verify` already proved the old passphrase via
            // the header token, so an entry tag failure is corruption,
            // not a wrong passphrase. Operators must see this — log the
            // non-secret wallet-id/label, never the secret. The first such
            // failure aborts the rekey, so this is not a hot path.
            let pt =
                crypto::open(&old_key, &e.nonce, &aad, &e.ciphertext).map_err(|err| match err {
                    FileStoreError::Decrypt => {
                        tracing::error!(
                            wallet_id = %wallet_id.to_hex(),
                            label = %e.label,
                            "vault entry failed integrity check during rekey (corruption or tampering)"
                        );
                        FileStoreError::Corruption
                    }
                    other => other,
                })?;
            let (nonce, ct) = crypto::seal(&new_key, &aad, pt.expose_secret())?;
            new_entries.push(VaultEntry {
                label: e.label.clone(),
                nonce,
                ciphertext: ct,
            });
        }
        self.write_vault(&path, &new_header, &new_entries)?;
        self.passphrase = new_passphrase;
        Ok(())
    }

    /// `put` — overwrite-safe atomic seal under `(wallet_id, label)`.
    fn put(&self, wallet_id: &WalletId, label: &str, bytes: &[u8]) -> Result<(), FileStoreError> {
        let label = validated_label(label)?.to_string();
        let path = self.vault_path(wallet_id);
        let (header, key, mut entries) = match self.read_vault(&path)? {
            Some((header, entries)) => {
                let key = self.derive_and_verify(wallet_id, &header)?;
                (header, key, entries)
            }
            None => {
                let (header, key) = self.new_header(wallet_id, &self.passphrase)?;
                (header, key, Vec::new())
            }
        };
        let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), &label);
        let (nonce, ciphertext) = crypto::seal(&key, &aad, bytes)?;
        entries.retain(|e| e.label != label);
        entries.push(VaultEntry {
            label,
            nonce,
            ciphertext,
        });
        self.write_vault(&path, &header, &entries)
    }

    /// `get` — returns the raw plaintext as `Vec<u8>` (the upstream
    /// SPI contract). Callers wrap into [`SecretBytes`] at the seam.
    /// `NoEntry`-shaped absence rides as `Ok(None)`.
    fn get(&self, wallet_id: &WalletId, label: &str) -> Result<Option<Vec<u8>>, FileStoreError> {
        let label = validated_label(label)?;
        let path = self.vault_path(wallet_id);
        let Some((header, entries)) = self.read_vault(&path)? else {
            return Ok(None);
        };
        let key = self.derive_and_verify(wallet_id, &header)?;
        let Some(entry) = entries.iter().find(|e| e.label == label) else {
            return Ok(None);
        };
        let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), label);
        match crypto::open(&key, &entry.nonce, &aad, &entry.ciphertext) {
            Ok(pt) => Ok(Some(pt.expose_secret().to_vec())),
            // The header verify-token already passed, so the passphrase is
            // correct: an entry tag failure here is corruption/tampering,
            // not a wrong passphrase. Operators must see this — log the
            // non-secret wallet-id/label, never the secret.
            Err(FileStoreError::Decrypt) => {
                tracing::error!(
                    wallet_id = %wallet_id.to_hex(),
                    label = %label,
                    "vault entry failed integrity check (corruption or tampering)"
                );
                Err(FileStoreError::Corruption)
            }
            Err(e) => Err(e),
        }
    }

    /// `delete` — upstream-compliant: returns whether an entry was
    /// removed so the SPI seam can surface `NoEntry` (D3, per the
    /// `CredentialApi::delete_credential` contract).
    fn delete(&self, wallet_id: &WalletId, label: &str) -> Result<bool, FileStoreError> {
        let label = validated_label(label)?;
        let path = self.vault_path(wallet_id);
        let Some((header, mut entries)) = self.read_vault(&path)? else {
            return Ok(false);
        };
        // Verify the passphrase before mutating, so a wrong pass can
        // neither delete an entry nor rewrite the vault.
        self.derive_and_verify(wallet_id, &header)?;
        let before = entries.len();
        entries.retain(|e| e.label != label);
        if entries.len() == before {
            return Ok(false);
        }
        self.write_vault(&path, &header, &entries)?;
        Ok(true)
    }
}

/// Parse a `service` string into a [`WalletId`]. The slash-prefixed
/// allowlist-disjoint shape (`label` never contains `/`) means an
/// attacker-controlled label cannot smuggle a bogus wallet id.
fn parse_service(service: &str) -> Result<WalletId, KeyringError> {
    let Some(hex) = service.strip_prefix(SERVICE_PREFIX) else {
        return Err(KeyringError::Invalid(
            "service".to_string(),
            "expected dash.platform-wallet-storage/<wallet-id-hex>".to_string(),
        ));
    };
    if hex.len() != 64 {
        return Err(KeyringError::Invalid(
            "service".to_string(),
            "wallet id hex must be 64 chars".to_string(),
        ));
    }
    // `hex::decode_to_slice` accepts uppercase, but the service string is
    // always constructed lowercase (`WalletId::to_hex`). Reject uppercase
    // up front so the lowercase form is a clean parse invariant.
    if hex.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(KeyringError::Invalid(
            "service".to_string(),
            "wallet id hex must be lowercase".to_string(),
        ));
    }
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(hex, &mut bytes).map_err(|_| {
        KeyringError::Invalid(
            "service".to_string(),
            "wallet id hex is not valid hex".to_string(),
        )
    })?;
    Ok(WalletId::from(bytes))
}

/// A `(wallet_id, label)` row in an [`EncryptedFileStore`].
///
/// All four operations re-validate `user` (label) and re-derive the
/// per-vault key (so a wrong passphrase fails closed at every call) —
/// defence in depth; the credential is long-lived and the cached
/// fields are reachable through `get_specifiers`.
pub struct EncryptedFileCredential {
    store: Arc<EncryptedFileStoreInner>,
    wallet_id: WalletId,
    label: String,
}

impl std::fmt::Debug for EncryptedFileCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptedFileCredential")
            .field("wallet_id", &self.wallet_id.to_hex())
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl CredentialApi for EncryptedFileCredential {
    fn set_secret(&self, secret: &[u8]) -> KeyringResult<()> {
        // Re-validate at every op (defence in depth, M-2 / SEC-REQ-4.3).
        let _ = validated_label(&self.label).map_err(FileStoreError::from)?;
        self.store
            .put(&self.wallet_id, &self.label, secret)
            .map_err(KeyringError::from)
    }

    fn get_secret(&self) -> KeyringResult<Vec<u8>> {
        let _ = validated_label(&self.label).map_err(FileStoreError::from)?;
        match self.store.get(&self.wallet_id, &self.label) {
            Ok(Some(v)) => Ok(v),
            Ok(None) => Err(KeyringError::NoEntry),
            Err(e) => Err(e.into()),
        }
    }

    fn delete_credential(&self) -> KeyringResult<()> {
        let _ = validated_label(&self.label).map_err(FileStoreError::from)?;
        match self.store.delete(&self.wallet_id, &self.label) {
            Ok(true) => Ok(()),
            Ok(false) => Err(KeyringError::NoEntry),
            Err(e) => Err(e.into()),
        }
    }

    fn get_credential(&self) -> KeyringResult<Option<Arc<Credential>>> {
        // Every entry is already a specifier — no wrapper layer.
        Ok(None)
    }

    fn get_specifiers(&self) -> Option<(String, String)> {
        Some((
            format!("{SERVICE_PREFIX}{}", self.wallet_id.to_hex()),
            self.label.clone(),
        ))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl CredentialStoreApi for EncryptedFileStore {
    fn vendor(&self) -> String {
        VENDOR.to_string()
    }

    fn id(&self) -> String {
        STORE_ID.to_string()
    }

    fn build(
        &self,
        service: &str,
        user: &str,
        _modifiers: Option<&HashMap<&str, &str>>,
    ) -> KeyringResult<Entry> {
        let wallet_id = parse_service(service)?;
        let label = validated_label(user)
            .map_err(FileStoreError::from)?
            .to_string();
        let cred = EncryptedFileCredential {
            store: self.inner.clone(),
            wallet_id,
            label,
        };
        Ok(Entry::new_with_credential(Arc::new(cred)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn persistence(&self) -> CredentialPersistence {
        CredentialPersistence::UntilDelete
    }
}

impl std::fmt::Debug for EncryptedFileStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptedFileStore")
            .field("dir", &self.inner.dir)
            .finish_non_exhaustive()
    }
}

#[cfg(unix)]
fn check_perms(meta: &fs::Metadata) -> Result<(), FileStoreError> {
    use std::os::unix::fs::MetadataExt;
    let mode = meta.mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(FileStoreError::InsecurePermissions { mode });
    }
    Ok(())
}

// TODO: Windows ACL read-check is not yet implemented; tracked in PR #3672.
#[cfg(not(unix))]
fn check_perms(_meta: &fs::Metadata) -> Result<(), FileStoreError> {
    Ok(())
}

#[cfg(unix)]
fn set_restrictive_perms(f: &fs::File) -> Result<(), FileStoreError> {
    use std::os::unix::fs::PermissionsExt;
    f.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_restrictive_perms(_f: &fs::File) -> Result<(), FileStoreError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &Path) -> EncryptedFileStore {
        EncryptedFileStore::open(dir, SecretString::new("pw-correct")).unwrap()
    }

    fn wid(b: u8) -> WalletId {
        WalletId::from([b; 32])
    }

    fn entry(s: &EncryptedFileStore, w: WalletId, label: &str) -> Entry {
        let service = format!("{SERVICE_PREFIX}{}", w.to_hex());
        s.build(&service, label, None).expect("build")
    }

    /// Whether a projected SPI error came from a wrong passphrase.
    /// `WrongPassphrase` rides in `NoStorageAccess` with the typed
    /// `FileStoreError` boxed as the source, recoverable losslessly.
    fn is_wrong_passphrase(e: &KeyringError) -> bool {
        matches!(
            e,
            KeyringError::NoStorageAccess(src)
                if matches!(src.downcast_ref::<FileStoreError>(), Some(FileStoreError::WrongPassphrase))
        )
    }

    /// Whether a projected SPI error is the lossy `Corruption`
    /// projection. `Corruption` collapses into `BadStoreFormat` with the
    /// variant's static `Display` text.
    fn is_corruption(e: &KeyringError) -> bool {
        matches!(e, KeyringError::BadStoreFormat(s) if *s == FileStoreError::Corruption.to_string())
    }

    #[test]
    fn roundtrip_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        {
            let s = store(dir.path());
            entry(&s, wid(1), "bip39_mnemonic")
                .set_secret(b"abandon abandon")
                .unwrap();
        }
        let s2 = store(dir.path());
        let got = entry(&s2, wid(1), "bip39_mnemonic").get_secret().unwrap();
        assert_eq!(got, b"abandon abandon");
        let missing = entry(&s2, wid(1), "missing").get_secret().unwrap_err();
        assert!(matches!(missing, KeyringError::NoEntry));
    }

    #[test]
    fn wrong_passphrase_fails_no_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        entry(&store(dir.path()), wid(1), "seed")
            .set_secret(b"super secret")
            .unwrap();
        let bad = EncryptedFileStore::open(dir.path(), SecretString::new("pw-wrong")).unwrap();
        let err = entry(&bad, wid(1), "seed").get_secret().unwrap_err();
        assert!(is_wrong_passphrase(&err), "unexpected error: {err:?}");
        // The error renders without any plaintext.
        assert!(!format!("{err}").contains("super secret"));
    }

    #[test]
    fn delete_returns_no_entry_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        // No vault file at all → NoEntry per D3.
        assert!(matches!(
            entry(&s, wid(1), "seed").delete_credential(),
            Err(KeyringError::NoEntry)
        ));
        entry(&s, wid(1), "seed").set_secret(b"v1").unwrap();
        entry(&s, wid(1), "seed").set_secret(b"v2").unwrap();
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"v2");
        entry(&s, wid(1), "seed").delete_credential().unwrap();
        // Second delete on the now-absent entry: NoEntry per D3.
        assert!(matches!(
            entry(&s, wid(1), "seed").delete_credential(),
            Err(KeyringError::NoEntry)
        ));
        assert!(matches!(
            entry(&s, wid(1), "seed").get_secret(),
            Err(KeyringError::NoEntry)
        ));
    }

    #[test]
    fn blob_swap_across_label_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        entry(&s, wid(1), "labelA").set_secret(b"secretA").unwrap();
        entry(&s, wid(1), "labelB").set_secret(b"secretB").unwrap();
        let path = s.test_vault_path(&wid(1));
        let (header, mut entries) = s.test_read_vault(&path).unwrap().unwrap();
        let a = entries
            .iter()
            .find(|e| e.label == "labelA")
            .unwrap()
            .clone();
        for e in entries.iter_mut() {
            if e.label == "labelB" {
                e.nonce = a.nonce;
                e.ciphertext = a.ciphertext.clone();
            }
        }
        s.test_write_vault(&path, &header, &entries).unwrap();
        let err = entry(&s, wid(1), "labelB").get_secret().unwrap_err();
        // The header verify-token passes (correct passphrase), so the
        // cross-label ciphertext swap surfaces as entry corruption, not
        // a wrong passphrase.
        assert!(is_corruption(&err), "unexpected error: {err:?}");
    }

    #[cfg(unix)]
    #[test]
    fn vault_created_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        entry(&s, wid(1), "seed").set_secret(b"x").unwrap();
        let mode = fs::metadata(s.test_vault_path(&wid(1)))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn loose_perms_preexisting_file_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        entry(&s, wid(1), "seed").set_secret(b"x").unwrap();
        let path = s.test_vault_path(&wid(1));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let err = entry(&s, wid(1), "seed").get_secret().unwrap_err();
        match &err {
            KeyringError::BadStoreFormat(s) => assert_eq!(
                *s,
                FileStoreError::InsecurePermissions { mode: 0o644 }.to_string()
            ),
            other => panic!("expected BadStoreFormat, got {other:?}"),
        }
    }

    #[test]
    fn rekey_reencrypts_and_old_passphrase_fails() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        let old_bytes = fs::read(s.test_vault_path(&wid(1))).unwrap();
        s.rekey(wid(1), SecretString::new("pw-new")).unwrap();
        // New passphrase reads; ciphertext changed; no .bak left.
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"value");
        let new_bytes = fs::read(s.test_vault_path(&wid(1))).unwrap();
        assert_ne!(old_bytes, new_bytes);
        let stale: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.ends_with(".bak") || n.contains(".tmp")
            })
            .collect();
        assert!(stale.is_empty(), "rekey left stale files: {stale:?}");
        let old = EncryptedFileStore::open(dir.path(), SecretString::new("pw-correct")).unwrap();
        let err = entry(&old, wid(1), "seed").get_secret().unwrap_err();
        assert!(is_wrong_passphrase(&err), "unexpected error: {err:?}");
    }

    #[test]
    fn rekey_with_outstanding_credential_returns_busy_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        // `build()` clones the inner `Arc`; keeping the credential alive
        // means the store no longer holds an exclusive reference.
        let live = entry(&s, wid(1), "seed");
        live.set_secret(b"value").unwrap();
        let err = s.rekey(wid(1), SecretString::new("pw-new")).unwrap_err();
        assert!(matches!(err, FileStoreError::Busy));
        // The credential is still usable and the passphrase unchanged.
        assert_eq!(live.get_secret().unwrap(), b"value");
        // Once the outstanding credential is dropped, rekey succeeds.
        drop(live);
        s.rekey(wid(1), SecretString::new("pw-new")).unwrap();
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"value");
    }

    #[test]
    fn put_with_wrong_passphrase_to_existing_vault_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        entry(&store(dir.path()), wid(1), "seed")
            .set_secret(b"orig")
            .unwrap();
        let wrong = EncryptedFileStore::open(dir.path(), SecretString::new("pw-wrong")).unwrap();
        // A wrong passphrase must be rejected before any mixed-key entry
        // is written.
        let err = entry(&wrong, wid(1), "seed2")
            .set_secret(b"intruder")
            .unwrap_err();
        assert!(is_wrong_passphrase(&err), "unexpected error: {err:?}");
        // Original vault still fully readable with the correct pass.
        let ok = store(dir.path());
        assert_eq!(entry(&ok, wid(1), "seed").get_secret().unwrap(), b"orig");
        // The rejected slot was never written.
        assert!(matches!(
            entry(&ok, wid(1), "seed2").get_secret(),
            Err(KeyringError::NoEntry)
        ));
    }

    #[test]
    fn get_and_delete_with_wrong_passphrase_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        entry(&store(dir.path()), wid(1), "seed")
            .set_secret(b"orig")
            .unwrap();
        let wrong = EncryptedFileStore::open(dir.path(), SecretString::new("pw-wrong")).unwrap();
        let get_err = entry(&wrong, wid(1), "seed").get_secret().unwrap_err();
        assert!(
            is_wrong_passphrase(&get_err),
            "unexpected error: {get_err:?}"
        );
        let del_err = entry(&wrong, wid(1), "seed")
            .delete_credential()
            .unwrap_err();
        assert!(
            is_wrong_passphrase(&del_err),
            "unexpected error: {del_err:?}"
        );
        // delete must not have mutated the vault.
        let ok = store(dir.path());
        assert_eq!(entry(&ok, wid(1), "seed").get_secret().unwrap(), b"orig");
    }

    #[test]
    fn get_corruption_after_verify_token_is_not_wrong_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        // Unlock works with the correct passphrase.
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"value");
        // Bit-flip the entry ciphertext on disk; the header verify-token
        // is untouched, so the passphrase is still correct.
        let path = s.test_vault_path(&wid(1));
        let (header, mut entries) = s.test_read_vault(&path).unwrap().unwrap();
        entries[0].ciphertext[0] ^= 0x01;
        s.test_write_vault(&path, &header, &entries).unwrap();
        let err = entry(&s, wid(1), "seed").get_secret().unwrap_err();
        assert!(is_corruption(&err), "unexpected error: {err:?}");
        assert!(
            !is_wrong_passphrase(&err),
            "must not be WrongPassphrase: {err:?}"
        );
    }

    #[test]
    fn rekey_corruption_on_existing_entry_is_not_wrong_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        // Corrupt the entry ciphertext but leave the verify-token intact.
        let path = s.test_vault_path(&wid(1));
        let (header, mut entries) = s.test_read_vault(&path).unwrap().unwrap();
        entries[0].ciphertext[0] ^= 0x01;
        s.test_write_vault(&path, &header, &entries).unwrap();
        // Rekey with the *correct* old passphrase: header verify passes,
        // the entry re-encrypt fails with Corruption, not WrongPassphrase
        // nor Busy.
        let err = s.rekey(wid(1), SecretString::new("pw-new")).unwrap_err();
        assert!(
            matches!(err, FileStoreError::Corruption),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn correct_passphrase_round_trips_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        entry(&s, wid(1), "seed").set_secret(b"orig").unwrap();
        entry(&s, wid(1), "seed2").set_secret(b"second").unwrap();
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"orig");
        assert_eq!(entry(&s, wid(1), "seed2").get_secret().unwrap(), b"second");
    }

    #[test]
    fn no_plaintext_in_vault_file() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        entry(&s, wid(1), "seed")
            .set_secret(b"PLAINTEXTNEEDLE")
            .unwrap();
        let raw = fs::read(s.test_vault_path(&wid(1))).unwrap();
        assert!(
            raw.windows(b"PLAINTEXTNEEDLE".len())
                .all(|w| w != b"PLAINTEXTNEEDLE"),
            "plaintext leaked into vault file"
        );
    }

    #[test]
    fn build_rejects_malformed_service() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        for bad in [
            "no-prefix",
            "dash.platform-wallet-storage/short",
            // wrong prefix
            "wrong-app/0000000000000000000000000000000000000000000000000000000000000000",
            // non-hex in expected slot
            "dash.platform-wallet-storage/zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        ] {
            let err = s.build(bad, "seed", None).unwrap_err();
            match err {
                KeyringError::Invalid(attr, _) => assert_eq!(attr, "service"),
                other => panic!("expected Invalid(\"service\"), got {other:?}"),
            }
        }
    }

    #[test]
    fn build_rejects_uppercase_hex_service() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        // 64-char, valid-hex, but uppercase: must be rejected before decode
        // so lowercase stays a clean parse invariant.
        let upper = format!("{SERVICE_PREFIX}{}", "A".repeat(64));
        let err = s.build(&upper, "seed", None).unwrap_err();
        match err {
            KeyringError::Invalid(attr, _) => assert_eq!(attr, "service"),
            other => panic!("expected Invalid(\"service\"), got {other:?}"),
        }
        // The lowercase form of the same bytes is accepted.
        let lower = format!("{SERVICE_PREFIX}{}", "aa".repeat(32));
        s.build(&lower, "seed", None)
            .expect("lowercase hex accepted");
    }

    #[test]
    fn build_rejects_invalid_label() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        let service = format!("{SERVICE_PREFIX}{}", wid(1).to_hex());
        for bad in ["../escape", "", "lab el", "a:b"] {
            let err = s.build(&service, bad, None).unwrap_err();
            match err {
                KeyringError::Invalid(attr, _) => assert_eq!(attr, "user"),
                other => panic!("expected Invalid(\"user\"), got {other:?}"),
            }
        }
    }

    #[test]
    fn get_specifiers_round_trip_the_pair() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        let e = entry(&s, wid(1), "seed");
        let (service, user) = e.get_specifiers().unwrap();
        assert_eq!(service, format!("{SERVICE_PREFIX}{}", wid(1).to_hex()));
        assert_eq!(user, "seed");
    }

    #[test]
    fn second_write_over_existing_vault_succeeds() {
        // `persist` replaces atomically on every target, so a second write
        // over an existing vault succeeds cross-platform.
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        entry(&s, wid(1), "seed").set_secret(b"v1").unwrap();
        entry(&s, wid(1), "seed").set_secret(b"v2").unwrap();
        entry(&s, wid(1), "other").set_secret(b"v3").unwrap();
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"v2");
        assert_eq!(entry(&s, wid(1), "other").get_secret().unwrap(), b"v3");
        // No staged temp survives a successful persist.
        let stale: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.ends_with(".bak") || n.contains(".tmp")
            })
            .collect();
        assert!(stale.is_empty(), "left stale files: {stale:?}");
    }

    #[test]
    fn inflated_kdf_params_fail_before_verify_token_derivation() {
        // A vault whose JSON declares m_kib = u32::MAX must be refused with
        // a KDF failure (projected to BadStoreFormat) at `derive_and_verify`
        // — before the verify-token is derived and without the ~4 TiB
        // allocation the inflated param would demand.
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        // Rewrite the on-disk vault's KDF m_kib to u32::MAX via the
        // header round-trip the test surface exposes.
        let path = s.test_vault_path(&wid(1));
        let (mut header, entries) = s.test_read_vault(&path).unwrap().unwrap();
        header.params.m_kib = u32::MAX;
        s.test_write_vault(&path, &header, &entries).unwrap();
        let err = entry(&s, wid(1), "seed").get_secret().unwrap_err();
        assert!(
            matches!(&err, KeyringError::BadStoreFormat(msg) if *msg == FileStoreError::KdfFailure.to_string()),
            "expected KdfFailure projection, got {err:?}"
        );
    }

    #[test]
    fn persistence_is_until_delete() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        assert!(matches!(
            s.persistence(),
            CredentialPersistence::UntilDelete
        ));
        assert_eq!(s.vendor(), VENDOR);
        assert_eq!(s.id(), STORE_ID);
    }
}
