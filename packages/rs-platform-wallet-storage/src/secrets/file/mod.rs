//! [`EncryptedFileStore`] — passphrase-encrypted on-disk vault, resident
//! in memory while the store handle lives.
//!
//! # Lifecycle
//!
//! - [`open`] grabs the cross-platform advisory lock on a sibling
//!   `.lock` sidecar (single attempt, no retry), creates a fresh vault
//!   if none exists yet, otherwise decrypts the existing one, and keeps
//!   the plaintext entry map resident.
//! - Every mutation ([`put`], [`delete`], [`rekey`]) edits the in-memory
//!   vault and immediately re-encrypts and atomically writes it back to
//!   disk (eager sync).
//! - [`get`] reads from the in-memory map — no KDF, no disk hit per op.
//! - [`Drop`] best-effort-syncs the resident state once more, re-asserts
//!   `0600` on Unix, and releases the lock when the file descriptor
//!   closes.
//!
//! Concurrency is intentionally not supported: a second `open()` against
//! a path some other store handle (in this or another process) is
//! already holding fails fast with [`SecretStoreError::AlreadyLocked`].
//!
//! One file, one passphrase, one lock — a multi-wallet store cannot
//! lock its other wallets out by construction. The lock sidecar
//! (`<path>.lock`) is distinct from the vault file itself so the atomic
//! `persist` rename never touches the inode an open lock fd points at.
//!
//! [`open`]: EncryptedFileStore::open
//! [`put`]: EncryptedFileStore::put_bytes
//! [`delete`]: EncryptedFileStore::delete_bytes
//! [`rekey`]: EncryptedFileStore::rekey
//! [`get`]: EncryptedFileStore::get_bytes
//!
//! ## Threat coverage
//!
//! Covers **A1** (other local user), **A4** (lost laptop / cold
//! backup), **A6** (synced backup of the vault file): the at-rest file
//! is Argon2id + AEAD, useless without the passphrase. Does **not**
//! cover **A3** (passphrase / derived key resident while unlocked), a
//! weak operator passphrase (KDF raises cost, does not eliminate the
//! risk — an accepted residual), or **A5** if the derived key / plaintext is
//! swapped or core-dumped while unlocked (best-effort mitigated by
//! zeroize + mlock, not eliminated). The derived AEAD key is held
//! resident inside a [`SecretBytes`] for the store's lifetime so reads
//! and writes do not pay the Argon2 cost per op; it is zeroized on Drop.

mod crypto;
mod format;

use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use keyring_core::api::{Credential, CredentialApi, CredentialPersistence, CredentialStoreApi};
use keyring_core::{Entry, Error as KeyringError, Result as KeyringResult};

use crypto::{KdfParams, SALT_LEN};
use format::{EntryBody, Vault};

use super::error::SecretStoreError;

use super::secret::{SecretBytes, SecretString};
use super::validate::{validated_label, WalletId};

/// Upstream service-prefix for vault entries. The full `service`
/// string is `SERVICE_PREFIX + hex(wallet_id)`, mapping each wallet
/// to its own keyring "service" namespace.
pub const SERVICE_PREFIX: &str = "dash.platform-wallet-storage/";

/// Vendor / id tags published through `CredentialStoreApi`.
const VENDOR: &str = "dash.platform-wallet-storage";
const STORE_ID: &str = "encrypted-file-store-v1";

/// Structural ceiling on the on-disk vault file. The vault is
/// attacker-controllable JSON; a multi-GiB file would force a huge
/// `fs::read` allocation ahead of any tag check, so refuse to even
/// allocate beyond this cap and surface
/// [`SecretStoreError::VaultTooLarge`].
pub const MAX_VAULT_SIZE_BYTES: u64 = 128 * 1024 * 1024;

/// A passphrase-encrypted file-backed credential store.
///
/// One file, one passphrase, one lock — the whole store rotates
/// together via [`rekey`](Self::rekey). Every [`SecretString`] and the
/// resident derived AEAD key are zeroized when the store drops.
/// The plaintext entry map is held in
/// [`EntryBody`]-shaped form: the bytes inside `ciphertext` are
/// ciphertext, but the structure is fully populated so reads do not
/// re-touch disk.
///
/// The handle is cheap-`Clone` — both clones share the same
/// `Arc<Mutex<…>>`, so every operation through any clone sees the same
/// resident state and serializes against every other operation.
#[derive(Clone)]
pub struct EncryptedFileStore {
    inner: Arc<Mutex<EncryptedFileStoreInner>>,
}

/// All resident state for the store — path, advisory file lock,
/// in-memory vault, cached AEAD key, and the store-wide passphrase —
/// coalesced behind a single [`Mutex`] at the [`Arc`] wrapper level so
/// every mutation observes a consistent triple (vault matches the key
/// it was sealed under, key matches the passphrase that derived it).
///
/// A single lock keeps put/get/delete/rekey serialized against each
/// other so a concurrent put cannot seal under an old key while rekey
/// is swapping in a new one. The disk write happens while
/// the lock is held; the file-lock sidecar already serializes
/// cross-process so this does not introduce a new I/O contention
/// point.
struct EncryptedFileStoreInner {
    /// Vault file path supplied by the caller at [`open`].
    ///
    /// [`open`]: EncryptedFileStore::open
    path: PathBuf,
    /// In-memory vault. Mutations edit this directly and then call
    /// `sync_to_disk` to re-encrypt and atomically replace the
    /// on-disk file. Reads return clones from here without hitting
    /// disk.
    vault: Vault,
    /// Cached AEAD key derived once at [`open`] from the salt + KDF
    /// params + passphrase. Re-derived only on [`rekey`]. Keeping the
    /// key resident is what makes mutations cheap (one AEAD seal per
    /// entry, no Argon2 per op) and matches the resident-vault model.
    /// A3 (key resident while unlocked) is an accepted threat in the
    /// module docs; the buffer zeroizes when the state drops.
    ///
    /// [`open`]: EncryptedFileStore::open
    /// [`rekey`]: EncryptedFileStore::rekey
    derived_key: SecretBytes,
    /// The store-wide passphrase. Swapped atomically with `vault` and
    /// `derived_key` under the same lock during [`rekey`].
    ///
    /// [`rekey`]: EncryptedFileStore::rekey
    passphrase: SecretString,
    /// Holds the cross-platform advisory write-lock on `<path>.lock`
    /// for the entire lifetime of the store. Dropped (releasing the
    /// flock / LockFileEx) when the store drops.
    _lock: VaultLock,
}

impl EncryptedFileStore {
    /// Open a vault store at `path`, unlocked by `passphrase`. `path`
    /// is the vault FILE, not a directory — the operator picks the
    /// filename.
    ///
    /// The call acquires an exclusive advisory lock on a sibling
    /// `<path>.lock` sidecar before touching the vault. If the lock is
    /// already held (by another handle in this process or by another
    /// process) the call returns [`SecretStoreError::AlreadyLocked`]
    /// immediately — there is no retry loop.
    ///
    /// If `path` does not exist yet a fresh vault (random salt, default
    /// Argon2 params, sealed verify token, no entries) is created at
    /// `0600` on Unix. If it exists the vault is read, the passphrase
    /// is verified against the header verify-token, and the plaintext
    /// entry map is loaded into memory. Either way the returned store
    /// is immediately usable.
    pub fn open(
        path: impl AsRef<Path>,
        passphrase: SecretString,
    ) -> Result<Self, SecretStoreError> {
        let path = path.as_ref().to_path_buf();

        // Make sure the parent directory exists so both the lock sidecar
        // open and the vault create do not fail on a not-yet-materialized
        // dir (canonical for first-setup operators). `Path::parent()`
        // returns `Some("")` for a bare relative filename, which neither
        // `create_dir_all` nor the cross-platform persist path can
        // consume — normalize the empty-string parent to ".".
        let parent = normalized_parent(&path);
        create_parent_dir(parent)?;

        // Acquire the lock first — every subsequent step assumes
        // exclusive ownership of the vault file.
        let lock = VaultLock::acquire(&lock_path_for(&path))?;

        // Decide between load-existing and create-fresh based on a
        // single open attempt: NotFound → fresh; anything else → load
        // (the perm check inside `read_existing_vault` covers loose
        // perms on a real file).
        let (vault, derived_key) = match Self::load_existing_vault(&path, &passphrase)? {
            Some(loaded) => loaded,
            None => Self::create_new_vault(&path, &passphrase)?,
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(EncryptedFileStoreInner {
                path,
                vault,
                derived_key,
                passphrase,
                _lock: lock,
            })),
        })
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

    /// Build a brand-new empty vault, persist it at `0600`, and return
    /// the in-memory state + derived key.
    fn create_new_vault(
        path: &Path,
        passphrase: &SecretString,
    ) -> Result<(Vault, SecretBytes), SecretStoreError> {
        let (vault, key) = build_fresh_vault(passphrase)?;
        write_vault_at(path, &vault)?;
        Ok((vault, key))
    }

    /// Re-encrypt the whole store under `new_passphrase`: fresh salt +
    /// fresh per-entry nonces for every wallet's entries, then
    /// atomically replace the vault file. No `.bak` retains old key
    /// material. The swap is whole-store: every
    /// wallet's entries are re-keyed in one shot, so the store cannot
    /// end up half-rotated. The in-memory vault, derived key, and
    /// passphrase advance together under the resident-state mutex.
    ///
    /// The fresh KDF / Argon2 derivation runs OUTSIDE the lock — it
    /// only touches the new passphrase + a fresh salt and never reads
    /// resident state, so paying ~hundreds of ms inside the critical
    /// section would just stall unrelated put/get operations.
    pub fn rekey(&self, new_passphrase: SecretString) -> Result<(), SecretStoreError> {
        let (new_vault, new_key) = build_fresh_vault(&new_passphrase)?;
        lock_inner(&self.inner).rekey(new_vault, new_key, new_passphrase)
    }

    /// Store `secret` under `(wallet_id, label)`, returning the typed
    /// [`SecretStoreError`] (lossless — no `keyring_core::Error` seam).
    /// The public [`SecretStore`](crate::secrets::SecretStore) file
    /// arm delegates here so the structural error distinction
    /// survives. Symmetric with [`get_bytes`]: the secret stays
    /// wrapped in [`SecretBytes`] across this seam; the lone bare-buffer
    /// exposure lives one layer down at the AEAD seal call.
    ///
    /// [`get_bytes`]: Self::get_bytes
    pub(crate) fn put_bytes(
        &self,
        wallet_id: &WalletId,
        label: &str,
        secret: &SecretBytes,
    ) -> Result<(), SecretStoreError> {
        lock_inner(&self.inner).put(wallet_id, label, secret)
    }

    /// Retrieve the plaintext under `(wallet_id, label)`, or `None` if
    /// absent, returning the typed [`SecretStoreError`]. The plaintext
    /// stays inside a zeroizing [`SecretBytes`] all the way to this
    /// boundary; the single `.expose_secret().to_vec()` conversion lives
    /// at the upstream `CredentialApi::get_secret`
    /// SPI seam, the only point where the SPI contract demands a bare
    /// `Vec<u8>`.
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

    #[cfg(test)]
    pub(crate) fn test_read_vault_from_disk(&self) -> Result<Option<Vault>, SecretStoreError> {
        read_vault_at(&lock_inner(&self.inner).path)
    }

    #[cfg(test)]
    pub(crate) fn test_write_vault_to_disk(&self, vault: &Vault) -> Result<(), SecretStoreError> {
        write_vault_at(&lock_inner(&self.inner).path, vault)
    }

    /// Drop the in-memory copy of the vault and reload it from disk
    /// under the current passphrase. Useful for tests that mutate the
    /// on-disk file out from under the store and want subsequent reads
    /// to observe the new bytes (the resident-vault model otherwise
    /// caches the loaded state).
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

/// Acquire the single coarse-grained state lock on `inner`.
/// Poisoned-mutex recovery is "log and continue": a previously-panicked
/// holder cannot have left the [`EncryptedFileStoreInner`] half-written
/// (every mutation either succeeds wholesale and writes to disk or
/// reverts), so the inner value is safe to keep using.
fn lock_inner(
    inner: &Arc<Mutex<EncryptedFileStoreInner>>,
) -> std::sync::MutexGuard<'_, EncryptedFileStoreInner> {
    inner.lock().unwrap_or_else(|p| p.into_inner())
}

impl EncryptedFileStoreInner {
    /// Re-encrypt the resident vault and atomically replace the
    /// on-disk file. Runs inside the state-lock critical section.
    fn sync_to_disk(&self) -> Result<(), SecretStoreError> {
        write_vault_at(&self.path, &self.vault)
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
        let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), &label);

        let (nonce, ciphertext) = crypto::seal(&self.derived_key, &aad, secret.expose_secret())?;

        // Mutate in memory; remember the prior body so we can roll
        // back on a disk-write failure (the resident state must
        // always match what is on disk after a returned-Ok mutation).
        let prior = {
            let entries = self.vault.wallets.entry(wallet_id.to_hex()).or_default();
            entries.insert(label.clone(), EntryBody { nonce, ciphertext })
        };

        if let Err(e) = self.sync_to_disk() {
            let entries = self
                .vault
                .wallets
                .get_mut(&wallet_id.to_hex())
                .expect("entry just inserted");
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

        // Stage the new triple in memory, write to disk, and on
        // failure restore the old triple so the live handle keeps
        // serving under the still-on-disk key.
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
        // Belt-and-suspenders sync of resident state. Eager-sync on
        // every mutation makes this redundant in the success path, but
        // a final write lets a future feature (e.g. opportunistic
        // background buffering) hang off the same Drop without changing
        // the contract. `&mut self` here implies unique ownership —
        // the outer `Mutex` is being dropped too, so no other holder
        // can be waiting.
        if let Err(e) = self.sync_to_disk() {
            tracing::warn!(error = %e, "drop-time vault sync failed");
        }
        // Re-assert restrictive perms on Unix. Between writes the file
        // is already 0600, but this defends against a peer that
        // loosened them through some other path while we held the
        // lock. Best-effort: any failure is non-fatal at Drop.
        #[cfg(unix)]
        if let Ok(file) = open_no_follow(&self.path) {
            if let Err(e) = set_restrictive_perms(&file) {
                tracing::warn!(error = %e, "drop-time perm re-assert failed");
            }
        }
        // The `VaultLock` field drops naturally after this method
        // returns, releasing the OS advisory lock.
    }
}

/// Sidecar advisory-lock path for the store's vault file. Kept
/// distinct from the vault file itself so the cross-platform
/// `persist` swap never touches the inode an open lock fd points
/// at — the lock fd remains valid across the atomic replace.
fn lock_path_for(path: &Path) -> PathBuf {
    let mut s = path.to_path_buf().into_os_string();
    s.push(".lock");
    PathBuf::from(s)
}

/// Build a fresh vault skeleton: random salt, default Argon2
/// params, and a passphrase-verification token sealed under the
/// freshly derived key (the token is the mixed-key-corruption guard).
/// Returns the (entry-less) vault and the
/// derived key so the caller can seal entries against it without
/// re-deriving.
fn build_fresh_vault(passphrase: &SecretString) -> Result<(Vault, SecretBytes), SecretStoreError> {
    let mut salt = [0u8; SALT_LEN];
    crypto::random_bytes(&mut salt)?;
    let kdf = KdfParams::default_target();
    let key = crypto::derive_key(passphrase, &salt, kdf)?;
    let v_aad = format::verify_aad(format::FORMAT_VERSION);
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
    let v_aad = format::verify_aad(format::FORMAT_VERSION);
    match crypto::open(&key, &vault.verify_nonce, &v_aad, &vault.verify_ct) {
        Ok(_) => Ok(key),
        Err(SecretStoreError::Decrypt) => Err(SecretStoreError::WrongPassphrase),
        Err(e) => Err(e),
    }
}

/// Read + parse the vault at `path`, or `None` if it does not exist.
/// Refuses a pre-existing file with looser-than-0600 perms and a file
/// exceeding [`MAX_VAULT_SIZE_BYTES`].
///
/// Eliminates the metadata→read TOCTOU: opens the file once with
/// `O_NOFOLLOW` on Unix, then derives perms / size from
/// the open handle's `metadata()` and reads from the same fd.
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

/// Atomically replace the vault at `path`, cross-platform.
///
/// Stages into a `NamedTempFile` in the SAME directory (so `persist`
/// cannot fail cross-volume), tightens perms to 0600 on Unix before
/// any byte is written, then: `write_all` → `sync_all` →
/// `persist(path)` → Unix parent-dir fsync. The destination is never
/// pre-removed, so a crash leaves either the old or the new vault,
/// never an absent one. On `persist` failure the temp drops and
/// self-cleans — no manual remove racing it. The temp holds only
/// ciphertext+header, never plaintext.
fn write_vault_at(path: &Path, vault: &Vault) -> Result<(), SecretStoreError> {
    do_write_vault_at(path, vault).inspect_err(|e| {
        tracing::warn!(error = %e, "failed to write vault file");
    })
}

fn do_write_vault_at(path: &Path, vault: &Vault) -> Result<(), SecretStoreError> {
    let serialized = format::serialize(vault);
    // Normalize an empty / bare-filename parent to "." so neither
    // `NamedTempFile::new_in` nor the Unix parent-dir fsync sees an
    // empty path.
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
    #[cfg(unix)]
    {
        let d = fs::File::open(parent).map_err(|e| SecretStoreError::io_at(parent, e))?;
        d.sync_all()
            .map_err(|e| SecretStoreError::io_at(parent, e))?;
    }
    Ok(())
}

/// Normalize `path.parent()` for callers that need a directory path
/// they can pass to `fs::create_dir_all`, `NamedTempFile::new_in`, and
/// the Unix parent-dir fsync. `Path::parent()` returns `Some("")` for a
/// bare relative filename like `"vault.pwsvault"`, and the empty path
/// errors out at every one of those calls — normalize to "." so a
/// caller that supplies a bare filename in their cwd just works.
fn normalized_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

/// Create the parent directory for a vault file, applying a `0700` mode
/// on Unix so the directory created at first-setup is not
/// world-readable. Idempotent: a pre-existing directory is left alone.
fn create_parent_dir(parent: &Path) -> Result<(), SecretStoreError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(parent)?;
    }
    // INTENTIONAL: Windows ACL hardening on the parent dir is deferred
    // to https://github.com/dashpay/platform/issues/3754. The recursive
    // create still runs so the path materializes; operators on Windows
    // MUST tighten ACLs manually until the follow-up lands.
    #[cfg(not(unix))]
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Cross-platform advisory write-lock holder. Owns a `Box<RwLock<File>>`
/// (so the address is stable) and an owned `RwLockWriteGuard` borrowing
/// from it. Dropping the holder drops the guard first (which releases
/// the OS lock via `fd-lock`'s Drop impl, calling `flock(LOCK_UN)` on
/// Unix and `UnlockFileEx` on Windows) and then frees the heap-pinned
/// `RwLock`.
///
/// The self-reference is unavoidable: `fd-lock`'s guard borrows the
/// `RwLock`, and the resident-vault model requires the lock to stay
/// held continuously between `open` and `Drop`. Wrapped in a small
/// allow-unsafe island so the rest of the crate keeps
/// `deny(unsafe_code)`. Safety arguments:
///
/// 1. The `RwLock<File>` lives on the heap via `Box::into_raw`, so its
///    address is stable for the holder's lifetime.
/// 2. The `'static` lifetime on the guard is a lie tolerated only
///    because the guard never outlives the holder, and the holder's
///    `Drop` impl takes the guard out (running its Drop) *before*
///    reclaiming the box.
/// 3. The raw pointer never escapes this module.
mod vault_lock {
    #![allow(unsafe_code)]

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
    unsafe impl Send for VaultLock {}
    unsafe impl Sync for VaultLock {}

    impl VaultLock {
        pub(super) fn acquire(lock_path: &Path) -> Result<Self, SecretStoreError> {
            // INTENTIONAL: on non-unix platforms the symlink-following
            // hardening is deferred to
            // https://github.com/dashpay/platform/issues/3754 — Windows
            // requires `FILE_FLAG_OPEN_REPARSE_POINT` via the raw API
            // and is out of scope for the secrets-feature landing.
            let mut opts = fs::OpenOptions::new();
            opts.read(true).write(true).create(true).truncate(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.custom_flags(libc::O_NOFOLLOW);
            }
            let lock_file = opts
                .open(lock_path)
                .map_err(|e| SecretStoreError::io_at(lock_path, e))?;
            #[cfg(unix)]
            set_restrictive_perms(&lock_file)?;

            let raw: *mut fd_lock::RwLock<fs::File> =
                Box::into_raw(Box::new(fd_lock::RwLock::new(lock_file)));

            // SAFETY: `raw` came straight from `Box::into_raw` of a
            // fresh box; it is non-null, properly aligned, and points
            // at a valid `RwLock<File>`. No other reference exists
            // yet, so promoting it to `&'static mut` is sound for the
            // borrow we hand to `try_write`.
            let static_ref: &'static mut fd_lock::RwLock<fs::File> = unsafe { &mut *raw };

            let guard = match static_ref.try_write() {
                Ok(guard) => guard,
                Err(e) => {
                    // SAFETY: the guard never came into existence, so
                    // no live borrow points at the box; reclaiming
                    // here is sound and avoids leaking on the error
                    // path.
                    unsafe { drop(Box::from_raw(raw)) };
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
            unsafe { drop(Box::from_raw(self.rwlock)) };
        }
    }
}

use vault_lock::VaultLock;

/// Why a wallet-id hex string failed the canonical-form check.
///
/// `WalletId::to_hex` only ever emits 64 lowercase hex chars, so every
/// seam that parses a wallet id back enforces exactly that shape in one
/// place via [`wallet_id_hex_to_bytes`]; this enum lets each caller map
/// the reason onto its own error type with the right message.
enum WalletIdHexError {
    /// Not exactly 64 characters.
    WrongLength,
    /// Contains uppercase hex digits — off-canonical.
    Uppercase,
    /// Not valid hexadecimal.
    NotHex,
}

/// Decode a 64-lowercase-hex-char wallet id into its 32 bytes, enforcing
/// the canonical form `WalletId::to_hex` writes (64 chars, lowercase).
/// The single seam both the on-disk outer-key check and the SPI
/// service-string parse go through so the contract lives in one place.
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

/// Decode a wallet-id hex string (the on-disk outer key) into the
/// 32-byte form the AAD construction expects. A malformed key here is
/// an on-disk integrity failure — the format-layer parse already
/// constrains entries to JSON object semantics, but the outer key is
/// a free-form string at the type level, so the bytes-back check is a
/// defence-in-depth structural guard. Off-canonical (uppercase / wrong
/// length / non-hex) keys are all rejected as corruption.
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

/// A `(wallet_id, label)` row in an [`EncryptedFileStore`].
///
/// Holds a [`Clone`]d handle to the parent store so each credential
/// goes through the same public store API (and the same single-lock
/// critical section per operation). All four operations re-validate
/// `user` (label); the store key is resident on the inner so a
/// wrong-passphrase race cannot happen at the credential layer — the
/// open already failed if the passphrase was wrong.
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
        // `try_lock` rather than `lock_inner` so a Debug invoked from
        // a panic path while the same thread already holds the state
        // lock cannot deadlock or double-panic. Poison is folded into
        // the same fallback display as contention.
        let path: PathBuf = match self.inner.try_lock() {
            Ok(guard) => guard.path.clone(),
            Err(_) => PathBuf::from("<unavailable>"),
        };
        f.debug_struct("EncryptedFileStore")
            .field("path", &path)
            .finish_non_exhaustive()
    }
}

/// Project an entry-level `crypto::open` result into the typed
/// distinction the secret backend exposes. The verify-token has already
/// passed at every caller (open / get / rekey), so a
/// `SecretStoreError::Decrypt` here is corruption or tampering of the
/// individual entry — **not** a wrong passphrase. Logs the non-secret
/// `(wallet_id, label)` pair at error level (never the secret) and
/// maps to `SecretStoreError::Corruption`. Every other variant rides
/// through unchanged.
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

// INTENTIONAL: Windows ACL read-check deferred to a follow-up PR —
// tracked at https://github.com/dashpay/platform/issues/3754. Vault
// file mode hardening on Windows requires GetSecurityInfo via
// `windows-acl` or `winapi`; out of scope for the secrets-feature
// landing. Operators on Windows MUST set ACLs manually until the
// follow-up lands.
#[cfg(not(unix))]
fn check_perms(_meta: &fs::Metadata) -> Result<(), SecretStoreError> {
    Ok(())
}

#[cfg(unix)]
fn set_restrictive_perms(f: &fs::File) -> Result<(), SecretStoreError> {
    use std::os::unix::fs::PermissionsExt;
    f.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(())
}

// INTENTIONAL: Windows ACL tightening deferred to the same follow-up
// as `check_perms` above — tracked at
// https://github.com/dashpay/platform/issues/3754. Vault file mode
// hardening on Windows requires SetSecurityInfo via `windows-acl` or
// `winapi`; out of scope for the secrets-feature landing. Operators on
// Windows MUST set ACLs manually until the follow-up lands.
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
        // Resident-vault model: open() creates a usable vault file even
        // without any subsequent put, so a second open() of the same
        // path observes a real on-disk file (modulo the lock).
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
        // Resident-vault model: a second open() of the same path while
        // the first store is alive returns AlreadyLocked immediately
        // (no retry, no wait). Once the first store drops the lock is
        // released and a fresh open() succeeds.
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
        // A vault whose JSON declares m_kib = u32::MAX must be refused
        // at open() with KdfFailure — before the verify-token is
        // derived and without the ~4 TiB allocation the inflated param
        // would demand. Under the resident-vault model this surfaces at
        // open() rather than on first get(). Drop the store BEFORE
        // patching the on-disk file so the drop-time sync cannot
        // overwrite our injected corruption.
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        {
            let s = store_at(&path);
            entry(&s, wid(1), "seed").set_secret(b"value").unwrap();
        }
        let mut vault = read_vault_at(&path).unwrap().unwrap();
        vault.kdf.m_kib = u32::MAX;
        write_vault_at(&path, &vault).unwrap();
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

    /// Two threads share a cloned [`EncryptedFileStore`] handle (same
    /// shape [`EncryptedFileCredential`] uses internally): one hammers
    /// `put_bytes` + `get_bytes`, the other calls `rekey`. The single
    /// state lock serializes every put/get/rekey, so a put can never
    /// capture an old key and insert under a newly-swapped vault — every
    /// `get` returns either the right plaintext, `Ok(None)`, or a clean
    /// typed error, NEVER garbled bytes from a mis-keyed seal (which
    /// would surface as `Corruption`).
    #[test]
    fn rekey_does_not_race_put_into_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let store = store_at(&path);
        let writer_store = store.clone();
        let rekeyer_store = store.clone();

        // Iteration counts are tuned for cost: every rekey runs Argon2
        // at the default-target params, so the rekey loop dominates
        // wall-clock. 16 rekeys overlap a 200-iter put loop reliably
        // enough to hit the pre-fix race window on the test runner
        // without dragging the suite out.
        const PUT_ITERS: usize = 200;
        const REKEY_ITERS: usize = 16;
        let wallet = wid(7);
        let label = "racy";

        // A fixed-prefix payload byte vector — never built with
        // `format!` so the in-source secrets-guard scanner does not
        // flag this test as a sink/expose_secret pairing.
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
                        // Must be one of OUR payloads — never random
                        // bytes from a mis-keyed seal. Compare only
                        // length + prefix; never log the bytes.
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
            // Alternate two passphrases so consecutive rekeys actually
            // change the resident key (the salt rerolls regardless, but
            // alternating distinct passphrases is the operator-facing
            // model and keeps the race window real).
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

    /// A bare relative filename makes `Path::parent()` return `Some("")`,
    /// which `NamedTempFile::new_in("")` and the Unix parent-dir fsync
    /// both reject; the `normalized_parent` helper rewrites the empty
    /// parent to ".". Switch cwd to a temp dir for the test scope so we
    /// exercise the bare-filename path without scribbling in the
    /// workspace.
    #[test]
    fn open_and_put_with_bare_filename_uses_cwd() {
        // A static mutex serializes cwd-changing tests so they cannot
        // race each other across the suite.
        static CWD_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _g = CWD_GUARD.lock().unwrap_or_else(|p| p.into_inner());

        let dir = tempfile::tempdir().unwrap();
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

    /// The lock sidecar must refuse to traverse a pre-existing symlink
    /// at the lock path on Unix. Without `O_NOFOLLOW` an attacker could
    /// redirect the lock file's open to an unrelated inode.
    #[cfg(unix)]
    #[test]
    fn vault_lock_rejects_symlink_at_lock_path() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let path = vault_path(dir.path());
        let lock = lock_path_for(&path);
        // Point the lock path at /dev/null. Any successful open of the
        // symlink would land on /dev/null's inode; O_NOFOLLOW makes the
        // open itself fail with ELOOP.
        symlink("/dev/null", &lock).unwrap();

        let err = EncryptedFileStore::open(&path, SecretString::new("pw-correct"))
            .expect_err("open must refuse to follow lock-path symlink");
        assert!(
            matches!(err, SecretStoreError::Io(_)),
            "expected an Io error from O_NOFOLLOW refusal, got {err:?}"
        );
    }
}
