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
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use keyring_core::api::{Credential, CredentialApi, CredentialPersistence, CredentialStoreApi};
use keyring_core::{Entry, Error as KeyringError, Result as KeyringResult};

use crypto::{KdfParams, SALT_LEN};
use error::FileStoreError;
use format::{EntryBody, Vault};

use super::secret::{SecretBytes, SecretString};
use super::validate::{validated_label, WalletId};

/// Upstream service-prefix for vault entries. The full `service`
/// string is `SERVICE_PREFIX + hex(wallet_id)`, mapping each wallet
/// to its own keyring "service" namespace.
pub const SERVICE_PREFIX: &str = "dash.platform-wallet-storage/";

/// Vendor / id tags published through `CredentialStoreApi`.
const VENDOR: &str = "dash.platform-wallet-storage";
const STORE_ID: &str = "encrypted-file-store-v1";

/// Structural ceiling on the on-disk vault file (CMT-003). The vault is
/// attacker-controllable JSON; a multi-GiB file would force a huge
/// `fs::read` allocation ahead of any tag check, so refuse to even
/// allocate beyond this cap and surface
/// [`FileStoreError::VaultTooLarge`].
pub const MAX_VAULT_SIZE_BYTES: u64 = 128 * 1024 * 1024;

/// Wall-clock budget for the cross-process advisory lock acquired
/// around every vault RMW (CMT-001). Picked well above any single
/// vault write's natural duration so honest contention always wins,
/// but bounded so a stuck peer fails fast as [`FileStoreError::Busy`]
/// instead of hanging the caller indefinitely.
const LOCK_WAIT_BUDGET: Duration = Duration::from_secs(5);

/// Poll cadence inside [`LOCK_WAIT_BUDGET`]. Short enough that the
/// release of a contending peer is observed promptly; long enough that
/// a busy retry loop costs no CPU worth noticing.
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(50);

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
    /// Per-wallet in-process serialization for the put/delete/rekey
    /// read-modify-write span (CMT-001). The outer `Mutex` only guards
    /// the map; the inner per-wallet `Mutex<()>` is held across the
    /// whole RMW. Different wallets stay parallel; same-wallet ops are
    /// strictly serial. Composed with the cross-process advisory lock
    /// in [`with_vault_lock`].
    locks: Mutex<HashMap<WalletId, Arc<Mutex<()>>>>,
}

impl EncryptedFileStore {
    /// Open (or prepare to create) a vault store rooted at `dir`,
    /// unlocked by `passphrase`. `dir` is created if missing. On Unix
    /// the directory is tightened to `0700`; a pre-existing dir whose
    /// perms were looser is logged at warn level and then tightened in
    /// place (CMT-002), so the operator sees the prior exposure but
    /// canonical bootstraps (umask 022 `mkdir`, `tempfile::tempdir`)
    /// still work. A post-tighten mode that is not `0700` surfaces
    /// [`FileStoreError::InsecurePermissions`].
    pub fn open(dir: impl AsRef<Path>, passphrase: SecretString) -> Result<Self, FileStoreError> {
        let dir = dir.as_ref().to_path_buf();
        let preexisted = dir.exists();
        fs::create_dir_all(&dir)?;
        set_restrictive_dir_perms(&dir, preexisted)?;
        Ok(Self {
            inner: Arc::new(EncryptedFileStoreInner {
                dir,
                passphrase,
                locks: Mutex::new(HashMap::new()),
            }),
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

    /// Store `secret` under `(wallet_id, label)`, returning the typed
    /// [`FileStoreError`] (lossless — no `keyring_core::Error` seam).
    /// The public [`SecretStore`](crate::secrets::SecretStore) file
    /// arm delegates here so the structural error distinction
    /// survives. Symmetric with [`get_bytes`]: the secret stays
    /// wrapped in [`SecretBytes`] across this seam (CMT-009); the lone
    /// bare-buffer exposure lives one layer down at the AEAD seal call.
    ///
    /// [`get_bytes`]: Self::get_bytes
    pub(crate) fn put_bytes(
        &self,
        wallet_id: &WalletId,
        label: &str,
        secret: &SecretBytes,
    ) -> Result<(), FileStoreError> {
        self.inner.put(wallet_id, label, secret)
    }

    /// Retrieve the plaintext under `(wallet_id, label)`, or `None` if
    /// absent, returning the typed [`FileStoreError`]. The plaintext
    /// stays inside a zeroizing [`SecretBytes`] all the way to this
    /// boundary (CMT-008); the single `.expose_secret().to_vec()`
    /// conversion lives at the upstream `CredentialApi::get_secret`
    /// SPI seam, the only point where the SPI contract demands a bare
    /// `Vec<u8>`.
    pub(crate) fn get_bytes(
        &self,
        wallet_id: &WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, FileStoreError> {
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
    pub(crate) fn test_read_vault(&self, path: &Path) -> Result<Option<Vault>, FileStoreError> {
        self.inner.read_vault(path)
    }

    #[cfg(test)]
    pub(crate) fn test_write_vault(
        &self,
        path: &Path,
        vault: &Vault,
    ) -> Result<(), FileStoreError> {
        self.inner.write_vault(path, vault)
    }
}

impl EncryptedFileStoreInner {
    fn vault_path(&self, wallet_id: &WalletId) -> PathBuf {
        self.dir.join(format!("{}.pwsvault", wallet_id.to_hex()))
    }

    /// Sidecar advisory-lock path next to a vault. Held across the
    /// whole put/delete/rekey RMW so a concurrent peer cannot read,
    /// re-encrypt, and write over our pending swap (CMT-001). Kept
    /// distinct from the vault file itself so the cross-platform
    /// `persist` swap never touches the file an open lock fd points at.
    fn vault_lock_path(&self, wallet_id: &WalletId) -> PathBuf {
        self.dir
            .join(format!("{}.pwsvault.lock", wallet_id.to_hex()))
    }

    /// Per-wallet in-process mutex (lazily inserted). The map mutex is
    /// only held for the lookup/insert; the returned `Arc<Mutex<()>>`
    /// is what the caller holds across the RMW.
    fn wallet_mutex(&self, wallet_id: &WalletId) -> Arc<Mutex<()>> {
        let mut map = self
            .locks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        map.entry(*wallet_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Run `f` with the in-process per-wallet mutex held AND the
    /// cross-process sidecar `.lock` file held exclusively (CMT-001).
    /// Both layers acquire-release strictly around `f`'s execution; the
    /// sidecar fd is opened under the in-process mutex so two threads
    /// in this process cannot fight over the same fd. Lock contention
    /// past [`LOCK_WAIT_BUDGET`] surfaces as
    /// [`FileStoreError::Busy`].
    fn with_vault_lock<R>(
        &self,
        wallet_id: &WalletId,
        f: impl FnOnce() -> Result<R, FileStoreError>,
    ) -> Result<R, FileStoreError> {
        let mutex = self.wallet_mutex(wallet_id);
        let _guard = mutex.lock().unwrap_or_else(|poison| poison.into_inner());

        let lock_path = self.vault_lock_path(wallet_id);
        let lock_file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        #[cfg(unix)]
        set_restrictive_perms(&lock_file)?;

        let mut rw = fd_lock::RwLock::new(lock_file);
        let deadline = Instant::now() + LOCK_WAIT_BUDGET;
        let _file_guard = loop {
            match rw.try_write() {
                Ok(guard) => break guard,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(LOCK_POLL_INTERVAL);
                }
                Err(_) => return Err(FileStoreError::Busy),
            }
        };
        f()
    }

    /// Build a fresh vault skeleton for a brand-new wallet: random salt,
    /// default Argon2 params, and a passphrase-verification token sealed
    /// under the freshly derived key (SEC-REQ-2.2.x; the token is the
    /// mixed-key-corruption guard). Returns the (entry-less) vault and
    /// the derived key so the caller can seal entries against it without
    /// re-deriving.
    fn new_vault(
        &self,
        wallet_id: &WalletId,
        passphrase: &SecretString,
    ) -> Result<(Vault, SecretBytes), FileStoreError> {
        let mut salt = [0u8; SALT_LEN];
        crypto::random_bytes(&mut salt)?;
        let kdf = KdfParams::default_target();
        let key = crypto::derive_key(passphrase, &salt, kdf)?;
        let v_aad = format::verify_aad(format::FORMAT_VERSION, wallet_id.as_bytes());
        let (verify_nonce, verify_ct) = crypto::seal(&key, &v_aad, format::VERIFY_CONSTANT)?;
        Ok((
            Vault {
                version: format::FORMAT_VERSION,
                kdf,
                salt,
                verify_nonce,
                verify_ct,
                entries: std::collections::BTreeMap::new(),
            },
            key,
        ))
    }

    /// Derive the key from the supplied passphrase and verify it
    /// against the vault's token *before* any entry is touched. A
    /// wrong passphrase fails the token's AEAD tag (constant-time) and
    /// yields `WrongPassphrase` with no plaintext, so a mismatched key is
    /// rejected before any entry is touched (SEC-REQ-2.2.x).
    fn derive_and_verify(
        &self,
        wallet_id: &WalletId,
        vault: &Vault,
    ) -> Result<SecretBytes, FileStoreError> {
        let key = crypto::derive_key(&self.passphrase, &vault.salt, vault.kdf)?;
        let v_aad = format::verify_aad(format::FORMAT_VERSION, wallet_id.as_bytes());
        match crypto::open(&key, &vault.verify_nonce, &v_aad, &vault.verify_ct) {
            Ok(_) => Ok(key),
            Err(FileStoreError::Decrypt) => Err(FileStoreError::WrongPassphrase),
            Err(e) => Err(e),
        }
    }

    /// Read + parse a vault file, or `None` if it does not exist.
    /// Refuses a pre-existing file with looser-than-0600 perms
    /// (SEC-REQ-2.2.10) and a file exceeding [`MAX_VAULT_SIZE_BYTES`]
    /// (CMT-003).
    ///
    /// Eliminates the metadata→read TOCTOU (CMT-004): opens the file
    /// once with `O_NOFOLLOW` on Unix, then derives perms / size from
    /// the open handle's `metadata()` and reads from the same fd. A
    /// symlink swap during the window now reads the original inode the
    /// open captured.
    fn read_vault(&self, path: &Path) -> Result<Option<Vault>, FileStoreError> {
        let file = match open_no_follow(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let meta = file.metadata()?;
        check_perms(&meta)?;
        let len = meta.len();
        if len > MAX_VAULT_SIZE_BYTES {
            return Err(FileStoreError::VaultTooLarge {
                found: len,
                max: MAX_VAULT_SIZE_BYTES,
            });
        }
        let mut bytes = Vec::with_capacity(len as usize);
        let mut handle = file.take(MAX_VAULT_SIZE_BYTES + 1);
        handle.read_to_end(&mut bytes)?;
        // A racing writer that grew the file past the cap between the
        // metadata check and the read also has to lose the structural
        // limit — `Read::take` caps the byte count above, but a peer
        // that resized in-place could still feed us up to that cap; the
        // explicit re-check guards a 0-padded grow that snuck under the
        // metadata snapshot.
        if bytes.len() as u64 > MAX_VAULT_SIZE_BYTES {
            return Err(FileStoreError::VaultTooLarge {
                found: bytes.len() as u64,
                max: MAX_VAULT_SIZE_BYTES,
            });
        }
        Ok(Some(format::deserialize(&bytes)?))
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
    fn write_vault(&self, path: &Path, vault: &Vault) -> Result<(), FileStoreError> {
        self.do_write_vault(path, vault).inspect_err(|e| {
            // Operators must see a failed durable write — paths are
            // caller-supplied non-secret (FileStoreError::Io doc); Display
            // only, never the secret.
            tracing::warn!(error = %e, "failed to write vault file");
        })
    }

    /// Inner write — separated from [`write_vault`] (CMT-012) so the
    /// warn-on-error rendering hangs off a single `inspect_err` on a
    /// real method call instead of an immediately-invoked closure.
    /// Same atomicity contract as [`write_vault`].
    fn do_write_vault(&self, path: &Path, vault: &Vault) -> Result<(), FileStoreError> {
        let serialized = format::serialize(vault);
        // `persist` is atomic-replace only within one filesystem, so the
        // temp MUST share the destination's parent dir (mirrors
        // sqlite/backup.rs).
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
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
    }

    fn rekey(
        &mut self,
        wallet_id: WalletId,
        new_passphrase: SecretString,
    ) -> Result<(), FileStoreError> {
        // The `&mut self` arrival gates in-process races (the outer
        // `EncryptedFileStore::rekey` proves exclusive `Arc` ownership
        // via `Arc::get_mut`). The cross-process advisory lock added in
        // CMT-001 guards the read→re-encrypt→write span against a peer
        // process; `with_vault_lock` also takes the in-process per-
        // wallet mutex so a future refactor that loses the `&mut self`
        // channel cannot silently regress the safety.
        let path = self.vault_path(&wallet_id);
        self.with_vault_lock(&wallet_id, || {
            let Some(old_vault) = self.read_vault(&path)? else {
                // No vault on disk yet: the new passphrase becomes the
                // active one for any future write (set below the lock).
                return Ok(());
            };
            let old_key = self.derive_and_verify(&wallet_id, &old_vault)?;
            let (mut new_vault, new_key) = self.new_vault(&wallet_id, &new_passphrase)?;

            for (label, body) in &old_vault.entries {
                let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), label);
                let pt = entry_decrypt_or_corruption(
                    &wallet_id,
                    label,
                    crypto::open(&old_key, &body.nonce, &aad, &body.ciphertext),
                )?;
                let (nonce, ct) = crypto::seal(&new_key, &aad, pt.expose_secret())?;
                new_vault.entries.insert(
                    label.clone(),
                    EntryBody {
                        nonce,
                        ciphertext: ct,
                    },
                );
            }
            self.write_vault(&path, &new_vault)
        })?;
        self.passphrase = new_passphrase;
        Ok(())
    }

    /// `put` — overwrite-safe atomic seal under `(wallet_id, label)`.
    /// The read-modify-write span is serialized in-process and
    /// cross-process via [`with_vault_lock`] (CMT-001). Takes
    /// `&SecretBytes` so the bare plaintext view exists only inside
    /// the `crypto::seal` call (CMT-009).
    ///
    /// [`with_vault_lock`]: Self::with_vault_lock
    fn put(
        &self,
        wallet_id: &WalletId,
        label: &str,
        secret: &SecretBytes,
    ) -> Result<(), FileStoreError> {
        let label = validated_label(label)?.to_string();
        let path = self.vault_path(wallet_id);
        self.with_vault_lock(wallet_id, || {
            let (mut vault, key) = match self.read_vault(&path)? {
                Some(vault) => {
                    let key = self.derive_and_verify(wallet_id, &vault)?;
                    (vault, key)
                }
                None => self.new_vault(wallet_id, &self.passphrase)?,
            };
            let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), &label);
            let (nonce, ciphertext) = crypto::seal(&key, &aad, secret.expose_secret())?;
            vault.entries.insert(label, EntryBody { nonce, ciphertext });
            self.write_vault(&path, &vault)
        })
    }

    /// `get` — returns the plaintext as a zeroizing [`SecretBytes`].
    /// `crypto::open` already returns `SecretBytes`, so the value
    /// propagates without an intervening `Vec<u8>` (CMT-008); the
    /// lone bare-buffer conversion lives at the upstream
    /// `CredentialApi::get_secret` SPI seam. `NoEntry`-shaped absence
    /// rides as `Ok(None)`.
    fn get(
        &self,
        wallet_id: &WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, FileStoreError> {
        let label = validated_label(label)?;
        let path = self.vault_path(wallet_id);
        let Some(vault) = self.read_vault(&path)? else {
            return Ok(None);
        };
        let key = self.derive_and_verify(wallet_id, &vault)?;
        let Some(body) = vault.entries.get(label) else {
            return Ok(None);
        };
        let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), label);
        entry_decrypt_or_corruption(
            wallet_id,
            label,
            crypto::open(&key, &body.nonce, &aad, &body.ciphertext),
        )
        .map(Some)
    }

    /// `delete` — upstream-compliant: returns whether an entry was
    /// removed so the SPI seam can surface `NoEntry` (D3, per the
    /// `CredentialApi::delete_credential` contract). The read-modify-
    /// write span is serialized via [`with_vault_lock`] (CMT-001).
    fn delete(&self, wallet_id: &WalletId, label: &str) -> Result<bool, FileStoreError> {
        let label = validated_label(label)?;
        let path = self.vault_path(wallet_id);
        self.with_vault_lock(wallet_id, || {
            let Some(mut vault) = self.read_vault(&path)? else {
                return Ok(false);
            };
            // Verify the passphrase before mutating, so a wrong pass
            // can neither delete an entry nor rewrite the vault.
            self.derive_and_verify(wallet_id, &vault)?;
            if vault.entries.remove(label).is_none() {
                return Ok(false);
            }
            self.write_vault(&path, &vault)?;
            Ok(true)
        })
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
        // Upstream SPI hands us a bare `&[u8]`; wrap into `SecretBytes`
        // immediately so the internal `put` chain only sees the
        // zeroizing wrapper (CMT-009). The wrap allocates once — the
        // same allocation the AEAD seal would have made anyway — and
        // gives the buffer mlock + zeroize-on-drop for the brief
        // window before seal consumes it.
        self.store
            .put(
                &self.wallet_id,
                &self.label,
                &SecretBytes::from_slice(secret),
            )
            .map_err(KeyringError::from)
    }

    fn get_secret(&self) -> KeyringResult<Vec<u8>> {
        let _ = validated_label(&self.label).map_err(FileStoreError::from)?;
        match self.store.get(&self.wallet_id, &self.label) {
            // Upstream SPI demands `Vec<u8>`; the single
            // `.expose_secret().to_vec()` conversion lives here, the
            // last point before the bare buffer crosses the SPI seam
            // (CMT-008). `SecretBytes` zeroizes on drop, so the
            // wrapped buffer is wiped as soon as it leaves scope.
            Ok(Some(v)) => Ok(v.expose_secret().to_vec()),
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

/// Project an entry-level `crypto::open` result into the typed
/// distinction the secret backend exposes (CMT-020). The verify-token
/// has already passed at every caller (`get` / `rekey`), so a
/// `FileStoreError::Decrypt` here is corruption or tampering of the
/// individual entry — **not** a wrong passphrase. Logs the non-secret
/// `(wallet_id, label)` pair at error level (never the secret) and
/// maps to `FileStoreError::Corruption`. Every other variant rides
/// through unchanged.
fn entry_decrypt_or_corruption(
    wallet_id: &WalletId,
    label: &str,
    result: Result<SecretBytes, FileStoreError>,
) -> Result<SecretBytes, FileStoreError> {
    result.map_err(|err| match err {
        FileStoreError::Decrypt => {
            tracing::error!(
                wallet_id = %wallet_id.to_hex(),
                label = %label,
                "vault entry failed integrity check (corruption or tampering)"
            );
            FileStoreError::Corruption
        }
        other => other,
    })
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

// INTENTIONAL(CMT-007): Windows ACL read-check deferred to a follow-up
// PR — tracked at https://github.com/dashpay/platform/issues/3754. Vault
// dir/file mode hardening on Windows requires GetSecurityInfo via
// `windows-acl` or `winapi`; out of scope for the secrets-feature
// landing. Operators on Windows MUST set ACLs manually until the
// follow-up lands.
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

/// Open a vault file for reading. On Unix the open refuses to traverse
/// a final-component symlink (`O_NOFOLLOW`) so a symlink swap between
/// the open and the read cannot redirect us to a different inode
/// (CMT-004). The file handle then drives every subsequent check
/// (perms, size, content), so a path-based race window cannot reopen
/// it.
#[cfg(unix)]
fn open_no_follow(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

/// Non-Unix fallback: no `O_NOFOLLOW` primitive available. The fd-based
/// metadata + read still close the metadata→read TOCTOU within this
/// process; symlink-swap defence on Windows is deferred with the same
/// scope note as [`check_perms`].
#[cfg(not(unix))]
fn open_no_follow(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new().read(true).open(path)
}

/// Tighten the vault directory to `0700` on Unix (CMT-002). A
/// pre-existing directory's looser bits are logged at warn level (so
/// the operator sees the prior exposure) and then in-place tightened
/// rather than refused — refusing would break the canonical
/// `tempfile::tempdir()` setup used throughout the test suite and any
/// real deployment that bootstraps the dir via `mkdir` under a 0o022
/// umask. After tightening, the mode is re-verified and a non-`0700`
/// result surfaces [`FileStoreError::InsecurePermissions`] (defence in
/// depth — `set_permissions` succeeding but not landing the bits is a
/// surprise worth failing loud).
#[cfg(unix)]
fn set_restrictive_dir_perms(dir: &Path, preexisted: bool) -> Result<(), FileStoreError> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(dir)?;
    let mode = meta.permissions().mode() & 0o777;
    if preexisted && mode & 0o077 != 0 {
        tracing::warn!(
            dir = %dir.display(),
            mode = format_args!("{mode:o}"),
            "pre-existing vault directory was looser than 0700; tightening in place"
        );
    }
    if mode != 0o700 {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        let after = fs::metadata(dir)?.permissions().mode() & 0o777;
        if after != 0o700 {
            return Err(FileStoreError::InsecurePermissions { mode: after });
        }
    }
    Ok(())
}

// INTENTIONAL(CMT-002): Windows ACL dir-tightening is deferred to the
// same follow-up that covers the file check (CMT-007), tracked at
// https://github.com/dashpay/platform/issues/3754. Vault dir hardening
// on Windows requires GetSecurityInfo via `windows-acl` or `winapi`;
// out of scope for the secrets-feature landing. Operators on Windows
// MUST set ACLs manually until the follow-up lands.
#[cfg(not(unix))]
fn set_restrictive_dir_perms(_dir: &Path, _preexisted: bool) -> Result<(), FileStoreError> {
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
        let mut vault = s.test_read_vault(&path).unwrap().unwrap();
        let a = vault.entries["labelA"].clone();
        let b = vault.entries.get_mut("labelB").unwrap();
        b.nonce = a.nonce;
        b.ciphertext = a.ciphertext.clone();
        s.test_write_vault(&path, &vault).unwrap();
        let err = entry(&s, wid(1), "labelB").get_secret().unwrap_err();
        // The verify-token passes (correct passphrase), so the
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
        // Bit-flip the entry ciphertext on disk; the verify-token is
        // untouched, so the passphrase is still correct.
        let path = s.test_vault_path(&wid(1));
        let mut vault = s.test_read_vault(&path).unwrap().unwrap();
        vault.entries.get_mut("seed").unwrap().ciphertext[0] ^= 0x01;
        s.test_write_vault(&path, &vault).unwrap();
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
        let mut vault = s.test_read_vault(&path).unwrap().unwrap();
        vault.entries.get_mut("seed").unwrap().ciphertext[0] ^= 0x01;
        s.test_write_vault(&path, &vault).unwrap();
        // Rekey with the *correct* old passphrase: verify token passes,
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
        // round-trip the test surface exposes.
        let path = s.test_vault_path(&wid(1));
        let mut vault = s.test_read_vault(&path).unwrap().unwrap();
        vault.kdf.m_kib = u32::MAX;
        s.test_write_vault(&path, &vault).unwrap();
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

    /// CMT-003 — vault files larger than [`MAX_VAULT_SIZE_BYTES`] must
    /// fail BEFORE the read allocates. Uses a sparse-file truncate so
    /// the test stays cheap (the allocator never sees real bytes), and
    /// asserts the typed `VaultTooLarge` projects through the SPI to
    /// `BadStoreFormat`.
    #[test]
    fn vault_above_size_cap_is_rejected_pre_read() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        // Materialize a fresh vault file so the path layout matches the
        // production one; then overwrite-extend to past the cap via a
        // sparse truncate (zero physical bytes used).
        entry(&s, wid(1), "seed").set_secret(b"v").unwrap();
        let path = s.test_vault_path(&wid(1));
        let oversized = MAX_VAULT_SIZE_BYTES + 1;
        let f = fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(oversized).unwrap();
        drop(f);

        let err = entry(&s, wid(1), "seed").get_secret().unwrap_err();
        match &err {
            KeyringError::BadStoreFormat(msg) => {
                let expected = FileStoreError::VaultTooLarge {
                    found: oversized,
                    max: MAX_VAULT_SIZE_BYTES,
                }
                .to_string();
                assert_eq!(*msg, expected, "unexpected message: {msg}");
            }
            other => panic!("expected BadStoreFormat(VaultTooLarge), got {other:?}"),
        }
    }

    /// CMT-001 — the cross-process advisory lock on the sidecar
    /// `.lock` file must serialize same-wallet writers within one
    /// process too. Holding the sidecar `.lock` from this thread (via
    /// a directly-held `fd_lock::RwLock::write` guard) must keep a
    /// peer `put` blocked-then-`Busy` past the wait budget.
    ///
    /// The test bypasses the in-process `Mutex<HashMap<...>>` layer on
    /// purpose: that layer guarantees an in-process serialization, but
    /// CMT-001's real teeth are the SIDECAR FILE lock (the bit that
    /// crosses process boundaries). We probe the file-lock branch
    /// directly so a future refactor that drops the file layer can't
    /// silently rely on the in-process map alone.
    #[test]
    fn vault_lock_contention_surfaces_busy() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        entry(&s, wid(1), "seed").set_secret(b"v").unwrap();
        let lock_path = s.inner.vault_lock_path(&wid(1));

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .unwrap();
        let mut rw = fd_lock::RwLock::new(file);
        let _guard = rw.write().expect("acquire exclusive sidecar lock");

        // While the sidecar is held, a put on the same wallet must hit
        // the timeout and surface Busy.
        let start = Instant::now();
        let err = s
            .inner
            .put(&wid(1), "other", &SecretBytes::from_slice(b"x"))
            .expect_err("peer put must contend");
        assert!(matches!(err, FileStoreError::Busy), "got {err:?}");
        assert!(
            start.elapsed() >= LOCK_WAIT_BUDGET,
            "Busy must arrive only after the wait budget; took {:?}",
            start.elapsed()
        );
    }
}
