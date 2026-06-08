//! Zeroizing secret wrappers: [`SecretString`] for UTF-8 secrets and
//! [`SecretBytes`] for byte secrets (seeds, xprivs, KDF output, AEAD
//! keys, decrypted plaintext). Both have a redacting `Debug`, no
//! `Display`/`Deref`/`Serialize`, a full buffer wipe on drop, and a
//! best-effort `region` mlock (CWE-316).

use std::fmt;

use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

/// Pre-allocation capacity for [`SecretString`] buffers.
///
/// `mlock` is page-granular, so a sub-page buffer locks a whole page
/// regardless; 4096 bytes also makes `String` reallocation (which
/// leaves an un-zeroed freed buffer the allocator owns) virtually
/// impossible for any human-entered passphrase or mnemonic.
const DEFAULT_CAPACITY: usize = 4096;

/// Zeroize-on-drop wrapper for secret UTF-8 strings (BIP-39 mnemonic,
/// `EncryptedFileStore` passphrase).
///
/// `Display`, `Deref`, `DerefMut`, `Serialize`, `PartialEq`, `Eq` are
/// intentionally **not** implemented; read access is the explicit
/// [`expose_secret`] only, and equality goes through
/// [`subtle::ConstantTimeEq`] (`==` on secret bytes is forbidden, no
/// exception, so future bridge code cannot inherit a non-constant-time
/// path). `Debug` is redacted. `Zeroizing<String>`
/// wipes the buffer over its full capacity on drop; the buffer is
/// best-effort `mlock`ed against swap.
///
/// [`expose_secret`]: SecretString::expose_secret
///
/// ```compile_fail
/// use platform_wallet_storage::secrets::SecretString;
/// let a = SecretString::new("pw");
/// let b = SecretString::new("pw");
/// let _ = a == b; // `==` on SecretString is forbidden; use ConstantTimeEq::ct_eq
/// ```
pub struct SecretString {
    // Field order is load-bearing: `inner` drops (and `Zeroizing` wipes
    // it) before `_lock` releases the page, so the buffer is wiped while
    // still mlock'ed.
    inner: Zeroizing<String>,
    _lock: Option<region::LockGuard>,
}

impl SecretString {
    /// Wrap a string, copying it into a capacity-padded buffer,
    /// zeroizing the source, and best-effort `mlock`ing the buffer.
    pub fn new(s: impl Into<String>) -> Self {
        let mut source: String = s.into();
        let cap = source.len().max(DEFAULT_CAPACITY);
        let mut buf = String::with_capacity(cap);
        buf.push_str(&source);
        source.zeroize();
        let lock = region::lock(buf.as_ptr(), buf.capacity())
            .map_err(|e| {
                tracing::warn!(
                    "mlock failed for SecretString; secret may be swappable to disk: {e}"
                );
                e
            })
            .ok();
        Self {
            inner: Zeroizing::new(buf),
            _lock: lock,
        }
    }

    /// An empty, capacity-padded, locked buffer.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Borrow the plaintext. The only read path.
    pub fn expose_secret(&self) -> &str {
        &self.inner
    }

    /// Secret length in bytes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the secret is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// A new `SecretString` holding the whitespace-trimmed content,
    /// keeping the trimmed copy inside the wrapper.
    pub fn trimmed(&self) -> Self {
        Self::new(self.inner.trim().to_string())
    }
}

impl Default for SecretString {
    fn default() -> Self {
        let s = String::with_capacity(DEFAULT_CAPACITY);
        let lock = region::lock(s.as_ptr(), s.capacity())
            .map_err(|e| {
                tracing::warn!(
                    "mlock failed for SecretString; secret may be swappable to disk: {e}"
                );
                e
            })
            .ok();
        Self {
            inner: Zeroizing::new(s),
            _lock: lock,
        }
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(***)")
    }
}

impl ConstantTimeEq for SecretString {
    /// Constant-time compare over the equal-length region. Unequal
    /// lengths return `0` without revealing where they differ; the
    /// only observable is the (non-secret) length difference.
    fn ct_eq(&self, other: &Self) -> subtle::Choice {
        self.expose_secret()
            .as_bytes()
            .ct_eq(other.expose_secret().as_bytes())
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

/// Zeroize-on-drop wrapper for secret **bytes**: BIP-32 seed
/// (`[u8; 64]`), xpriv, Argon2 output, AEAD key, decrypted plaintext,
/// ciphertext-in-flight.
///
/// Not `Copy`; `Clone` is intentionally absent to enforce copy
/// minimization — move it, or `expose_secret()` and copy
/// deliberately into another wrapper. `Display`, `Deref`, `Serialize`,
/// `PartialEq`, `Eq` are intentionally **not** implemented; equality
/// goes through [`subtle::ConstantTimeEq`] only (`==` on secret bytes is
/// forbidden, no exception, so future bridge code cannot inherit a
/// non-constant-time path). `Debug` is redacted; the
/// buffer is wiped on drop and best-effort `mlock`ed.
///
/// ```compile_fail
/// use platform_wallet_storage::secrets::SecretBytes;
/// let a = SecretBytes::new(vec![0u8; 32]);
/// let b = SecretBytes::new(vec![0u8; 32]);
/// let _ = a == b; // `==` on SecretBytes is forbidden; use ConstantTimeEq::ct_eq
/// ```
pub struct SecretBytes {
    // Field order is load-bearing: `inner` drops (and `Zeroizing` wipes
    // it) before `_lock` releases the page, so the buffer is wiped while
    // still mlock'ed.
    inner: Zeroizing<Vec<u8>>,
    _lock: Option<region::LockGuard>,
}

impl SecretBytes {
    /// Wrap a byte vector, moving it into the wrapper and best-effort
    /// `mlock`ing the buffer.
    pub fn new(bytes: Vec<u8>) -> Self {
        // Lock only a non-empty allocation: an empty `Vec`'s `as_ptr()`
        // is dangling, and `region::lock` rejects a 0-length region.
        let lock = if bytes.capacity() > 0 {
            region::lock(bytes.as_ptr(), bytes.capacity())
                .map_err(|e| {
                    tracing::warn!(
                        "mlock failed for SecretBytes; secret may be swappable to disk: {e}"
                    );
                    e
                })
                .ok()
        } else {
            None
        };
        // The move transfers ownership of the allocation into
        // `Zeroizing`; the source buffer is not copied, so there is
        // nothing left behind to wipe.
        Self {
            inner: Zeroizing::new(bytes),
            _lock: lock,
        }
    }

    /// A zeroed buffer of `len` bytes, best-effort `mlock`ed — for
    /// in-place fills (KDF output, decrypt target).
    pub fn zeroed(len: usize) -> Self {
        Self::new(vec![0u8; len])
    }

    /// Copy a borrowed slice into a fresh wrapper. Deliberate, explicit
    /// copy — the only way to duplicate secret bytes.
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self::new(bytes.to_vec())
    }

    /// Borrow the plaintext bytes. The only read path.
    pub fn expose_secret(&self) -> &[u8] {
        &self.inner
    }

    /// Mutably borrow the plaintext bytes (in-place KDF/decrypt fill).
    pub fn expose_secret_mut(&mut self) -> &mut [u8] {
        &mut self.inner
    }

    /// Secret length in bytes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the secret is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl ConstantTimeEq for SecretBytes {
    /// Fixed-width constant-time compare over the byte region — no
    /// length early-return. `subtle::ConstantTimeEq` on
    /// unequal-length slices yields `0` without leaking *where* they
    /// differ; the only observable is the (non-secret) length.
    fn ct_eq(&self, other: &Self) -> subtle::Choice {
        self.inner.as_slice().ct_eq(other.inner.as_slice())
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretBytes([REDACTED; {}])", self.inner.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_string_debug_redacted() {
        let s = SecretString::new("correct horse battery staple");
        let dbg = format!("{s:?}");
        assert_eq!(dbg, "SecretString(***)");
        assert!(!dbg.contains("horse"));
    }

    #[test]
    fn secret_string_expose_and_trim() {
        let s = SecretString::new("  abandon ability  ");
        assert_eq!(s.expose_secret(), "  abandon ability  ");
        assert_eq!(s.trimmed().expose_secret(), "abandon ability");
    }

    #[test]
    fn secret_string_ct_eq_is_value_based() {
        // Equality goes through `ConstantTimeEq` only.
        let same = SecretString::new("pw").ct_eq(&SecretString::new("pw"));
        let diff = SecretString::new("pw").ct_eq(&SecretString::new("px"));
        let len_diff = SecretString::new("pw").ct_eq(&SecretString::new("pww"));
        assert!(bool::from(same));
        assert!(!bool::from(diff));
        assert!(!bool::from(len_diff));
    }

    #[test]
    fn secret_string_empty_default() {
        assert!(SecretString::empty().is_empty());
        assert_eq!(SecretString::default().len(), 0);
    }

    #[test]
    fn secret_bytes_debug_redacted() {
        let b = SecretBytes::from_slice(&[1, 2, 3, 4, 5]);
        let dbg = format!("{b:?}");
        assert_eq!(dbg, "SecretBytes([REDACTED; 5])");
        assert!(!dbg.contains('1'));
    }

    #[test]
    fn secret_bytes_roundtrip_and_zeroed() {
        let b = SecretBytes::from_slice(&[9, 8, 7]);
        assert_eq!(b.expose_secret(), &[9, 8, 7]);
        assert_eq!(b.len(), 3);
        let z = SecretBytes::zeroed(4);
        assert_eq!(z.expose_secret(), &[0, 0, 0, 0]);
    }

    #[test]
    fn empty_secret_bytes_constructs_without_mlocking_dangling_ptr() {
        // A capacity-0 `Vec` has a dangling `as_ptr()`; `new` must not
        // pass it to `region::lock`. Constructing must not panic and the
        // wrapper must round-trip as empty.
        let b = SecretBytes::new(Vec::new());
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
        assert_eq!(b.expose_secret(), &[] as &[u8]);
        let z = SecretBytes::zeroed(0);
        assert!(z.is_empty());
    }

    #[test]
    fn secret_bytes_constant_time_eq() {
        let a = SecretBytes::from_slice(&[1, 2, 3, 4]);
        let b = SecretBytes::from_slice(&[1, 2, 3, 4]);
        let c = SecretBytes::from_slice(&[1, 2, 3, 5]);
        let d = SecretBytes::from_slice(&[1, 2, 3]);
        assert!(bool::from(a.ct_eq(&b)));
        assert!(!bool::from(a.ct_eq(&c)));
        assert!(!bool::from(a.ct_eq(&d)));
    }

    #[test]
    fn secret_bytes_expose_mut_fills_in_place() {
        let mut b = SecretBytes::zeroed(3);
        b.expose_secret_mut().copy_from_slice(&[7, 7, 7]);
        assert_eq!(b.expose_secret(), &[7, 7, 7]);
    }

    // `SecretBytes`/`SecretString` must run `Drop` (zeroize), so they
    // cannot be trivially droppable.
    const _: () = {
        assert!(std::mem::needs_drop::<SecretString>());
        assert!(std::mem::needs_drop::<SecretBytes>());
    };

    /// Best-effort runtime check that `Drop` wipes the full `SecretString`
    /// capacity. Reads freed memory — UB in the strict sense, flaky under
    /// parallelism; run single-threaded:
    /// `cargo test --features secrets -- secret_string_drop_zeroes --ignored --test-threads=1`
    #[test]
    #[ignore]
    fn secret_string_drop_zeroes_full_capacity() {
        let ptr: *const u8;
        let cap: usize;
        {
            let s = SecretString::new("sensitive_seed_material");
            ptr = s.inner.as_ptr();
            cap = s.inner.capacity();
            // SAFETY: live allocation, read for `cap` bytes pre-drop.
            #[allow(unsafe_code)]
            let pre = unsafe { std::slice::from_raw_parts(ptr, cap) };
            assert!(pre.iter().any(|&b| b != 0));
        }
        // SAFETY: best-effort post-free read; single-thread makes page
        // reuse before this read unlikely.
        #[allow(unsafe_code)]
        let post = unsafe { std::slice::from_raw_parts(ptr, cap) };
        assert!(post.iter().all(|&b| b == 0), "buffer not zeroed on drop");
    }

    /// Best-effort runtime check that `Drop` wipes `SecretBytes`. Same
    /// caveat as above; run single-threaded with `--ignored`. A
    /// page-sized buffer is used so the allocator is unlikely to reuse
    /// the freed page before the post-drop read (a tiny `Vec` would be
    /// recycled immediately, making the check meaningless).
    #[test]
    #[ignore]
    fn secret_bytes_drop_zeroes() {
        let ptr: *const u8;
        let cap: usize;
        {
            let b = SecretBytes::from_slice(&[0xAB; 4096]);
            ptr = b.inner.as_ptr();
            cap = b.inner.capacity();
            // SAFETY: live allocation, read for `cap` bytes pre-drop.
            #[allow(unsafe_code)]
            let pre = unsafe { std::slice::from_raw_parts(ptr, cap) };
            assert!(pre.iter().any(|&x| x != 0));
        }
        // SAFETY: best-effort post-free read; see note above.
        #[allow(unsafe_code)]
        let post = unsafe { std::slice::from_raw_parts(ptr, cap) };
        assert!(post.iter().all(|&x| x == 0), "buffer not zeroed on drop");
    }
}
