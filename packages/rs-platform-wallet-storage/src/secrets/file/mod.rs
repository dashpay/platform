//! [`EncryptedFileStore`] — passphrase-encrypted on-disk vault, resident
//! in memory while the store handle lives.
//!
//! [`open`] takes the advisory lock on a sibling `.lock` sidecar (single
//! attempt), then creates or decrypts the vault and keeps the plaintext
//! entry map resident. [`get`] serves from memory (no per-op KDF/disk);
//! every mutation ([`put`], [`delete`], [`rekey`]) edits memory then
//! re-encrypts and atomically rewrites the file. [`Drop`] best-effort
//! re-syncs, re-asserts `0600` on Unix, and releases the lock. A second
//! `open()` of a held path fails fast with
//! [`SecretStoreError::AlreadyLocked`].
//!
//! **Cross-process exclusion is LOCAL-filesystem only.** `AlreadyLocked`
//! rests on `fd-lock` (`flock` / `LockFileEx`), which does NOT interlock
//! over NFS/CIFS/SMB — two hosts could each "lock" and last-writer-wins
//! would lose secrets. A vault file MUST NOT be shared across hosts; use
//! the OS keyring ([`SecretStore::Os`]) for multi-host access. The lock
//! sidecar is distinct from the vault file so the atomic `persist` rename
//! never touches the inode the open lock fd points at.
//!
//! [`SecretStore::Os`]: crate::secrets::SecretStore::Os
//!
//! [`open`]: EncryptedFileStore::open
//! [`put`]: EncryptedFileStore::put_bytes
//! [`delete`]: EncryptedFileStore::delete_bytes
//! [`rekey`]: EncryptedFileStore::rekey
//! [`get`]: EncryptedFileStore::get_bytes
//!
//! Threat coverage: the at-rest file is Argon2id + AEAD, so it protects
//! **A1** (other local user), **A4** (lost laptop / cold backup), and
//! **A6** (synced backup). It does NOT cover **A3** (key/passphrase
//! resident while unlocked), a weak operator passphrase, or **A5**
//! (swap / core-dump while unlocked) — the last is best-effort mitigated
//! by zeroize + mlock. The derived AEAD key stays resident in a
//! [`SecretBytes`] (to avoid per-op Argon2) and is zeroized on Drop.

// `pub(super)` (= visible within `crate::secrets`) so the Tier-2
// `envelope` module — a sibling of `file` under `secrets` — can reuse the
// shared Argon2id/XChaCha primitives and `KDF_ID_ARGON2ID` without
// duplicating crypto. Items inside stay `pub(crate)`/`pub(in …file)`, so
// nothing escapes the secrets tree (see the crypto.rs module doc).
pub(super) mod crypto;
pub(super) mod format;

use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use keyring_core::api::{Credential, CredentialApi, CredentialPersistence, CredentialStoreApi};
use keyring_core::{Entry, Error as KeyringError, Result as KeyringResult};

use crypto::{KdfParams, SALT_LEN};
use format::{EntryBody, Vault};

use super::error::SecretStoreError;

use super::secret::{SecretBytes, SecretString, MIN_PASSPHRASE_LEN};
use super::validate::{validated_label, WalletId};

/// Service-prefix for vault entries: the full `service` string is
/// `SERVICE_PREFIX + hex(wallet_id)`, one namespace per wallet.
pub const SERVICE_PREFIX: &str = "dash.platform-wallet-storage/";

/// Vendor / id tags published through `CredentialStoreApi`.
const VENDOR: &str = "dash.platform-wallet-storage";
const STORE_ID: &str = "encrypted-file-store-v1";

/// Structural ceiling on the on-disk vault file. The vault is
/// attacker-controllable JSON, so refuse to allocate / parse beyond this
/// cap (surfacing [`SecretStoreError::VaultTooLarge`]) ahead of any tag
/// check rather than let a multi-GiB file force a huge `fs::read`.
pub const MAX_VAULT_SIZE_BYTES: u64 = 128 * 1024 * 1024;

/// Per-secret write-side ceiling. The vault is ONE shared document, so an
/// uncapped oversized entry would inflate it past [`MAX_VAULT_SIZE_BYTES`]
/// and brick every wallet on reopen. 64 KiB is far above any legitimate
/// mnemonic / seed / xpriv. Enforced with
/// [`SecretStoreError::SecretTooLarge`] at the write boundary before the
/// secret is sealed or inserted.
pub const MAX_SECRET_LEN: usize = 64 * 1024;

/// A passphrase-encrypted file-backed credential store.
///
/// One file, one passphrase, one lock; the whole store rotates together
/// via [`rekey`](Self::rekey). The cheap-`Clone` handle shares one
/// `Arc<Mutex<…>>`, so every clone sees the same resident state and
/// serializes against every other operation. All [`SecretString`]s and
/// the resident AEAD key are zeroized on drop.
#[derive(Clone)]
pub struct EncryptedFileStore {
    inner: Arc<Mutex<EncryptedFileStoreInner>>,
    /// Shared clone of [`EncryptedFileStoreInner::durability_uncertain`] so
    /// the pollable accessor reads it WITHOUT taking the state lock.
    durability_uncertain: Arc<AtomicU64>,
    /// Argon2 params this store DERIVES at: the header written by a fresh
    /// vault or a [`rekey`](Self::rekey), and the per-secret Tier-2 wrap
    /// (via [`kdf_params`](Self::kdf_params)). Always
    /// [`KdfParams::default_target`] except on a mock store
    /// ([`open_mock`](Self::open_mock)), which floors it (#4111).
    ///
    /// Read-only after open and `Copy`, so `rekey` reads it without taking
    /// the state lock — preserving its derive-outside-the-lock property.
    /// It does NOT describe an already-existing vault: unlocking one always
    /// re-derives under the params in ITS header.
    kdf: KdfParams,
}

/// Resident store state behind a single [`Mutex`] so every mutation sees
/// a consistent triple (vault matches the key it was sealed under, key
/// matches the passphrase that derived it). The single lock serializes
/// put/get/delete/rekey so a put cannot seal under an old key mid-rekey;
/// the disk write happens under it, and the file-lock sidecar already
/// serializes cross-process, so this adds no new I/O contention point.
struct EncryptedFileStoreInner {
    /// Vault file path supplied by the caller at [`open`].
    ///
    /// [`open`]: EncryptedFileStore::open
    path: PathBuf,
    /// In-memory vault. Mutations edit it then `sync_to_disk` to
    /// re-encrypt and atomically replace the file; reads clone from here.
    vault: Vault,
    /// AEAD key derived once at [`open`] (re-derived only on [`rekey`]).
    /// Held resident to avoid per-op Argon2 — A3 (key resident while
    /// unlocked) is an accepted threat; zeroized when the state drops.
    ///
    /// [`open`]: EncryptedFileStore::open
    /// [`rekey`]: EncryptedFileStore::rekey
    derived_key: SecretBytes,
    /// Store-wide passphrase, swapped with `vault` + `derived_key` under
    /// the same lock during [`rekey`].
    ///
    /// [`rekey`]: EncryptedFileStore::rekey
    passphrase: SecretString,
    /// Count of vault writes whose data committed (atomic rename succeeded)
    /// but whose parent-directory fsync could NOT be confirmed, so the
    /// rename's durability across power loss is uncertain. Non-fatal by
    /// design — the write still returns `Ok` and the data is visible — this
    /// is a pollable signal, not a rollback trigger (see
    /// [`EncryptedFileStore::durability_uncertain_count`]). Bumped under the
    /// state lock from `sync_to_disk`; read lock-free via the shared `Arc`.
    durability_uncertain: Arc<AtomicU64>,
    /// Holds the advisory write-lock on `<path>.lock` for the store's
    /// lifetime; dropped (releasing the lock) when the store drops.
    _lock: VaultLock,
}

impl EncryptedFileStore {
    /// Open a vault store at `path` (the vault FILE, not a directory),
    /// unlocked by `passphrase`.
    ///
    /// Acquires an exclusive advisory lock on a sibling `<path>.lock`
    /// first; if already held, returns [`SecretStoreError::AlreadyLocked`]
    /// immediately (no retry). A missing `path` is created fresh (random
    /// salt, default Argon2 params, sealed verify token) at `0600` on
    /// Unix; an existing one is read and its passphrase verified against
    /// the header verify-token. Either way the returned store is usable.
    pub fn open(
        path: impl AsRef<Path>,
        passphrase: SecretString,
    ) -> Result<Self, SecretStoreError> {
        // Tier-1 baseline: reject a blank passphrase (empty / all-whitespace)
        // BEFORE touching the filesystem. A blank passphrase derives a key
        // from a public salt only — obfuscation, not confidentiality
        // (obfuscation, not confidentiality). This is an INTENDED behavioural break for any caller
        // that relied on `SecretString::empty()`; a deliberate keyless vault
        // must use [`open_unprotected`](Self::open_unprotected). No vault
        // file is created or altered for a blank passphrase.
        reject_weak_passphrase(&passphrase)?;
        Self::open_inner(path.as_ref(), passphrase, KdfParams::default_target())
    }

    /// [`open`](Self::open), but every Argon2id derivation this store
    /// performs runs at the enforced FLOOR instead of the shipped 64 MiB
    /// target — the fastest configuration the crate's own bounds check
    /// still accepts. Covers both the vault-unlock derivation here and the
    /// per-secret Tier-2 wrap inside
    /// [`SecretStore::set_secret`](crate::secrets::SecretStore::set_secret) /
    /// [`reprotect`](crate::secrets::SecretStore::reprotect).
    ///
    /// **Test-only.** A downstream suite that drives real end-to-end
    /// `SecretStore` flows otherwise pays a production-strength KDF per call
    /// (#4111). The resulting vault is REAL — same formats, same AAD
    /// binding, same fail-closed reads — merely cheap to attack. Never build
    /// one over a vault that holds live secrets.
    ///
    /// Two independent gates keep it out of production:
    /// 1. compile-time — the `test-util` feature (or `cfg(test)`);
    /// 2. runtime — the panic inherited from `KdfParams::floor_target`, in
    ///    case feature unification switches `test-util` on for a release build.
    ///
    /// # Panics
    ///
    /// Panics unless the build has `debug_assertions` on, or is this crate's
    /// own test harness — the guard lives on `KdfParams::floor_target`, the
    /// single choke point for weak-but-legal params, and fires before this
    /// constructor touches the filesystem. `cargo test --release` on a
    /// DOWNSTREAM crate is indistinguishable from a leaked release build and
    /// panics too; such a suite must keep `debug-assertions = true`.
    ///
    /// An EXISTING vault is still unlocked under the params in its own header
    /// — a mock store cannot make a production vault cheap to open.
    #[cfg(any(test, feature = "test-util"))]
    pub fn open_mock(
        path: impl AsRef<Path>,
        passphrase: SecretString,
    ) -> Result<Self, SecretStoreError> {
        // Ahead of every other check, so a rejected passphrase cannot mask the
        // release-build panic behind a plain `Err`.
        let kdf = KdfParams::floor_target();
        reject_weak_passphrase(&passphrase)?;
        Self::open_inner(path.as_ref(), passphrase, kdf)
    }

    /// Open (or create) a **deliberately keyless** vault — the only door
    /// that accepts no passphrase. The vault key is derived from an empty
    /// passphrase under the public salt, so this is **obfuscation, not
    /// confidentiality**: use it only where the stored secrets carry their
    /// own Tier-2 object password, or as a staging step before
    /// [`rekey`](Self::rekey) to a real passphrase. This is the explicit
    /// keyless door, distinct from [`open`](Self::open) (which now rejects a
    /// blank passphrase).
    pub fn open_unprotected(path: impl AsRef<Path>) -> Result<Self, SecretStoreError> {
        Self::open_inner(
            path.as_ref(),
            SecretString::empty(),
            KdfParams::default_target(),
        )
    }

    /// Shared open/create core for [`open`](Self::open),
    /// [`open_unprotected`](Self::open_unprotected) and
    /// [`open_mock`](Self::open_mock). Does NOT apply the blank-passphrase
    /// guard — the public doors decide that. `kdf` is the params a FRESH
    /// vault is created under; an existing one keeps its own header.
    fn open_inner(
        path: &Path,
        passphrase: SecretString,
        kdf: KdfParams,
    ) -> Result<Self, SecretStoreError> {
        let path = path.to_path_buf();

        // Materialize the parent so the lock-sidecar open and vault
        // create do not fail on a not-yet-existing dir.
        let parent = normalized_parent(&path);
        create_parent_dir(parent)?;
        // Refuse a group/other-WRITABLE parent: directory write governs
        // rename/unlink, so a writable parent lets another local user
        // replace the vault despite its own 0600 (the A1 guarantee).
        check_parent_perms(parent)?;

        // Lock first — every subsequent step assumes exclusive ownership.
        let lock = VaultLock::acquire(&lock_path_for(&path))?;

        // NotFound → create fresh; anything else → load.
        let (vault, derived_key) = match Self::load_existing_vault(&path, &passphrase)? {
            Some(loaded) => loaded,
            None => Self::create_new_vault(&path, &passphrase, kdf)?,
        };

        let durability_uncertain = Arc::new(AtomicU64::new(0));
        Ok(Self {
            inner: Arc::new(Mutex::new(EncryptedFileStoreInner {
                path,
                vault,
                derived_key,
                passphrase,
                durability_uncertain: Arc::clone(&durability_uncertain),
                _lock: lock,
            })),
            durability_uncertain,
            kdf,
        })
    }

    /// The Argon2 params this store derives at — see [`kdf`](Self::kdf).
    /// The Tier-2 wrap in [`SecretStore`](crate::secrets::SecretStore) reads
    /// it so a mock store's per-secret seals are floored too, with no new
    /// parameter on any public method.
    pub(crate) fn kdf_params(&self) -> KdfParams {
        self.kdf
    }

    /// Number of vault writes whose data committed but whose parent-directory
    /// fsync could not be confirmed (rename durability across power loss is
    /// uncertain). Monotonic, process-lifetime; `0` means every write this
    /// store performed was confirmed durable. This is the **observable signal**
    /// behind the intentionally non-fatal handling: such a write still returns
    /// `Ok` (data committed + visible), so a caller that cares about hard
    /// durability polls this rather than seeing a spurious error it would
    /// otherwise roll back. Read lock-free.
    pub fn durability_uncertain_count(&self) -> u64 {
        self.durability_uncertain.load(Ordering::Relaxed)
    }

    /// Load and decrypt an existing vault file, returning `Ok(None)` if
    /// the file does not exist. Verifies the passphrase against the
    /// header verify-token before returning.
    fn load_existing_vault(
        path: &Path,
        passphrase: &SecretString,
    ) -> Result<Option<(Vault, SecretBytes)>, SecretStoreError> {
        let Some(vault) = read_vault_at(path)? else {
            return Ok(None);
        };
        let key = derive_and_verify(&vault, passphrase)?;
        Ok(Some((vault, key)))
    }

    /// Build a brand-new empty vault under `kdf`, persist it at `0600`, and
    /// return the in-memory state + derived key.
    fn create_new_vault(
        path: &Path,
        passphrase: &SecretString,
        kdf: KdfParams,
    ) -> Result<(Vault, SecretBytes), SecretStoreError> {
        let (vault, key) = build_fresh_vault(passphrase, kdf)?;
        // Initial create: the store isn't built yet, so no durability counter
        // to bump — a brand-new store's count starts at 0.
        write_vault_at(path, &vault, None)?;
        Ok((vault, key))
    }

    /// Re-encrypt the whole store under `new_passphrase` — fresh salt and
    /// per-entry nonces for every wallet, atomically in one shot (no
    /// half-rotated state, no `.bak` retaining old key material). Vault,
    /// derived key, and passphrase advance together under the mutex.
    ///
    /// The Argon2 derivation runs OUTSIDE the lock — it touches only the
    /// new passphrase + fresh salt, so paying ~hundreds of ms inside the
    /// critical section would needlessly stall unrelated put/get ops.
    pub fn rekey(&self, new_passphrase: SecretString) -> Result<(), SecretStoreError> {
        // Reject a blank target passphrase: `rekey` always advances to a
        // REAL passphrase (the empty→real migration uses this). The resident
        // vault, key, and on-disk file are untouched on rejection. To make a
        // vault keyless, use `open_unprotected` on a fresh path instead.
        reject_weak_passphrase(&new_passphrase)?;
        let (new_vault, new_key) = build_fresh_vault(&new_passphrase, self.kdf)?;
        lock_inner(&self.inner).rekey(new_vault, new_key, new_passphrase)
    }

    /// Store `secret` under `(wallet_id, label)`, returning the typed
    /// [`SecretStoreError`] losslessly (no SPI seam). The secret stays
    /// wrapped across this boundary; the bare-buffer exposure is one layer
    /// down at the AEAD seal.
    pub(crate) fn put_bytes(
        &self,
        wallet_id: &WalletId,
        label: &str,
        secret: &SecretBytes,
    ) -> Result<(), SecretStoreError> {
        lock_inner(&self.inner).put(wallet_id, label, secret)
    }

    /// Retrieve the plaintext under `(wallet_id, label)`, or `None` if
    /// absent. The plaintext stays inside a zeroizing [`SecretBytes`] to
    /// this boundary; the bare-`Vec<u8>` conversion lives only at the
    /// `CredentialApi::get_secret` SPI seam, where the contract demands it.
    pub(crate) fn get_bytes(
        &self,
        wallet_id: &WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, SecretStoreError> {
        lock_inner(&self.inner).get(wallet_id, label)
    }

    /// Delete the entry under `(wallet_id, label)`; `Ok(false)` if it was
    /// already absent. Returns the typed [`SecretStoreError`].
    pub(crate) fn delete_bytes(
        &self,
        wallet_id: &WalletId,
        label: &str,
    ) -> Result<bool, SecretStoreError> {
        lock_inner(&self.inner).delete(wallet_id, label)
    }

    /// Atomic read-modify-write of `(wallet_id, label)`. Holds the store lock
    /// across the read → `transform` → write so a concurrent `put`/`delete`
    /// can't interleave and let a transform built on stale bytes clobber a
    /// newer value. `transform` receives the currently-stored bytes (`None`
    /// if absent) and returns the bytes to persist.
    pub(crate) fn reprotect_bytes<F>(
        &self,
        wallet_id: &WalletId,
        label: &str,
        transform: F,
    ) -> Result<(), SecretStoreError>
    where
        F: FnOnce(Option<SecretBytes>) -> Result<SecretBytes, SecretStoreError>,
    {
        let mut inner = lock_inner(&self.inner);
        let current = inner.get(wallet_id, label)?;
        let next = transform(current)?;
        inner.put(wallet_id, label, &next)
    }

    #[cfg(test)]
    pub(crate) fn test_read_vault_from_disk(&self) -> Result<Option<Vault>, SecretStoreError> {
        read_vault_at(&lock_inner(&self.inner).path)
    }

    #[cfg(test)]
    pub(crate) fn test_write_vault_to_disk(&self, vault: &Vault) -> Result<(), SecretStoreError> {
        write_vault_at(&lock_inner(&self.inner).path, vault, None)
    }

    /// Reload the vault from disk under the current passphrase, so a test
    /// that patched the on-disk file sees the new bytes (the resident
    /// model otherwise serves the cached state).
    #[cfg(test)]
    pub(crate) fn test_reload_from_disk(&self) -> Result<(), SecretStoreError> {
        let mut state = lock_inner(&self.inner);
        let Some(vault) = read_vault_at(&state.path)? else {
            return Err(SecretStoreError::MalformedVault);
        };
        let key = derive_and_verify(&vault, &state.passphrase)?;
        state.vault = vault;
        state.derived_key = key;
        Ok(())
    }
}

/// Acquire the state lock on `inner`. A poisoned mutex is recovered (not
/// propagated): every mutation either commits wholesale or reverts, so a
/// panicked holder cannot have left the inner value half-written.
fn lock_inner(
    inner: &Arc<Mutex<EncryptedFileStoreInner>>,
) -> std::sync::MutexGuard<'_, EncryptedFileStoreInner> {
    inner.lock().unwrap_or_else(|p| p.into_inner())
}

impl EncryptedFileStoreInner {
    /// Re-encrypt the resident vault and atomically replace the
    /// on-disk file. Runs inside the state-lock critical section.
    fn sync_to_disk(&self) -> Result<(), SecretStoreError> {
        write_vault_at(&self.path, &self.vault, Some(&self.durability_uncertain))
    }

    /// In-place seal + disk-write for [`EncryptedFileStore::put_bytes`];
    /// rolls the in-memory entry back on a disk-write failure.
    fn put(
        &mut self,
        wallet_id: &WalletId,
        label: &str,
        secret: &SecretBytes,
    ) -> Result<(), SecretStoreError> {
        let label = validated_label(label)?.to_string();
        // Reject before sealing: the shared document would otherwise
        // inflate past the read-side ceiling and brick every wallet.
        if secret.len() > MAX_SECRET_LEN {
            return Err(SecretStoreError::SecretTooLarge {
                found: secret.len(),
                max: MAX_SECRET_LEN,
            });
        }
        let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), &label);

        let (nonce, ciphertext) = crypto::seal(&self.derived_key, &aad, secret.expose_secret())?;

        // Remember the prior body so a disk-write failure can revert the
        // resident state to match disk (Ok must imply memory == disk).
        let prior = {
            let entries = self.vault.wallets.entry(wallet_id.to_hex()).or_default();
            entries.insert(label.clone(), EntryBody { nonce, ciphertext })
        };

        if let Err(e) = self.sync_to_disk() {
            // A missing bucket means the insert never landed (nothing to
            // undo) — return the error rather than panic.
            if let Some(entries) = self.vault.wallets.get_mut(&wallet_id.to_hex()) {
                match prior {
                    Some(prev) => {
                        entries.insert(label, prev);
                    }
                    None => {
                        entries.remove(&label);
                        if entries.is_empty() {
                            self.vault.wallets.remove(&wallet_id.to_hex());
                        }
                    }
                }
            }
            return Err(e);
        }
        Ok(())
    }

    /// Resident-vault lookup for [`EncryptedFileStore::get_bytes`];
    /// absence is `Ok(None)`, never an error.
    fn get(
        &self,
        wallet_id: &WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, SecretStoreError> {
        let label = validated_label(label)?;
        let wallet_hex = wallet_id.to_hex();
        let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), label);

        let Some(entries) = self.vault.wallets.get(&wallet_hex) else {
            return Ok(None);
        };
        let Some(body) = entries.get(label) else {
            return Ok(None);
        };
        entry_decrypt_or_corruption(
            &wallet_hex,
            label,
            crypto::open(&self.derived_key, &body.nonce, &aad, &body.ciphertext),
        )
        .map(Some)
    }

    /// Removal for [`EncryptedFileStore::delete_bytes`]; returns
    /// `Ok(false)` if absent, rolls the in-memory entry back on a
    /// disk-write failure.
    fn delete(&mut self, wallet_id: &WalletId, label: &str) -> Result<bool, SecretStoreError> {
        let label = validated_label(label)?;
        let wallet_hex = wallet_id.to_hex();

        let removed = {
            let Some(entries) = self.vault.wallets.get_mut(&wallet_hex) else {
                return Ok(false);
            };
            let Some(prev) = entries.remove(label) else {
                return Ok(false);
            };
            let wallet_emptied = entries.is_empty();
            if wallet_emptied {
                self.vault.wallets.remove(&wallet_hex);
            }
            (prev, wallet_emptied)
        };

        if let Err(e) = self.sync_to_disk() {
            let entries = self.vault.wallets.entry(wallet_hex).or_default();
            entries.insert(label.to_string(), removed.0);
            return Err(e);
        }
        Ok(true)
    }

    /// Swap-in for [`EncryptedFileStore::rekey`]: re-seals every
    /// resident entry under `new_key` and atomically replaces the
    /// vault file, restoring the old triple on a disk-write failure.
    fn rekey(
        &mut self,
        mut new_vault: Vault,
        new_key: SecretBytes,
        new_passphrase: SecretString,
    ) -> Result<(), SecretStoreError> {
        for (wallet_hex, entries) in &self.vault.wallets {
            let wallet_bytes = decode_wallet_id_hex(wallet_hex)?;
            let mut new_entries: std::collections::BTreeMap<String, EntryBody> =
                std::collections::BTreeMap::new();
            for (label, body) in entries {
                let aad = format::aad(format::FORMAT_VERSION, &wallet_bytes, label);
                let pt = entry_decrypt_or_corruption(
                    wallet_hex,
                    label,
                    crypto::open(&self.derived_key, &body.nonce, &aad, &body.ciphertext),
                )?;
                let (nonce, ct) = crypto::seal(&new_key, &aad, pt.expose_secret())?;
                new_entries.insert(
                    label.clone(),
                    EntryBody {
                        nonce,
                        ciphertext: ct,
                    },
                );
            }
            new_vault.wallets.insert(wallet_hex.clone(), new_entries);
        }

        // Stage the new triple; on disk-write failure restore the old one
        // so the live handle keeps serving under the still-on-disk key.
        let old_vault = std::mem::replace(&mut self.vault, new_vault);
        let old_key = std::mem::replace(&mut self.derived_key, new_key);
        let old_pp = std::mem::replace(&mut self.passphrase, new_passphrase);

        if let Err(e) = self.sync_to_disk() {
            self.vault = old_vault;
            self.derived_key = old_key;
            self.passphrase = old_pp;
            return Err(e);
        }
        Ok(())
    }
}

impl Drop for EncryptedFileStoreInner {
    fn drop(&mut self) {
        // Best-effort final sync. Redundant in the success path (every
        // mutation eager-syncs) but kept as a contract anchor.
        if let Err(e) = self.sync_to_disk() {
            tracing::warn!(error = %e, "drop-time vault sync failed");
        }
        // Re-assert 0600 on Unix in case a peer loosened it while we held
        // the lock. Best-effort: failures are non-fatal at Drop.
        #[cfg(unix)]
        match open_no_follow(&self.path) {
            Ok(file) => {
                if let Err(e) = set_restrictive_perms(&file) {
                    tracing::warn!(error = %e, "drop-time perm re-assert failed");
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "drop-time perm re-assert skipped: vault re-open refused"
                );
            }
        }
        // `VaultLock` drops after this returns, releasing the OS lock.
    }
}

/// Sidecar lock path (`<path>.lock`). Distinct from the vault file so the
/// atomic `persist` swap never touches the inode the lock fd points at.
fn lock_path_for(path: &Path) -> PathBuf {
    let mut s = path.to_path_buf().into_os_string();
    s.push(".lock");
    PathBuf::from(s)
}

/// Reject a blank (empty / all-whitespace) or sub-floor passphrase →
/// [`SecretStoreError::BlankPassphrase`]. The floor is the coarse
/// [`MIN_PASSPHRASE_LEN`] (1 today = merely non-blank); the real entropy
/// policy is the consumer's (see `SECRETS.md`). A blank check alone closes
/// the length term keeps the floor wired for a future bump.
fn reject_weak_passphrase(passphrase: &SecretString) -> Result<(), SecretStoreError> {
    if passphrase.is_blank() || passphrase.trimmed().len() < MIN_PASSPHRASE_LEN {
        return Err(SecretStoreError::BlankPassphrase);
    }
    Ok(())
}

/// Build a fresh entry-less vault (random salt, `kdf` Argon2 params,
/// verify-token sealed under the derived key) plus that derived key, so
/// the caller can seal entries without re-deriving. `kdf` lands in the
/// header AND in the verify-token's AAD, so the params stay tamper-bound
/// whichever value the caller picked.
fn build_fresh_vault(
    passphrase: &SecretString,
    kdf: KdfParams,
) -> Result<(Vault, SecretBytes), SecretStoreError> {
    let mut salt = [0u8; SALT_LEN];
    crypto::random_bytes(&mut salt)?;
    let key = crypto::derive_key(passphrase, &salt, kdf)?;
    let v_aad = format::verify_aad(format::FORMAT_VERSION, &salt, &kdf);
    let (verify_nonce, verify_ct) = crypto::seal(&key, &v_aad, format::VERIFY_CONSTANT)?;
    Ok((
        Vault {
            version: format::FORMAT_VERSION,
            kdf,
            salt,
            verify_nonce,
            verify_ct,
            wallets: std::collections::BTreeMap::new(),
        },
        key,
    ))
}

/// Derive the key from `passphrase` and verify it against the vault's
/// token *before* any entry is touched. A wrong passphrase fails the
/// token's AEAD tag (constant-time) and yields `WrongPassphrase` with
/// no plaintext.
fn derive_and_verify(
    vault: &Vault,
    passphrase: &SecretString,
) -> Result<SecretBytes, SecretStoreError> {
    let key = crypto::derive_key(passphrase, &vault.salt, vault.kdf)?;
    let v_aad = format::verify_aad(format::FORMAT_VERSION, &vault.salt, &vault.kdf);
    match crypto::open(&key, &vault.verify_nonce, &v_aad, &vault.verify_ct) {
        Ok(_) => Ok(key),
        Err(SecretStoreError::Decrypt) => Err(SecretStoreError::WrongPassphrase),
        Err(e) => Err(e),
    }
}

/// Read + parse the vault at `path`, or `None` if absent. Refuses
/// looser-than-0600 perms and a file over [`MAX_VAULT_SIZE_BYTES`].
/// Opens once with `O_NOFOLLOW` and derives perms/size from the same fd
/// to avoid a metadata→read TOCTOU.
fn read_vault_at(path: &Path) -> Result<Option<Vault>, SecretStoreError> {
    let file = match open_no_follow(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(SecretStoreError::io_at(path, e)),
    };
    let meta = file
        .metadata()
        .map_err(|e| SecretStoreError::io_at(path, e))?;
    check_perms(&meta)?;
    let len = meta.len();
    if len > MAX_VAULT_SIZE_BYTES {
        return Err(SecretStoreError::VaultTooLarge {
            found: len,
            max: MAX_VAULT_SIZE_BYTES,
        });
    }
    let mut bytes = Vec::with_capacity(len as usize);
    let mut handle = file.take(MAX_VAULT_SIZE_BYTES + 1);
    handle
        .read_to_end(&mut bytes)
        .map_err(|e| SecretStoreError::io_at(path, e))?;
    if bytes.len() as u64 > MAX_VAULT_SIZE_BYTES {
        return Err(SecretStoreError::VaultTooLarge {
            found: bytes.len() as u64,
            max: MAX_VAULT_SIZE_BYTES,
        });
    }
    Ok(Some(format::deserialize(&bytes)?))
}

/// Atomically replace the vault at `path`. Stages into a same-directory
/// `NamedTempFile` (so `persist` cannot fail cross-volume), tightens to
/// 0600 before writing, then `write_all` → `sync_all` → `persist` → Unix
/// parent-dir fsync. The destination is never pre-removed, so a crash
/// leaves the old or new vault, never none. The temp holds only
/// ciphertext + header, never plaintext.
fn write_vault_at(
    path: &Path,
    vault: &Vault,
    durability_uncertain: Option<&AtomicU64>,
) -> Result<(), SecretStoreError> {
    do_write_vault_at(path, vault, durability_uncertain).inspect_err(|e| {
        tracing::warn!(error = %e, "failed to write vault file");
    })
}

fn do_write_vault_at(
    path: &Path,
    vault: &Vault,
    durability_uncertain: Option<&AtomicU64>,
) -> Result<(), SecretStoreError> {
    let serialized = format::serialize(vault);
    // Defence in depth: never write a vault the read path would refuse,
    // so the on-disk file is never left unopenable.
    if serialized.len() as u64 > MAX_VAULT_SIZE_BYTES {
        return Err(SecretStoreError::VaultTooLarge {
            found: serialized.len() as u64,
            max: MAX_VAULT_SIZE_BYTES,
        });
    }
    let parent = normalized_parent(path);
    create_parent_dir(parent)?;
    let mut tmp =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| SecretStoreError::io_at(parent, e))?;
    set_restrictive_perms(tmp.as_file())?;
    tmp.as_file_mut()
        .write_all(&serialized)
        .map_err(|e| SecretStoreError::io_at(path, e))?;
    tmp.as_file()
        .sync_all()
        .map_err(|e| SecretStoreError::io_at(path, e))?;
    tmp.persist(path)
        .map_err(|e| SecretStoreError::io_at(path, e.error))?;
    // The vault is now committed on disk via the atomic persist() rename above.
    // A subsequent parent-directory fsync failure cannot undo the already-written
    // file. Propagating the error would force callers (put/delete/rekey) to roll
    // back in-memory state that already matches the on-disk vault — diverging the
    // live handle from disk. So this stays NON-FATAL: the write returns `Ok` and
    // the data is committed + visible. We surface the unconfirmed power-loss
    // durability two ways instead of swallowing it — an `error!` log AND a bump
    // of the pollable `durability_uncertain` counter
    // ([`EncryptedFileStore::durability_uncertain_count`]).
    #[cfg(unix)]
    {
        let signal_unconfirmed = |e: &std::io::Error| {
            tracing::error!(
                error = %e,
                parent = %parent.display(),
                "parent-dir fsync unconfirmed after vault persist; data is committed on disk \
                 but its rename durability across power loss is NOT confirmed"
            );
            if let Some(counter) = durability_uncertain {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        };
        match fs::File::open(parent) {
            Ok(d) => {
                if let Err(e) = d.sync_all() {
                    signal_unconfirmed(&e);
                }
            }
            Err(e) => signal_unconfirmed(&e),
        }
    }
    Ok(())
}

/// Normalize `path.parent()` to a usable directory: `Path::parent()`
/// returns `Some("")` for a bare filename, which errors at
/// `create_dir_all` / `NamedTempFile::new_in` / parent-dir fsync — map
/// the empty parent to "." so a bare filename in the cwd just works.
fn normalized_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

/// Create the vault's parent directory at `0700` on Unix (not
/// world-readable). Idempotent — a pre-existing directory is left alone.
fn create_parent_dir(parent: &Path) -> Result<(), SecretStoreError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(parent)?;
    }
    // INTENTIONAL: Windows parent-dir ACL hardening deferred to
    // https://github.com/dashpay/platform/issues/3754 — tighten manually.
    #[cfg(not(unix))]
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Advisory write-lock holder owning a heap-pinned `Box<RwLock<File>>`
/// and a self-referential `'static` guard borrowing from it. The
/// self-reference is unavoidable: `fd-lock`'s guard borrows the `RwLock`,
/// and the resident-vault model needs the lock held continuously between
/// `open` and `Drop`. Safety:
///
/// 1. `Box::into_raw` gives the `RwLock` a stable address for the
///    holder's lifetime.
/// 2. The `'static` guard lifetime is a lie sound only because `Drop`
///    takes the guard out (running its Drop, releasing the OS lock)
///    BEFORE reclaiming the box.
/// 3. The raw pointer never escapes this module.
///
/// Calibrated to `fd-lock = "=4.0.4"` (exact-pinned): any bump must
/// re-verify the guard releases the OS lock before the box is reclaimed.
mod vault_lock {
    // INTENTIONAL: the crate's only unsafe island; soundness rests on the
    // drop-order argument above, not a Miri test. `#![deny(unsafe_code)]`
    // still applies everywhere outside the narrowed per-item allows.

    use std::fs;
    use std::path::Path;

    #[cfg(unix)]
    use super::set_restrictive_perms;
    use crate::secrets::error::SecretStoreError;

    pub(super) struct VaultLock {
        rwlock: *mut fd_lock::RwLock<fs::File>,
        guard: Option<fd_lock::RwLockWriteGuard<'static, fs::File>>,
    }

    // SAFETY: `RwLock<File>` is `Send + Sync` (its only non-trivial
    // member is a `File`/`RawFd`, both `Send + Sync`). The raw pointer
    // points at the heap-pinned `RwLock` this struct owns; sending the
    // struct moves ownership of the box address with it.
    #[allow(unsafe_code)]
    unsafe impl Send for VaultLock {}
    #[allow(unsafe_code)]
    unsafe impl Sync for VaultLock {}

    impl VaultLock {
        pub(super) fn acquire(lock_path: &Path) -> Result<Self, SecretStoreError> {
            // INTENTIONAL: non-unix symlink hardening (Windows needs
            // FILE_FLAG_OPEN_REPARSE_POINT) deferred to
            // https://github.com/dashpay/platform/issues/3754.
            let mut opts = fs::OpenOptions::new();
            opts.read(true).write(true).create(true).truncate(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.custom_flags(libc::O_NOFOLLOW);
                // Restrictive from the first byte — no loose-perm window.
                opts.mode(0o600);
            }
            let lock_file = opts
                .open(lock_path)
                .map_err(|e| SecretStoreError::io_at(lock_path, e))?;
            // `mode()` only applies to a file this call creates; re-assert
            // 0600 on a pre-existing sidecar.
            #[cfg(unix)]
            set_restrictive_perms(&lock_file)?;

            let raw: *mut fd_lock::RwLock<fs::File> =
                Box::into_raw(Box::new(fd_lock::RwLock::new(lock_file)));

            // SAFETY: `raw` came straight from `Box::into_raw` of a
            // fresh box; it is non-null, properly aligned, and points
            // at a valid `RwLock<File>`. No other reference exists
            // yet, so promoting it to `&'static mut` is sound for the
            // borrow we hand to `try_write`.
            #[allow(unsafe_code)]
            let static_ref: &'static mut fd_lock::RwLock<fs::File> = unsafe { &mut *raw };

            let guard = match static_ref.try_write() {
                Ok(guard) => guard,
                Err(e) => {
                    // SAFETY: the guard never came into existence, so
                    // no live borrow points at the box; reclaiming
                    // here is sound and avoids leaking on the error
                    // path.
                    #[allow(unsafe_code)]
                    unsafe {
                        drop(Box::from_raw(raw))
                    };
                    return Err(match e.kind() {
                        std::io::ErrorKind::WouldBlock => SecretStoreError::AlreadyLocked,
                        _ => SecretStoreError::from(e),
                    });
                }
            };

            Ok(Self {
                rwlock: raw,
                guard: Some(guard),
            })
        }
    }

    impl Drop for VaultLock {
        fn drop(&mut self) {
            // Drop the guard FIRST so its Drop impl releases the OS
            // lock while the backing `RwLock<File>` is still alive.
            self.guard.take();
            // SAFETY: `rwlock` came from `Box::into_raw` in `acquire`,
            // the guard has just been dropped (no live borrow), and we
            // are the only owner. Reclaiming the Box runs the
            // `RwLock`'s Drop, which closes the file fd.
            #[allow(unsafe_code)]
            unsafe {
                drop(Box::from_raw(self.rwlock))
            };
        }
    }
}

use vault_lock::VaultLock;

/// Why a wallet-id hex string failed the canonical-form check, so each
/// caller of [`wallet_id_hex_to_bytes`] can map the reason onto its own
/// error type and message.
enum WalletIdHexError {
    /// Not exactly 64 characters.
    WrongLength,
    /// Contains uppercase hex digits — off-canonical.
    Uppercase,
    /// Not valid hexadecimal.
    NotHex,
}

/// Decode a wallet id into 32 bytes, enforcing the canonical form
/// `WalletId::to_hex` writes (64 lowercase hex chars). The single seam
/// for both the on-disk outer-key check and the SPI service-string parse.
fn wallet_id_hex_to_bytes(s: &str) -> Result<[u8; 32], WalletIdHexError> {
    if s.len() != 64 {
        return Err(WalletIdHexError::WrongLength);
    }
    if s.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(WalletIdHexError::Uppercase);
    }
    let mut out = [0u8; 32];
    hex::decode_to_slice(s, &mut out).map_err(|_| WalletIdHexError::NotHex)?;
    Ok(out)
}

/// Decode the on-disk outer-key wallet hex into the 32 bytes the AAD
/// expects. The outer key is a free-form string at the type level, so
/// this bytes-back check is a defence-in-depth structural guard; any
/// off-canonical key is rejected as corruption.
pub(super) fn decode_wallet_id_hex(s: &str) -> Result<[u8; 32], SecretStoreError> {
    wallet_id_hex_to_bytes(s).map_err(|_| SecretStoreError::MalformedVault)
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
    let bytes = wallet_id_hex_to_bytes(hex).map_err(|reason| {
        let msg = match reason {
            WalletIdHexError::WrongLength => "wallet id hex must be 64 chars",
            WalletIdHexError::Uppercase => "wallet id hex must be lowercase",
            WalletIdHexError::NotHex => "wallet id hex is not valid hex",
        };
        KeyringError::Invalid("service".to_string(), msg.to_string())
    })?;
    Ok(WalletId::from(bytes))
}

/// A `(wallet_id, label)` row in an [`EncryptedFileStore`]. Holds a
/// cloned handle to the parent so each op goes through the same store API
/// and single-lock critical section. All ops re-validate the label; the
/// passphrase was already verified at open, so no wrong-pass race here.
pub struct EncryptedFileCredential {
    store: EncryptedFileStore,
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
        let _ = validated_label(&self.label).map_err(SecretStoreError::from)?;
        // Cap before wrapping so an oversized secret is never materialized.
        if secret.len() > MAX_SECRET_LEN {
            return Err(KeyringError::from(SecretStoreError::SecretTooLarge {
                found: secret.len(),
                max: MAX_SECRET_LEN,
            }));
        }
        self.store
            .put_bytes(
                &self.wallet_id,
                &self.label,
                &SecretBytes::from_slice(secret),
            )
            .map_err(KeyringError::from)
    }

    fn get_secret(&self) -> KeyringResult<Vec<u8>> {
        let _ = validated_label(&self.label).map_err(SecretStoreError::from)?;
        match self.store.get_bytes(&self.wallet_id, &self.label) {
            // SPI contract forces a bare Vec; caller owns disposal —
            // prefer SecretStore::get for a zeroizing SecretBytes.
            Ok(Some(v)) => Ok(v.expose_secret().to_vec()),
            Ok(None) => Err(KeyringError::NoEntry),
            Err(e) => Err(e.into()),
        }
    }

    fn delete_credential(&self) -> KeyringResult<()> {
        let _ = validated_label(&self.label).map_err(SecretStoreError::from)?;
        match self.store.delete_bytes(&self.wallet_id, &self.label) {
            Ok(true) => Ok(()),
            Ok(false) => Err(KeyringError::NoEntry),
            Err(e) => Err(e.into()),
        }
    }

    fn get_credential(&self) -> KeyringResult<Option<Arc<Credential>>> {
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

    /// Build a credential for `(service, user)`. SPI-direct consumers:
    /// format the returned [`KeyringError`] with `Display`, never `Debug`
    /// — byte-bearing variants embed raw bytes in `Debug` (CWE-209/
    /// CWE-532). Prefer the typed
    /// [`SecretStore`](crate::secrets::SecretStore) path.
    fn build(
        &self,
        service: &str,
        user: &str,
        _modifiers: Option<&HashMap<&str, &str>>,
    ) -> KeyringResult<Entry> {
        let wallet_id = parse_service(service)?;
        let label = validated_label(user)
            .map_err(SecretStoreError::from)?
            .to_string();
        let cred = EncryptedFileCredential {
            store: self.clone(),
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
        // `try_lock` (not `lock_inner`) so a Debug from a panic path that
        // already holds the lock cannot deadlock; poison folds into the
        // same fallback as contention.
        let path: PathBuf = match self.inner.try_lock() {
            Ok(guard) => guard.path.clone(),
            Err(_) => PathBuf::from("<unavailable>"),
        };
        f.debug_struct("EncryptedFileStore")
            .field("path", &path)
            .finish_non_exhaustive()
    }
}

/// Map an entry-level `crypto::open` failure to the typed distinction.
/// The verify-token has already passed at every caller, so a `Decrypt`
/// here is entry corruption/tampering, NOT a wrong passphrase — logs the
/// non-secret `(wallet_id, label)` and maps to `Corruption`; other
/// variants pass through.
fn entry_decrypt_or_corruption(
    wallet_hex: &str,
    label: &str,
    result: Result<SecretBytes, SecretStoreError>,
) -> Result<SecretBytes, SecretStoreError> {
    result.map_err(|err| match err {
        SecretStoreError::Decrypt => {
            tracing::error!(
                wallet_id = %wallet_hex,
                label = %label,
                "vault entry failed integrity check (corruption or tampering)"
            );
            SecretStoreError::Corruption
        }
        other => other,
    })
}

#[cfg(unix)]
fn check_perms(meta: &fs::Metadata) -> Result<(), SecretStoreError> {
    use std::os::unix::fs::MetadataExt;
    let mode = meta.mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(SecretStoreError::InsecurePermissions { mode });
    }
    Ok(())
}

// INTENTIONAL: Windows ACL read-check (needs GetSecurityInfo) deferred to
// https://github.com/dashpay/platform/issues/3754 — set ACLs manually.
#[cfg(not(unix))]
fn check_perms(_meta: &fs::Metadata) -> Result<(), SecretStoreError> {
    Ok(())
}

/// Refuse a group/other-WRITABLE vault parent (`mode & 0o022`). The
/// threat is rename/unlink/replace, which POSIX gates on directory WRITE,
/// so this targets write bits only — a 0o755 read-only parent leaks
/// filenames but not the 0600 contents and is the common layout.
/// `DirBuilder::mode` only hardens dirs this process creates, so a
/// pre-existing loose dir must still be checked here.
#[cfg(unix)]
fn check_parent_perms(parent: &Path) -> Result<(), SecretStoreError> {
    use std::os::unix::fs::MetadataExt;
    let meta = fs::metadata(parent).map_err(|e| SecretStoreError::io_at(parent, e))?;
    let mode = meta.mode() & 0o777;
    if mode & 0o022 != 0 {
        return Err(SecretStoreError::InsecureParentDir { mode });
    }
    Ok(())
}

// INTENTIONAL: Windows parent-dir ACL check deferred to the same
// follow-up as `check_perms` — https://github.com/dashpay/platform/issues/3754.
#[cfg(not(unix))]
fn check_parent_perms(_parent: &Path) -> Result<(), SecretStoreError> {
    Ok(())
}

#[cfg(unix)]
fn set_restrictive_perms(f: &fs::File) -> Result<(), SecretStoreError> {
    use std::os::unix::fs::PermissionsExt;
    f.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(())
}

// INTENTIONAL: Windows ACL tightening (needs SetSecurityInfo) deferred to
// https://github.com/dashpay/platform/issues/3754 — set ACLs manually.
#[cfg(not(unix))]
fn set_restrictive_perms(_f: &fs::File) -> Result<(), SecretStoreError> {
    Ok(())
}

/// Open a vault file for reading. On Unix the open refuses to traverse
/// a final-component symlink (`O_NOFOLLOW`) so a symlink swap between
/// the open and the read cannot redirect us to a different inode.
#[cfg(unix)]
fn open_no_follow(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(not(unix))]
fn open_no_follow(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new().read(true).open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_at(path: &Path) -> EncryptedFileStore {
        EncryptedFileStore::open(path, SecretString::new("pw-correct")).unwrap()
    }

    fn vault_path(dir: &Path) -> PathBuf {
        // Tighten the umask-0002 tempdir (0o775) to 0o700 so it passes the
        // parent-dir perm check (dedicated perm tests use a subdir).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
        }
        dir.join("vault.pwsvault")
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
    /// `SecretStoreError` boxed as the source, recoverable losslessly.
    fn is_wrong_passphrase(e: &KeyringError) -> bool {
        matches!(
            e,
            KeyringError::NoStorageAccess(src)
                if matches!(src.downcast_ref::<SecretStoreError>(), Some(SecretStoreError::WrongPassphrase))
        )
    }

    /// Whether a projected SPI error is the lossy `Corruption`
    /// projection.
    fn is_corruption(e: &KeyringError) -> bool {
        matches!(e, KeyringError::BadStoreFormat(s) if *s == SecretStoreError::Corruption.to_string())
    }

    #[test]
    fn open_creates_vault_file_on_first_open() {
        // open() creates a usable vault file even without a put.
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let _s = store_at(&path);
        }
        assert!(path.exists(), "open() must create the vault file");
    }

    #[test]
    fn roundtrip_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "bip39_mnemonic")
                .set_secret(b"abandon abandon")
                .unwrap();
        }
        let s2 = store_at(&path);
        let got = entry(&s2, wid(1), "bip39_mnemonic").get_secret().unwrap();
        assert_eq!(got, b"abandon abandon");
        let missing = entry(&s2, wid(1), "missing").get_secret().unwrap_err();
        assert!(matches!(missing, KeyringError::NoEntry));
    }

    /// The durability-uncertain counter exists, starts at 0, and a normal
    /// write (whose parent-dir fsync is confirmed) does NOT bump it. The
    /// increment-on-fsync-FAILURE path is environment-specific and not forced
    /// here; this guards the accessor + the no-spurious-bump invariant.
    #[test]
    fn durability_uncertain_count_zero_for_confirmed_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s = store_at(&path);
        assert_eq!(s.durability_uncertain_count(), 0, "fresh store has none");
        entry(&s, wid(1), "bip39_mnemonic")
            .set_secret(b"abandon abandon")
            .unwrap();
        assert_eq!(
            s.durability_uncertain_count(),
            0,
            "a confirmed-durable write must not bump the counter"
        );
    }

    #[test]
    fn wrong_passphrase_on_reopen_fails_with_typed_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "seed")
                .set_secret(b"super secret")
                .unwrap();
        }
        let err = EncryptedFileStore::open(&path, SecretString::new("pw-wrong")).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassphrase),
            "got {err:?}"
        );
        // The error renders without any plaintext.
        assert!(!format!("{err}").contains("super secret"));
    }

    #[test]
    fn open_acquires_exclusive_lock_until_drop() {
        // A second open() while the first store is alive returns
        // AlreadyLocked; once it drops, a fresh open() succeeds.
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s1 = store_at(&path);
        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("second open must contend");
        assert!(
            matches!(err, SecretStoreError::AlreadyLocked),
            "got {err:?}"
        );
        drop(s1);
        let _s2 = store_at(&path);
    }

    #[test]
    fn mutations_are_visible_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "A").set_secret(b"value-A").unwrap();
            entry(&s, wid(2), "B").set_secret(b"value-B").unwrap();
        }
        let s2 = store_at(&path);
        assert_eq!(entry(&s2, wid(1), "A").get_secret().unwrap(), b"value-A");
        assert_eq!(entry(&s2, wid(2), "B").get_secret().unwrap(), b"value-B");
    }

    #[test]
    fn delete_returns_no_entry_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let s = store_at(&vault_path(dir.path()));
        assert!(matches!(
            entry(&s, wid(1), "seed").delete_credential(),
            Err(KeyringError::NoEntry)
        ));
        entry(&s, wid(1), "seed").set_secret(b"v1").unwrap();
        entry(&s, wid(1), "seed").set_secret(b"v2").unwrap();
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"v2");
        entry(&s, wid(1), "seed").delete_credential().unwrap();
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
        let path = vault_path(dir.path());
        let s = store_at(&path);
        entry(&s, wid(1), "labelA").set_secret(b"secretA").unwrap();
        entry(&s, wid(1), "labelB").set_secret(b"secretB").unwrap();
        let mut vault = s.test_read_vault_from_disk().unwrap().unwrap();
        let wallet_hex = wid(1).to_hex();
        let entries = vault.wallets.get_mut(&wallet_hex).unwrap();
        let a = entries["labelA"].clone();
        let b = entries.get_mut("labelB").unwrap();
        b.nonce = a.nonce;
        b.ciphertext = a.ciphertext.clone();
        s.test_write_vault_to_disk(&vault).unwrap();
        s.test_reload_from_disk().unwrap();
        let err = entry(&s, wid(1), "labelB").get_secret().unwrap_err();
        // The verify-token passes (correct passphrase), so the
        // cross-label ciphertext swap surfaces as entry corruption, not
        // a wrong passphrase.
        assert!(is_corruption(&err), "unexpected error: {err:?}");
    }

    #[test]
    fn blob_swap_across_wallet_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s = store_at(&path);
        entry(&s, wid(1), "seed").set_secret(b"secretA").unwrap();
        entry(&s, wid(2), "seed").set_secret(b"secretB").unwrap();
        let mut vault = s.test_read_vault_from_disk().unwrap().unwrap();
        let body_a = vault.wallets[&wid(1).to_hex()]["seed"].clone();
        vault
            .wallets
            .get_mut(&wid(2).to_hex())
            .unwrap()
            .insert("seed".to_string(), body_a);
        s.test_write_vault_to_disk(&vault).unwrap();
        s.test_reload_from_disk().unwrap();
        let err = entry(&s, wid(2), "seed").get_secret().unwrap_err();
        assert!(is_corruption(&err), "unexpected error: {err:?}");
    }

    #[cfg(unix)]
    #[test]
    fn vault_created_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let _s = store_at(&path);
        // Resident-vault model creates the file at open(), so the perm
        // assertion fires without a subsequent set_secret.
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn loose_perms_preexisting_file_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "seed").set_secret(b"x").unwrap();
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("loose perms must be refused at open");
        assert!(
            matches!(err, SecretStoreError::InsecurePermissions { mode: 0o644 }),
            "got {err:?}"
        );
    }

    #[test]
    fn rekey_reencrypts_and_old_passphrase_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let old_bytes = {
            let s = store_at(&path);
            entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
            let pre = fs::read(&path).unwrap();
            s.rekey(SecretString::new("pw-new")).unwrap();
            assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"value");
            let new_bytes = fs::read(&path).unwrap();
            assert_ne!(pre, new_bytes);
            pre
        };
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

        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("old passphrase must fail to open");
        assert!(
            matches!(err, SecretStoreError::WrongPassphrase),
            "got {err:?}"
        );
        // The new bytes are intact.
        let new_bytes = fs::read(&path).unwrap();
        assert_ne!(old_bytes, new_bytes);
    }

    /// Whole-store rekey is atomic: every wallet's entries are
    /// re-encrypted under the new passphrase in one shot, and the old
    /// passphrase fails on reopen.
    #[test]
    fn rekey_rotates_whole_store_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "seedA").set_secret(b"value-A").unwrap();
            entry(&s, wid(2), "seedB").set_secret(b"value-B").unwrap();
            entry(&s, wid(2), "seedB2").set_secret(b"value-B2").unwrap();

            s.rekey(SecretString::new("pw-rotated")).unwrap();
            assert_eq!(entry(&s, wid(1), "seedA").get_secret().unwrap(), b"value-A");
            assert_eq!(entry(&s, wid(2), "seedB").get_secret().unwrap(), b"value-B");
            assert_eq!(
                entry(&s, wid(2), "seedB2").get_secret().unwrap(),
                b"value-B2"
            );
        }

        // Reopening with the OLD passphrase fails at open().
        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("old passphrase rejected on reopen");
        assert!(matches!(err, SecretStoreError::WrongPassphrase));

        // Reopening with the NEW passphrase reads every wallet.
        let new_pp = EncryptedFileStore::open(&path, SecretString::new("pw-rotated")).unwrap();
        assert_eq!(
            entry(&new_pp, wid(1), "seedA").get_secret().unwrap(),
            b"value-A"
        );
        assert_eq!(
            entry(&new_pp, wid(2), "seedB").get_secret().unwrap(),
            b"value-B"
        );
        assert_eq!(
            entry(&new_pp, wid(2), "seedB2").get_secret().unwrap(),
            b"value-B2"
        );
    }

    /// A mid-rekey write failure (parent dir made read-only after the
    /// first write) must leave the original vault file intact and the
    /// in-memory state must revert to the old key/pass so the old data
    /// remains readable through the live handle.
    #[cfg(unix)]
    #[test]
    fn rekey_does_not_corrupt_on_disk_temp_failure() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s = store_at(&path);
        entry(&s, wid(1), "seed").set_secret(b"value-A").unwrap();
        let original_bytes = fs::read(&path).unwrap();

        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let result = s.rekey(SecretString::new("pw-new"));
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        assert!(result.is_err(), "rekey should have failed: {result:?}");
        let on_disk = fs::read(&path).unwrap();
        assert_eq!(on_disk, original_bytes, "vault was modified mid-rekey");

        // The in-memory passphrase/key were reverted, so the live store
        // still serves the original value under the old pass.
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"value-A");

        drop(s);
        let reopened = EncryptedFileStore::open(&path, SecretString::new("pw-correct")).unwrap();
        assert_eq!(
            entry(&reopened, wid(1), "seed").get_secret().unwrap(),
            b"value-A"
        );
    }

    /// A parent-dir fsync failure that occurs AFTER `persist()` has already
    /// committed the vault to disk must NOT cause `put` to return an error or
    /// roll back the in-memory entry.  The vault is already on disk; propagating
    /// the post-persist error would diverge in-memory state from the on-disk file.
    ///
    /// Trigger: set the parent dir to `0o300` (write + execute, no read) so
    /// `persist()` (a rename — needs write) succeeds but the subsequent
    /// `fs::File::open(parent)` for fsync fails (needs read).
    #[cfg(unix)]
    #[test]
    fn put_succeeds_when_parent_dir_fsync_fails_post_persist() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s = store_at(&path);
        entry(&s, wid(1), "seed")
            .set_secret(b"first-value")
            .unwrap();

        // Tighten the parent dir to write+execute only (no read):
        //   - NamedTempFile::new_in  : needs write (0o200) + execute (0o100) → OK
        //   - persist() (rename)     : needs write on directory              → OK
        //   - fs::File::open(parent) : needs read (0o400) — MISSING          → FAILS
        // This forces the post-persist parent-dir fsync to fail, which is the
        // scenario under test.
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o300)).unwrap();
        let result = entry(&s, wid(1), "seed").set_secret(b"second-value");
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        // Post-fix: put must succeed even though the parent-dir fsync failed —
        // the vault was already committed via persist().
        assert!(
            result.is_ok(),
            "put must succeed even when parent-dir fsync fails post-persist; got {result:?}"
        );

        // In-memory state was NOT rolled back.
        assert_eq!(
            entry(&s, wid(1), "seed").get_secret().unwrap(),
            b"second-value",
            "in-memory state must reflect the committed put"
        );

        // Drop `s` (releases the vault lock) before opening a second store on
        // the same file — `EncryptedFileStore::open` is exclusive by design.
        drop(s);

        // The vault on disk reflects the committed value.
        let reopened = EncryptedFileStore::open(&path, SecretString::new("pw-correct")).unwrap();
        assert_eq!(
            entry(&reopened, wid(1), "seed").get_secret().unwrap(),
            b"second-value",
            "vault on disk must have the committed value after post-persist fsync failure"
        );
    }

    #[test]
    fn get_corruption_after_verify_token_is_not_wrong_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s = store_at(&path);
        entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"value");
        // Bit-flip the entry ciphertext on disk; verify-token is intact.
        let mut vault = s.test_read_vault_from_disk().unwrap().unwrap();
        vault
            .wallets
            .get_mut(&wid(1).to_hex())
            .unwrap()
            .get_mut("seed")
            .unwrap()
            .ciphertext[0] ^= 0x01;
        s.test_write_vault_to_disk(&vault).unwrap();
        s.test_reload_from_disk().unwrap();
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
        let path = vault_path(dir.path());
        let s = store_at(&path);
        entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        // Corrupt the entry ciphertext on disk; leave the verify-token.
        let mut vault = s.test_read_vault_from_disk().unwrap().unwrap();
        vault
            .wallets
            .get_mut(&wid(1).to_hex())
            .unwrap()
            .get_mut("seed")
            .unwrap()
            .ciphertext[0] ^= 0x01;
        s.test_write_vault_to_disk(&vault).unwrap();
        s.test_reload_from_disk().unwrap();
        // Rekey re-encrypts every entry under the new key — the
        // corrupt entry fails AEAD-open under the (correct) old key,
        // and we project that as Corruption.
        let err = s.rekey(SecretString::new("pw-new")).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::Corruption),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn correct_passphrase_round_trips_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let s = store_at(&vault_path(dir.path()));
        entry(&s, wid(1), "seed").set_secret(b"orig").unwrap();
        entry(&s, wid(1), "seed2").set_secret(b"second").unwrap();
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"orig");
        assert_eq!(entry(&s, wid(1), "seed2").get_secret().unwrap(), b"second");
    }

    #[test]
    fn no_plaintext_in_vault_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s = store_at(&path);
        entry(&s, wid(1), "seed")
            .set_secret(b"PLAINTEXTNEEDLE")
            .unwrap();
        let raw = fs::read(&path).unwrap();
        assert!(
            raw.windows(b"PLAINTEXTNEEDLE".len())
                .all(|w| w != b"PLAINTEXTNEEDLE"),
            "plaintext leaked into vault file"
        );
    }

    /// The no-plaintext-at-rest guarantee also holds through the public
    /// `SecretStore::set` path (which writes an unprotected envelope sealed
    /// under the vault key), not just the raw SPI entry path.
    #[test]
    fn no_plaintext_in_vault_file_via_secret_store_set() {
        use crate::secrets::SecretStore;
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let store = SecretStore::file(&path, SecretString::new("pw-correct")).unwrap();
        store
            .set(
                &wid(1),
                "seed",
                &SecretBytes::from_slice(b"PLAINTEXTNEEDLE"),
            )
            .unwrap();
        let raw = fs::read(&path).unwrap();
        assert!(
            raw.windows(b"PLAINTEXTNEEDLE".len())
                .all(|w| w != b"PLAINTEXTNEEDLE"),
            "plaintext leaked into vault file via SecretStore::set"
        );
    }

    /// A blank passphrase is rejected at `open` →
    /// `BlankPassphrase`; no vault file (or lock sidecar) is created.
    #[test]
    fn open_rejects_blank_passphrase() {
        for blank in [
            SecretString::empty(),
            SecretString::new(""),
            SecretString::new("   "),
            SecretString::new("\t\n"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = vault_path(dir.path());
            let err = EncryptedFileStore::open(&path, blank).unwrap_err();
            assert!(
                matches!(err, SecretStoreError::BlankPassphrase),
                "blank passphrase must be rejected, got {err:?}"
            );
            assert!(!path.exists(), "no vault file for a blank passphrase");
            assert!(
                !lock_path_for(&path).exists(),
                "no lock sidecar for a blank passphrase"
            );
        }
    }

    /// A blank passphrase is rejected at `rekey`; the resident
    /// vault, key, and on-disk file are UNCHANGED — the original passphrase
    /// still reads every entry, live and after reopen.
    #[test]
    fn rekey_rejects_blank_passphrase_vault_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s = store_at(&path); // real "pw-correct"
        entry(&s, wid(1), "seed").set_secret(b"v1").unwrap();
        for blank in [SecretString::empty(), SecretString::new("   ")] {
            let err = s.rekey(blank).unwrap_err();
            assert!(
                matches!(err, SecretStoreError::BlankPassphrase),
                "blank rekey must be rejected, got {err:?}"
            );
        }
        // Old passphrase still reads the entry, live…
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"v1");
        // …and after a clean reopen under the original passphrase.
        drop(s);
        let s2 = store_at(&path);
        assert_eq!(entry(&s2, wid(1), "seed").get_secret().unwrap(), b"v1");
    }

    /// `open_unprotected` permits a deliberate keyless vault that
    /// round-trips; a real-passphrase `open` of that keyless vault then
    /// fails with `WrongPassphrase` (it is keyless, not real-pass).
    #[test]
    fn open_unprotected_permits_keyless_vault() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = EncryptedFileStore::open_unprotected(&path).unwrap();
            entry(&s, wid(1), "seed")
                .set_secret(b"keyless-seed")
                .unwrap();
        }
        {
            let s = EncryptedFileStore::open_unprotected(&path).unwrap();
            assert_eq!(
                entry(&s, wid(1), "seed").get_secret().unwrap(),
                b"keyless-seed"
            );
        }
        let err = EncryptedFileStore::open(&path, SecretString::new("real")).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassphrase),
            "real-pass open of a keyless vault must fail, got {err:?}"
        );
    }

    /// Empty→real passphrase migration via `rekey`. After rekey,
    /// `open(real)` reads every entry; the keyless door no longer opens it;
    /// no `.bak`/`.tmp` residue beside the vault.
    #[test]
    fn empty_to_real_rekey_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = EncryptedFileStore::open_unprotected(&path).unwrap();
            entry(&s, wid(1), "seed").set_secret(b"migrate-me").unwrap();
            s.rekey(SecretString::new("real-pass")).unwrap();
            // The live handle keeps working post-rekey.
            assert_eq!(
                entry(&s, wid(1), "seed").get_secret().unwrap(),
                b"migrate-me"
            );
        }
        // Reopen under the real passphrase reads the entry.
        {
            let s = EncryptedFileStore::open(&path, SecretString::new("real-pass")).unwrap();
            assert_eq!(
                entry(&s, wid(1), "seed").get_secret().unwrap(),
                b"migrate-me"
            );
        }
        // The keyless door no longer opens it.
        let err = EncryptedFileStore::open_unprotected(&path).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::WrongPassphrase),
            "keyless open after migration must fail, got {err:?}"
        );
        // No .bak / .tmp residue (mirrors rekey_reencrypts_and_old_passphrase_fails).
        for sibling in fs::read_dir(dir.path()).unwrap().flatten() {
            let name = sibling.file_name();
            let name = name.to_string_lossy();
            assert!(
                !name.ends_with(".bak") && !name.ends_with(".tmp"),
                "unexpected residue: {name}"
            );
        }
    }

    /// Crash-safety: a disk-write failure mid-rekey leaves the
    /// pre-rekey keyless vault intact and readable via `open_unprotected`
    /// (mirrors rekey_does_not_corrupt_on_disk_temp_failure).
    #[cfg(unix)]
    #[test]
    fn empty_to_real_rekey_crash_safe_stays_keyless() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s = EncryptedFileStore::open_unprotected(&path).unwrap();
        entry(&s, wid(1), "seed").set_secret(b"keyless").unwrap();

        // Read-only parent → the rekey atomic temp-write fails.
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let err = s.rekey(SecretString::new("real-pass")).unwrap_err();
        assert!(matches!(err, SecretStoreError::Io(_)), "got {err:?}");
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        // The live handle still serves the pre-rekey keyless vault…
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"keyless");
        // …and on disk it is still the keyless vault.
        drop(s);
        let s2 = EncryptedFileStore::open_unprotected(&path).unwrap();
        assert_eq!(entry(&s2, wid(1), "seed").get_secret().unwrap(), b"keyless");
    }

    #[test]
    fn build_rejects_malformed_service() {
        let dir = tempfile::tempdir().unwrap();
        let s = store_at(&vault_path(dir.path()));
        for bad in [
            "no-prefix",
            "dash.platform-wallet-storage/short",
            "wrong-app/0000000000000000000000000000000000000000000000000000000000000000",
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
        let s = store_at(&vault_path(dir.path()));
        let upper = format!("{SERVICE_PREFIX}{}", "A".repeat(64));
        let err = s.build(&upper, "seed", None).unwrap_err();
        match err {
            KeyringError::Invalid(attr, _) => assert_eq!(attr, "service"),
            other => panic!("expected Invalid(\"service\"), got {other:?}"),
        }
        let lower = format!("{SERVICE_PREFIX}{}", "aa".repeat(32));
        s.build(&lower, "seed", None)
            .expect("lowercase hex accepted");
    }

    #[test]
    fn decode_wallet_id_hex_enforces_canonical_form() {
        // The on-disk outer-key check matches the SPI parse: only 64
        // lowercase hex chars round-trip; uppercase, wrong length and
        // non-hex all surface as corruption.
        decode_wallet_id_hex(&"aa".repeat(32)).expect("lowercase hex accepted");
        for bad in [&"A".repeat(64), &"aa".repeat(16), &"zz".repeat(32)] {
            assert!(
                matches!(
                    decode_wallet_id_hex(bad),
                    Err(SecretStoreError::MalformedVault)
                ),
                "expected MalformedVault for off-canonical key {bad:?}"
            );
        }
    }

    #[test]
    fn build_rejects_invalid_label() {
        let dir = tempfile::tempdir().unwrap();
        let s = store_at(&vault_path(dir.path()));
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
        let s = store_at(&vault_path(dir.path()));
        let e = entry(&s, wid(1), "seed");
        let (service, user) = e.get_specifiers().unwrap();
        assert_eq!(service, format!("{SERVICE_PREFIX}{}", wid(1).to_hex()));
        assert_eq!(user, "seed");
    }

    #[test]
    fn second_write_over_existing_vault_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let s = store_at(&vault_path(dir.path()));
        entry(&s, wid(1), "seed").set_secret(b"v1").unwrap();
        entry(&s, wid(1), "seed").set_secret(b"v2").unwrap();
        entry(&s, wid(1), "other").set_secret(b"v3").unwrap();
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"v2");
        assert_eq!(entry(&s, wid(1), "other").get_secret().unwrap(), b"v3");
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
    fn inflated_kdf_params_fail_open_with_kdf_failure() {
        // A JSON m_kib = u32::MAX must be refused at open() with
        // KdfFailure, before the ~4 TiB allocation it would demand. Drop
        // the store before patching disk so the drop-sync can't undo it.
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        }
        let mut vault = read_vault_at(&path).unwrap().unwrap();
        vault.kdf.m_kib = u32::MAX;
        write_vault_at(&path, &vault, None).unwrap();
        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("inflated KDF must fail open");
        assert!(matches!(err, SecretStoreError::KdfFailure), "got {err:?}");
    }

    #[test]
    fn persistence_is_until_delete() {
        let dir = tempfile::tempdir().unwrap();
        let s = store_at(&vault_path(dir.path()));
        assert!(matches!(
            s.persistence(),
            CredentialPersistence::UntilDelete
        ));
        assert_eq!(s.vendor(), VENDOR);
        assert_eq!(s.id(), STORE_ID);
    }

    /// Size cap: vault files larger than [`MAX_VAULT_SIZE_BYTES`] must
    /// fail BEFORE the read allocates. The check fires at open()
    /// under the resident-vault model.
    #[test]
    fn vault_above_size_cap_is_rejected_pre_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "seed").set_secret(b"v").unwrap();
        }
        let oversized = MAX_VAULT_SIZE_BYTES + 1;
        let f = fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(oversized).unwrap();
        drop(f);

        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("oversized vault must be refused");
        assert!(
            matches!(
                err,
                SecretStoreError::VaultTooLarge { found, max }
                    if found == oversized && max == MAX_VAULT_SIZE_BYTES
            ),
            "unexpected error: {err:?}"
        );
    }

    /// Two threads share a cloned handle: one hammers put/get, the other
    /// rekeys. The single state lock serializes them, so a `get` only ever
    /// returns the right plaintext, `Ok(None)`, or a clean typed error —
    /// never garbled bytes from a put that sealed under a swapped key.
    #[test]
    fn rekey_does_not_race_put_into_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let store = store_at(&path);
        let writer_store = store.clone();
        let rekeyer_store = store.clone();

        // Counts tuned for cost: each rekey runs Argon2, so 16 rekeys
        // overlapping a 200-iter put loop hits the race window affordably.
        const PUT_ITERS: usize = 200;
        const REKEY_ITERS: usize = 16;
        let wallet = wid(7);
        let label = "racy";

        // Fixed prefix, never built with `format!` so the secrets-guard
        // scanner does not flag this as a sink/expose_secret pairing.
        const PREFIX: &[u8] = b"payload-";
        let writer = std::thread::spawn(move || {
            let mut buf = Vec::with_capacity(PREFIX.len() + 4);
            for i in 0..PUT_ITERS {
                buf.clear();
                buf.extend_from_slice(PREFIX);
                buf.extend_from_slice(&(i as u32).to_le_bytes());
                writer_store
                    .put_bytes(&wallet, label, &SecretBytes::from_slice(&buf))
                    .expect("put");
                match writer_store.get_bytes(&wallet, label) {
                    Ok(Some(bytes)) => {
                        // Must be one of OUR payloads, never mis-keyed
                        // garbage. Check length + prefix; never log bytes.
                        let got = bytes.expose_secret();
                        assert!(got.starts_with(PREFIX), "garbled get-after-put");
                        assert_eq!(got.len(), PREFIX.len() + 4);
                    }
                    Ok(None) => {}
                    Err(e) => panic!("unexpected get error: {e:?}"),
                }
            }
        });

        let rekeyer = std::thread::spawn(move || {
            // Alternate passphrases so consecutive rekeys change the
            // resident key, keeping the race window real.
            let passphrases = ["pw-A", "pw-B"];
            for i in 0..REKEY_ITERS {
                rekeyer_store
                    .rekey(SecretString::new(passphrases[i % 2]))
                    .expect("rekey");
            }
        });

        writer.join().expect("writer thread");
        rekeyer.join().expect("rekeyer thread");
    }

    /// A bare filename makes `Path::parent()` return `Some("")`, which
    /// `normalized_parent` rewrites to "."; exercise that path in a temp
    /// cwd so nothing lands in the workspace.
    #[test]
    fn open_and_put_with_bare_filename_uses_cwd() {
        // Serialize cwd-changing tests so they cannot race each other.
        static CWD_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _g = CWD_GUARD.lock().unwrap_or_else(|p| p.into_inner());

        let dir = tempfile::tempdir().unwrap();
        // Tighten the cwd-parent so the parent-dir perm check passes (a
        // umask-0002 tempdir is group-writable at 0o775).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        }
        let prior = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        // Tear-down guard so a panic still restores cwd.
        struct CwdReset(PathBuf);
        impl Drop for CwdReset {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }
        let _reset = CwdReset(prior);

        let s =
            EncryptedFileStore::open(Path::new("vault.pwsvault"), SecretString::new("pw-correct"))
                .expect("open with bare filename must succeed");

        entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        assert_eq!(entry(&s, wid(1), "seed").get_secret().unwrap(), b"value");

        // The vault file landed in the cwd-mapped temp dir.
        assert!(dir.path().join("vault.pwsvault").exists());
    }

    /// The lock-sidecar open must refuse a pre-existing symlink
    /// (`O_NOFOLLOW`) so an attacker can't redirect it to another inode.
    #[cfg(unix)]
    #[test]
    fn vault_lock_rejects_symlink_at_lock_path() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let lock = lock_path_for(&path);
        // O_NOFOLLOW makes the open of this symlink fail with ELOOP.
        symlink("/dev/null", &lock).unwrap();

        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("open must refuse to follow lock-path symlink");
        assert!(
            matches!(err, SecretStoreError::Io(_)),
            "expected an Io error from O_NOFOLLOW refusal, got {err:?}"
        );
    }

    /// A group/other-WRITABLE parent is refused at open (it would let a
    /// peer rename/replace the vault despite its 0600); a read-only 0o750
    /// parent is fine.
    #[cfg(unix)]
    #[test]
    fn writable_parent_dir_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("vaultdir");
        fs::create_dir(&sub).unwrap();
        // Group-writable (0o770) trips the write-bit check. Build the path
        // directly — `vault_path` would tighten the dir back to 0o700.
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o770)).unwrap();
        let path = sub.join("vault.pwsvault");
        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("writable parent dir must be refused");
        assert!(
            matches!(err, SecretStoreError::InsecureParentDir { mode } if mode & 0o022 != 0),
            "got {err:?}"
        );
        // Dropping the write bits (still group-readable at 0o750) lets the
        // open succeed: read-only group access is not a rename threat.
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o750)).unwrap();
        let _s = store_at(&path);
    }

    /// An oversized secret is rejected at the write boundary with
    /// `SecretTooLarge`, and the vault stays openable — the per-secret
    /// cap prevents the shared document from being inflated past the
    /// read-side ceiling.
    #[test]
    fn oversized_secret_rejected_and_vault_stays_openable() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "ok").set_secret(b"small").unwrap();
            let too_big = vec![0xABu8; MAX_SECRET_LEN + 1];
            let err = entry(&s, wid(1), "huge").set_secret(&too_big).unwrap_err();
            // Surfaces through the SPI as BadStoreFormat carrying the
            // secret-free SecretTooLarge message.
            assert!(
                matches!(&err, KeyringError::BadStoreFormat(m)
                    if *m == SecretStoreError::SecretTooLarge {
                        found: MAX_SECRET_LEN + 1,
                        max: MAX_SECRET_LEN,
                    }.to_string()),
                "got {err:?}"
            );
            // The earlier good entry is still readable on this handle.
            assert_eq!(entry(&s, wid(1), "ok").get_secret().unwrap(), b"small");
        }
        // The vault reopens cleanly — the oversized put never landed.
        let s2 = store_at(&path);
        assert_eq!(entry(&s2, wid(1), "ok").get_secret().unwrap(), b"small");
        assert!(matches!(
            entry(&s2, wid(1), "huge").get_secret(),
            Err(KeyringError::NoEntry)
        ));
    }

    /// An in-bounds KDF-param shift on a correct-passphrase vault is
    /// rejected at open with `WrongPassphrase` — driven by the changed
    /// DERIVED KEY, not the AAD binding (which `verify_aad_binds_salt_and_
    /// kdf_params` covers).
    #[test]
    fn header_tamper_kdf_shift_smoke_test() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        }
        let mut vault = read_vault_at(&path).unwrap().unwrap();
        // Shift to still-valid-but-weaker params (defaults are 64 MiB/t=3;
        // halving m_kib and dropping t stays above the 19 MiB / t=2 floor).
        vault.kdf.m_kib /= 2;
        vault.kdf.t -= 1;
        assert!(
            vault.kdf.enforce_bounds().is_ok(),
            "shift must stay in bounds"
        );
        write_vault_at(&path, &vault, None).unwrap();
        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("KDF-param shift must fail the verify-token");
        assert!(
            matches!(err, SecretStoreError::WrongPassphrase),
            "got {err:?}"
        );
    }

    /// A flipped salt byte on a correct-passphrase vault is rejected at
    /// open with `WrongPassphrase` — driven by the changed DERIVED KEY
    /// (salt feeds the KDF), not the AAD binding.
    #[test]
    fn header_tamper_flipped_salt_smoke_test() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        }
        let mut vault = read_vault_at(&path).unwrap().unwrap();
        vault.salt[0] ^= 0x01;
        write_vault_at(&path, &vault, None).unwrap();
        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("flipped salt must fail open");
        assert!(
            matches!(err, SecretStoreError::WrongPassphrase),
            "got {err:?}"
        );
    }

    /// A flipped entry NONCE byte (verify-token intact) surfaces as
    /// `Corruption`: the per-entry AEAD-open fails its tag under the
    /// correct key, mirroring the ciphertext-flip route.
    #[test]
    fn flipped_entry_nonce_is_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let s = store_at(&path);
        entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        let mut vault = s.test_read_vault_from_disk().unwrap().unwrap();
        vault
            .wallets
            .get_mut(&wid(1).to_hex())
            .unwrap()
            .get_mut("seed")
            .unwrap()
            .nonce[0] ^= 0x01;
        s.test_write_vault_to_disk(&vault).unwrap();
        s.test_reload_from_disk().unwrap();
        let err = entry(&s, wid(1), "seed").get_secret().unwrap_err();
        assert!(is_corruption(&err), "unexpected error: {err:?}");
    }

    /// A secret exactly at the cap is accepted (boundary is inclusive).
    #[test]
    fn secret_exactly_at_cap_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let s = store_at(&vault_path(dir.path()));
        let at_cap = vec![0x5Au8; MAX_SECRET_LEN];
        entry(&s, wid(1), "atcap").set_secret(&at_cap).unwrap();
        assert_eq!(
            entry(&s, wid(1), "atcap").get_secret().unwrap().len(),
            MAX_SECRET_LEN
        );
    }
}
