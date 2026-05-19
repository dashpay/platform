//! The [`SecretStore`] port.

use super::error::SecretStoreError;
use super::secret::SecretBytes;
use super::validate::WalletId;

/// Stores wallet secret material out-of-band of the SQLite persister.
///
/// Implementations MUST NOT write any secret byte to the database, its
/// WAL, backups, `tracing` events, `Debug`/`Display`, error payloads,
/// panic messages, or temp files outside their own controlled path
/// (the SECRETS.md invariant, SEC-REQ-2.0.1).
///
/// All three methods validate `label` against the
/// `^[A-Za-z0-9._-]{1,64}$` allowlist before touching a backing store,
/// returning [`SecretStoreError::InvalidLabel`] on violation rather
/// than sanitizing.
pub trait SecretStore: Send + Sync {
    /// Store `bytes` under `(wallet_id, label)`, overwrite-safe: an
    /// existing label is atomically replaced or the call fails closed —
    /// both old and new plaintext are never simultaneously recoverable
    /// (SEC-REQ-2.0.2).
    ///
    /// The caller owns and must zeroize the source buffer; prefer
    /// [`put_secret`](SecretStore::put_secret) so the source is a
    /// `&SecretBytes`. The implementation MUST NOT copy `bytes` into a
    /// long-lived unwrapped buffer.
    fn put(&self, wallet_id: WalletId, label: &str, bytes: &[u8]) -> Result<(), SecretStoreError>;

    /// Retrieve the secret. `Ok(None)` for a missing label — idempotent
    /// and non-secret-leaking (SEC-REQ-2.0.3). The returned buffer
    /// zeroizes on drop (SEC-REQ-4.1); a bare `Vec<u8>` is never
    /// returned.
    fn get(
        &self,
        wallet_id: WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, SecretStoreError>;

    /// Idempotent delete. `Ok(())` whether or not the label existed; no
    /// secret-bearing error distinguishes the two cases.
    fn delete(&self, wallet_id: WalletId, label: &str) -> Result<(), SecretStoreError>;

    /// Ergonomic [`put`](SecretStore::put) over an already-wrapped
    /// secret. Default impl forwards the exposed bytes; no extra
    /// long-lived copy is made.
    fn put_secret(
        &self,
        wallet_id: WalletId,
        label: &str,
        secret: &SecretBytes,
    ) -> Result<(), SecretStoreError> {
        self.put(wallet_id, label, secret.expose_secret())
    }
}
