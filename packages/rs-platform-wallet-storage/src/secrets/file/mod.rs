//! [`EncryptedFileStore`] — passphrase-encrypted on-disk vault.
//!
//! One vault file per `wallet_id` (path namespaced by `wallet_id`
//! hex). Argon2id KDF + XChaCha20-Poly1305 AEAD, AAD-bound to
//! `(format_version, wallet_id, label)`, written atomically at mode
//! 0600.
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
mod format;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crypto::{KdfParams, SALT_LEN};
use format::{Entry, Header};

use super::error::SecretStoreError;
use super::secret::{SecretBytes, SecretString};
use super::store::SecretStore;
use super::validate::{validated_label, WalletId};

/// A passphrase-encrypted file-backed [`SecretStore`].
///
/// The passphrase is held in a [`SecretString`] for the store's
/// lifetime so each operation can re-derive the per-vault key; it is
/// never written anywhere and is zeroized when the store drops
/// (SEC-REQ-2.2.13). The derived AEAD key is recomputed per operation
/// and dropped (zeroized) immediately after use — it is never retained
/// on the struct.
pub struct EncryptedFileStore {
    dir: PathBuf,
    passphrase: SecretString,
}

impl EncryptedFileStore {
    /// Open (or prepare to create) a vault store rooted at `dir`,
    /// unlocked by `passphrase`. `dir` is created if missing.
    pub fn open(dir: impl AsRef<Path>, passphrase: SecretString) -> Result<Self, SecretStoreError> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;
        Ok(Self { dir, passphrase })
    }

    fn vault_path(&self, wallet_id: &WalletId) -> PathBuf {
        self.dir.join(format!("{}.pwsvault", wallet_id.to_hex()))
    }

    /// Read + parse a vault file, or `None` if it does not exist.
    /// Refuses a pre-existing file with looser-than-0600 perms
    /// (SEC-REQ-2.2.10).
    fn read_vault(&self, path: &Path) -> Result<Option<(Header, Vec<Entry>)>, SecretStoreError> {
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

    /// Atomically (temp → fsync → rename → dir-fsync) write the vault,
    /// creating the temp at 0600 via `O_EXCL`+`fchmod` before any
    /// ciphertext byte is written (SEC-REQ-2.2.10/.11). The temp holds
    /// only ciphertext+header — never plaintext.
    fn write_vault(
        &self,
        path: &Path,
        header: &Header,
        entries: &[Entry],
    ) -> Result<(), SecretStoreError> {
        let serialized = format::serialize(header, entries);
        let tmp = path.with_extension("pwsvault.tmp");
        // Remove a stale temp so O_EXCL can take a clean lock.
        let _ = fs::remove_file(&tmp);
        let result = (|| -> Result<(), SecretStoreError> {
            let mut opts = OpenOptions::new();
            opts.write(true).create_new(true);
            set_create_mode(&mut opts);
            let mut f = opts.open(&tmp)?;
            enforce_mode_0600(&f)?;
            f.write_all(&serialized)?;
            f.sync_all()?;
            fs::rename(&tmp, path)?;
            if let Some(parent) = path.parent() {
                if let Ok(d) = fs::File::open(parent) {
                    let _ = d.sync_all();
                }
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    /// Re-encrypt every entry under a fresh salt + fresh per-entry
    /// nonces with the current default Argon2 params and atomically
    /// replace the vault — no `.bak` retains old key material
    /// (SEC-REQ-2.2.12).
    pub fn rekey(
        &mut self,
        wallet_id: WalletId,
        new_passphrase: SecretString,
    ) -> Result<(), SecretStoreError> {
        let path = self.vault_path(&wallet_id);
        let Some((old_header, old_entries)) = self.read_vault(&path)? else {
            self.passphrase = new_passphrase;
            return Ok(());
        };
        let old_key = crypto::derive_key(
            self.passphrase.expose_secret().as_bytes(),
            &old_header.salt,
            old_header.params,
        )?;

        let mut new_salt = [0u8; SALT_LEN];
        crypto::random_bytes(&mut new_salt)?;
        let new_params = KdfParams::default_target();
        let new_key = crypto::derive_key(
            new_passphrase.expose_secret().as_bytes(),
            &new_salt,
            new_params,
        )?;

        let mut new_entries = Vec::with_capacity(old_entries.len());
        for e in &old_entries {
            let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), &e.label);
            let pt = crypto::open(&old_key, &e.nonce, &aad, &e.ciphertext)
                .map_err(|_| SecretStoreError::WrongPassphrase)?;
            let (nonce, ct) = crypto::seal(&new_key, &aad, pt.expose_secret())?;
            new_entries.push(Entry {
                label: e.label.clone(),
                nonce,
                ciphertext: ct,
            });
        }
        let new_header = Header {
            params: new_params,
            salt: new_salt,
        };
        self.write_vault(&path, &new_header, &new_entries)?;
        self.passphrase = new_passphrase;
        Ok(())
    }
}

impl SecretStore for EncryptedFileStore {
    fn put(&self, wallet_id: WalletId, label: &str, bytes: &[u8]) -> Result<(), SecretStoreError> {
        let label = validated_label(label)?.to_string();
        let path = self.vault_path(&wallet_id);
        let (header, mut entries) = match self.read_vault(&path)? {
            Some(v) => v,
            None => {
                let mut salt = [0u8; SALT_LEN];
                crypto::random_bytes(&mut salt)?;
                (
                    Header {
                        params: KdfParams::default_target(),
                        salt,
                    },
                    Vec::new(),
                )
            }
        };
        let key = crypto::derive_key(
            self.passphrase.expose_secret().as_bytes(),
            &header.salt,
            header.params,
        )?;
        let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), &label);
        let (nonce, ciphertext) = crypto::seal(&key, &aad, bytes)?;
        entries.retain(|e| e.label != label);
        entries.push(Entry {
            label,
            nonce,
            ciphertext,
        });
        self.write_vault(&path, &header, &entries)
    }

    fn get(
        &self,
        wallet_id: WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, SecretStoreError> {
        let label = validated_label(label)?;
        let path = self.vault_path(&wallet_id);
        let Some((header, entries)) = self.read_vault(&path)? else {
            return Ok(None);
        };
        let Some(entry) = entries.iter().find(|e| e.label == label) else {
            return Ok(None);
        };
        let key = crypto::derive_key(
            self.passphrase.expose_secret().as_bytes(),
            &header.salt,
            header.params,
        )?;
        let aad = format::aad(format::FORMAT_VERSION, wallet_id.as_bytes(), label);
        match crypto::open(&key, &entry.nonce, &aad, &entry.ciphertext) {
            Ok(pt) => Ok(Some(pt)),
            Err(SecretStoreError::Decrypt) => Err(SecretStoreError::WrongPassphrase),
            Err(e) => Err(e),
        }
    }

    fn delete(&self, wallet_id: WalletId, label: &str) -> Result<(), SecretStoreError> {
        let label = validated_label(label)?;
        let path = self.vault_path(&wallet_id);
        let Some((header, mut entries)) = self.read_vault(&path)? else {
            return Ok(());
        };
        let before = entries.len();
        entries.retain(|e| e.label != label);
        if entries.len() == before {
            return Ok(());
        }
        self.write_vault(&path, &header, &entries)
    }
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

#[cfg(not(unix))]
fn check_perms(_meta: &fs::Metadata) -> Result<(), SecretStoreError> {
    Ok(())
}

#[cfg(unix)]
fn set_create_mode(opts: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    opts.mode(0o600);
}

#[cfg(not(unix))]
fn set_create_mode(_opts: &mut OpenOptions) {}

#[cfg(unix)]
fn enforce_mode_0600(f: &fs::File) -> Result<(), SecretStoreError> {
    use std::os::unix::fs::PermissionsExt;
    f.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn enforce_mode_0600(_f: &fs::File) -> Result<(), SecretStoreError> {
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

    #[test]
    fn roundtrip_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        {
            let s = store(dir.path());
            s.put(wid(1), "bip39_mnemonic", b"abandon abandon").unwrap();
        }
        let s2 = store(dir.path());
        let got = s2.get(wid(1), "bip39_mnemonic").unwrap().unwrap();
        assert_eq!(got.expose_secret(), b"abandon abandon");
        assert!(s2.get(wid(1), "missing").unwrap().is_none());
    }

    #[test]
    fn wrong_passphrase_fails_no_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        store(dir.path())
            .put(wid(1), "seed", b"super secret")
            .unwrap();
        let bad = EncryptedFileStore::open(dir.path(), SecretString::new("pw-wrong")).unwrap();
        let err = bad.get(wid(1), "seed").unwrap_err();
        assert!(matches!(err, SecretStoreError::WrongPassphrase));
        // The error renders without any plaintext.
        assert!(!format!("{err}").contains("super secret"));
    }

    #[test]
    fn idempotent_delete_and_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        s.delete(wid(1), "seed").unwrap(); // no vault yet
        s.put(wid(1), "seed", b"v1").unwrap();
        s.put(wid(1), "seed", b"v2").unwrap();
        assert_eq!(
            s.get(wid(1), "seed").unwrap().unwrap().expose_secret(),
            b"v2"
        );
        s.delete(wid(1), "seed").unwrap();
        s.delete(wid(1), "seed").unwrap(); // idempotent
        assert!(s.get(wid(1), "seed").unwrap().is_none());
    }

    #[test]
    fn blob_swap_across_label_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        s.put(wid(1), "labelA", b"secretA").unwrap();
        s.put(wid(1), "labelB", b"secretB").unwrap();
        let path = s.vault_path(&wid(1));
        let (header, mut entries) = s.read_vault(&path).unwrap().unwrap();
        // Move A's ciphertext+nonce into B's slot.
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
        s.write_vault(&path, &header, &entries).unwrap();
        assert!(matches!(
            s.get(wid(1), "labelB"),
            Err(SecretStoreError::WrongPassphrase) | Err(SecretStoreError::Decrypt)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn vault_created_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        s.put(wid(1), "seed", b"x").unwrap();
        let mode = fs::metadata(s.vault_path(&wid(1)))
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
        s.put(wid(1), "seed", b"x").unwrap();
        let path = s.vault_path(&wid(1));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            s.get(wid(1), "seed"),
            Err(SecretStoreError::InsecurePermissions { mode: 0o644 })
        ));
    }

    #[test]
    fn rekey_reencrypts_and_old_passphrase_fails() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = store(dir.path());
        s.put(wid(1), "seed", b"value").unwrap();
        let old_bytes = fs::read(s.vault_path(&wid(1))).unwrap();
        s.rekey(wid(1), SecretString::new("pw-new")).unwrap();
        // New passphrase reads; ciphertext changed; no .bak left.
        assert_eq!(
            s.get(wid(1), "seed").unwrap().unwrap().expose_secret(),
            b"value"
        );
        let new_bytes = fs::read(s.vault_path(&wid(1))).unwrap();
        assert_ne!(old_bytes, new_bytes);
        let stale: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.ends_with(".bak") || n.ends_with(".tmp")
            })
            .collect();
        assert!(stale.is_empty(), "rekey left stale files: {stale:?}");
        let old = EncryptedFileStore::open(dir.path(), SecretString::new("pw-correct")).unwrap();
        assert!(matches!(
            old.get(wid(1), "seed"),
            Err(SecretStoreError::WrongPassphrase)
        ));
    }

    #[test]
    fn no_plaintext_in_vault_file() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        s.put(wid(1), "seed", b"PLAINTEXTNEEDLE").unwrap();
        let raw = fs::read(s.vault_path(&wid(1))).unwrap();
        assert!(
            raw.windows(b"PLAINTEXTNEEDLE".len())
                .all(|w| w != b"PLAINTEXTNEEDLE"),
            "plaintext leaked into vault file"
        );
    }
}
