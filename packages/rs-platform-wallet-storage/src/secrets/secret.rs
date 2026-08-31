//! Zeroizing secret wrappers: [`SecretString`] for UTF-8 secrets and
//! [`SecretBytes`] for byte secrets (seeds, xprivs, KDF output, AEAD
//! keys, decrypted plaintext). Both have a redacting `Debug`, no
//! `Display`/`Deref`/`Serialize`, a full buffer wipe on drop, and a
//! best-effort `region` mlock (CWE-316).

use std::fmt;

use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

/// Pre-allocation capacity for [`SecretString`] buffers. `mlock` is
/// page-granular (a sub-page buffer locks a whole page anyway), and 4096
/// bytes makes a reallocation — which would leave an un-zeroed freed
/// buffer behind — virtually impossible for any human-entered secret.
const DEFAULT_CAPACITY: usize = 4096;

/// Minimum post-trim byte length for a vault passphrase or Tier-2 password.
///
/// This defense-in-depth floor rejects trivially short inputs but is not a
/// strength estimator. Dictionary checks, UX feedback, and the real entropy
/// policy remain the consumer's responsibility (see `SECRETS.md`).
pub const MIN_PASSPHRASE_LEN: usize = 8;

/// An `mlock`ed region that releases best-effort, never fatally.
///
/// Used in place of [`region::LockGuard`], whose `Drop` runs
/// `debug_assert!(result.is_ok())` on the unlock and so aborts a debug
/// build when the OS reports the region was already unlocked. That is a
/// routine outcome, not a caller bug: locking is page-granular, so two
/// secrets allocated close together share a page, and `VirtualUnlock` is
/// not reference-counted — the first secret to drop releases the shared
/// page and the second sees `ERROR_NOT_LOCKED`. A destructor must not
/// fail on an expected, benign condition, so this one logs instead.
///
/// The address is held as a `usize` and only ever handed back to
/// [`region::unlock`], never dereferenced. That keeps the lock — and so
/// [`SecretString`]/[`SecretBytes`] — `Send + Sync` without the
/// `unsafe impl`s `region::LockGuard` relies on, which this crate's
/// `deny(unsafe_code)` forbids.
struct PageLock {
    address: usize,
    size: usize,
}

impl PageLock {
    /// Lock `[address, address + size)` into RAM.
    ///
    /// Callers log their own failures: the two `SecretString` call sites
    /// deliberately report at different levels with distinct wording, so
    /// this returns the error rather than emitting one message for all.
    ///
    /// # Errors
    ///
    /// Propagates [`region::lock`] — a zero `size`, or an OS refusal
    /// (a low `RLIMIT_MEMLOCK` being the usual cause).
    fn acquire<T>(address: *const T, size: usize) -> Result<Self, region::Error> {
        let guard = region::lock(address, size)?;
        // Leak the guard object, not the lock: its `Drop` is the aborting
        // one, and it exposes no accessors, so releasing the region has to
        // go through the address and size recorded here. `region::unlock`
        // rounds to page boundaries exactly as `region::lock` did, so this
        // releases precisely the region that was locked.
        std::mem::forget(guard);
        Ok(Self {
            address: address.addr(),
            size,
        })
    }
}

impl Drop for PageLock {
    fn drop(&mut self) {
        let address = std::ptr::without_provenance::<u8>(self.address);
        if let Err(e) = region::unlock(address, self.size) {
            tracing::debug!(
                "munlock failed while releasing a secret buffer; a secret \
                 sharing the page may already have released it: {e}"
            );
        }
    }
}

/// Zeroize-on-drop wrapper for secret UTF-8 strings (BIP-39 mnemonic,
/// `EncryptedFileStore` passphrase).
///
/// Read access is [`expose_secret`] only; equality goes through
/// [`subtle::ConstantTimeEq`] (`==` is forbidden so bridge code cannot
/// inherit a non-constant-time path). `Display`/`Deref`/`Serialize`/`Eq`
/// are deliberately absent, `Debug` is redacted, and the buffer wipes
/// over its full capacity on drop and is best-effort `mlock`ed.
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
    // Field order is load-bearing: `inner` drops (Zeroizing wipes it)
    // before `_lock` releases the page, so the wipe runs while mlock'ed.
    inner: Zeroizing<String>,
    _lock: Option<PageLock>,
}

impl SecretString {
    /// Wrap a string, copying it into a capacity-padded buffer,
    /// zeroizing the source, and best-effort `mlock`ing the buffer.
    pub fn new(s: impl Into<String>) -> Self {
        let mut source: String = s.into();
        let cap = source.len().max(DEFAULT_CAPACITY);
        let mut buf = String::with_capacity(cap);
        buf.push_str(&source);
        // Do not remove: wipes the moved-in plaintext source before it drops.
        // A direct freed-buffer scan would require `unsafe`, which this crate
        // forbids; the test `secret_string_new_zeroizes_string_source` instead
        // pins the `String::zeroize` primitive and this call site.
        source.zeroize();
        let lock = PageLock::acquire(buf.as_ptr(), buf.capacity())
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

    /// Whether the secret is empty or all Unicode-whitespace.
    ///
    /// Returns only blank-ness — never a borrowed view of the plaintext —
    /// and uses [`str::trim`] (the Unicode `White_Space` property), so a
    /// NBSP (`U+00A0`) trims to blank but a ZWSP (`U+200B`, not
    /// `White_Space`) does not. Minimum-length enforcement uses
    /// [`is_below_minimum_passphrase_len`](Self::is_below_minimum_passphrase_len)
    /// instead. Always available — **not** feature-gated.
    pub fn is_blank(&self) -> bool {
        self.inner.trim().is_empty()
    }

    /// Whether the trimmed secret is shorter than [`MIN_PASSPHRASE_LEN`].
    pub(crate) fn is_below_minimum_passphrase_len(&self) -> bool {
        self.inner.trim().len() < MIN_PASSPHRASE_LEN
    }
}

impl Default for SecretString {
    fn default() -> Self {
        let s = String::with_capacity(DEFAULT_CAPACITY);
        let lock = PageLock::acquire(s.as_ptr(), s.capacity())
            .map_err(|e| {
                // Empty buffer — no secret at risk, so this is diagnostic
                // noise, not a confidentiality event. `debug!` (not the
                // `new()` path's `warn!`) and distinct wording keep the two
                // call sites individually greppable.
                tracing::debug!(
                    "mlock failed for empty default SecretString buffer; no secret at risk: {e}"
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
    /// Constant-time compare. Unequal lengths return `0` without
    /// revealing where they differ; the only leak is the non-secret length.
    fn ct_eq(&self, other: &Self) -> subtle::Choice {
        self.expose_secret()
            .as_bytes()
            .ct_eq(other.expose_secret().as_bytes())
    }
}

impl Zeroize for SecretString {
    /// Wipe the buffer in place on a live value. `Drop` runs the same
    /// wipe automatically; this lets a holder zeroize early.
    fn zeroize(&mut self) {
        self.inner.zeroize();
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

/// Deserialize a UTF-8 secret (a vault passphrase or a Tier-2 object
/// password arriving via config), routing the owned `String` through
/// [`SecretString::new`] — which zeroizes its source — so no
/// intermediate plaintext buffer **we own** lingers (CWE-316).
///
/// Gated behind the dedicated, default-off `secret-serde` feature, NOT the
/// crate's internal `serde` dep (which `secrets` already pulls): the gate
/// is on the IMPL, so the impl is absent unless explicitly opted in, even
/// though `serde` itself is compiled. There is deliberately **no**
/// `Serialize` companion (a secret is read-from-config, never written
/// back / round-tripped / logged), so this type cannot leak out through
/// serde under any feature combination.
///
/// **Residual (documented, not closeable here):** the deserializer's own
/// input buffer holds the cleartext before this visitor runs and is
/// outside `SecretString`'s ownership, so it cannot be wiped here — feed
/// secrets from a zeroizing source. Mirrors the Argon2 `Block` residual
/// noted at `crypto::derive_key`.
#[cfg(feature = "secret-serde")]
impl<'de> serde::Deserialize<'de> for SecretString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SecretStringVisitor;

        impl<'v> serde::de::Visitor<'v> for SecretStringVisitor {
            type Value = SecretString;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a secret string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // Take ownership of the borrowed bytes, then hand the owned
                // `String` to the zeroizing constructor below.
                self.visit_string(v.to_owned())
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                // `SecretString::new` zeroizes the moved-in `String`.
                Ok(SecretString::new(v))
            }
        }

        deserializer.deserialize_string(SecretStringVisitor)
    }
}

/// Render the JSON schema as a plain `string` carrying **no** length or
/// value policy: no `minLength`/`maxLength`/`pattern`/`format` (would leak
/// a length policy) and no `example`/`default` (would embed a value)
/// A short, value-free `description` marks sensitivity.
///
/// Gated behind the default-off `secret-schemars` feature (which implies
/// `secret-serde`). Pulls in no `Serialize`/`Display` path.
#[cfg(feature = "secret-schemars")]
impl schemars::JsonSchema for SecretString {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("SecretString")
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("platform_wallet_storage::secrets::SecretString")
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "A secret string. Write-only: never serialized, never echoed."
        })
    }
}

/// Zeroize-on-drop wrapper for secret **bytes**: BIP-32 seed
/// (`[u8; 64]`), xpriv, Argon2 output, AEAD key, decrypted plaintext.
///
/// `Clone` is absent to force deliberate copies (move it, or
/// `expose_secret()` into another wrapper). Equality goes through
/// [`subtle::ConstantTimeEq`] only (`==` is forbidden so bridge code
/// cannot inherit a non-constant-time path). `Display`/`Deref`/`Serialize`
/// /`Eq` are absent, `Debug` is redacted, and the buffer wipes on drop
/// and is best-effort `mlock`ed.
///
/// ```compile_fail
/// use platform_wallet_storage::secrets::SecretBytes;
/// let a = SecretBytes::new(vec![0u8; 32]);
/// let b = SecretBytes::new(vec![0u8; 32]);
/// let _ = a == b; // `==` on SecretBytes is forbidden; use ConstantTimeEq::ct_eq
/// ```
pub struct SecretBytes {
    // Field order is load-bearing: `inner` drops (Zeroizing wipes it)
    // before `_lock` releases the page, so the wipe runs while mlock'ed.
    inner: Zeroizing<Vec<u8>>,
    _lock: Option<PageLock>,
}

impl SecretBytes {
    /// Wrap a byte vector, moving it into the wrapper and best-effort
    /// `mlock`ing the buffer.
    pub fn new(bytes: Vec<u8>) -> Self {
        // Skip an empty allocation: an empty `Vec`'s `as_ptr()` is
        // dangling and `region::lock` rejects a 0-length region.
        let lock = if bytes.capacity() > 0 {
            PageLock::acquire(bytes.as_ptr(), bytes.capacity())
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
    /// Constant-time compare, no length early-return. Unequal lengths
    /// yield `0` without leaking *where* they differ; only the non-secret
    /// length is observable.
    fn ct_eq(&self, other: &Self) -> subtle::Choice {
        self.inner.as_slice().ct_eq(other.inner.as_slice())
    }
}

impl Zeroize for SecretBytes {
    /// Wipe the buffer in place on a live value. `Drop` runs the same
    /// wipe automatically; this lets a holder zeroize early.
    fn zeroize(&mut self) {
        self.inner.zeroize();
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

    /// Two sound checks (a direct freed-buffer scan would be use-after-free,
    /// and this crate forbids `unsafe`): (1) `String::zeroize` empties a
    /// buffer — the primitive `new` relies on; (2) `new` copies the content
    /// into the wrapper faithfully. That `new` actually calls
    /// `source.zeroize()` on its moved-in source is pinned by the
    /// do-not-remove comment at that call site, not asserted here.
    #[test]
    fn secret_string_new_zeroizes_string_source() {
        let mut source = String::from("super secret seed material");
        source.zeroize();
        assert!(source.is_empty(), "String::zeroize must empty the source");
        let s = SecretString::new(String::from("super secret seed material"));
        assert_eq!(s.expose_secret(), "super secret seed material");
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

    /// `is_blank()` truth table. The boundary deliberately
    /// exercises Unicode whitespace — `str::trim` uses the `White_Space`
    /// property, so NBSP (`U+00A0`) trims to blank but ZWSP (`U+200B`,
    /// not `White_Space`) does not.
    #[test]
    fn is_blank_truth_table() {
        // Blank inputs.
        assert!(SecretString::empty().is_blank());
        assert!(SecretString::new("").is_blank());
        assert!(SecretString::new("   ").is_blank());
        assert!(SecretString::new("\t\r\n ").is_blank());
        assert!(
            SecretString::new("\u{00A0}").is_blank(),
            "NBSP is White_Space"
        );
        // Non-blank inputs.
        assert!(!SecretString::new("pw").is_blank());
        assert!(!SecretString::new("  pw  ").is_blank());
        assert!(
            !SecretString::new("\u{200B}").is_blank(),
            "ZWSP is NOT White_Space"
        );
    }

    /// `is_blank` returns a `bool` and exposes no borrowed
    /// plaintext, callable with only `secrets` (no serde/schemars).
    #[test]
    fn is_blank_signature_returns_bool_no_borrow() {
        let f: fn(&SecretString) -> bool = SecretString::is_blank;
        assert!(f(&SecretString::new("")));
        assert!(!f(&SecretString::new("x")));
    }

    /// `SecretString` must never implement
    /// `Serialize` or `Display`, even with serde compiled in. This is a
    /// compile-time `!impl` assertion — adding either impl breaks the
    /// build. `serde::Serialize` is nameable here because `secrets` always
    /// pulls the `serde` dep.
    #[test]
    fn secret_string_has_no_serialize_no_display() {
        static_assertions::assert_not_impl_any!(SecretString: serde::Serialize, std::fmt::Display);
    }

    /// Regression: the `serde` DEP is on under
    /// `secrets`, yet the `Deserialize` IMPL stays ABSENT because it is
    /// gated on the dedicated `secret-serde` feature — proving the
    /// default-off gate is satisfiable even while serde is compiled.
    #[cfg(not(feature = "secret-serde"))]
    #[test]
    fn deserialize_absent_without_secret_serde_even_though_serde_dep_on() {
        static_assertions::assert_not_impl_any!(
            SecretString: serde::de::DeserializeOwned
        );
    }

    /// With `secret-serde` on, the `Deserialize` impl is
    /// present (and `Serialize` is still absent — see the always-on test).
    #[cfg(feature = "secret-serde")]
    #[test]
    fn deserialize_present_with_secret_serde() {
        static_assertions::assert_impl_all!(SecretString: serde::de::DeserializeOwned);
        static_assertions::assert_not_impl_any!(SecretString: serde::Serialize);
    }

    /// `Deserialize` round-trips the value through the
    /// zeroizing constructor; the result `ct_eq`s a directly-built secret
    /// and has the right length.
    #[cfg(feature = "secret-serde")]
    #[test]
    fn deserialize_routes_value_through_zeroizing_constructor() {
        let s: SecretString = serde_json::from_str("\"correct horse battery staple\"").unwrap();
        assert!(bool::from(
            s.ct_eq(&SecretString::new("correct horse battery staple"))
        ));
        assert_eq!(s.len(), 28);
    }

    /// `JsonSchema` renders a plain `string` and leaks no
    /// length/value policy — no `minLength`/`maxLength`/`pattern`/`format`,
    /// no `example`/`default`/`enum`.
    #[cfg(feature = "secret-schemars")]
    #[test]
    fn json_schema_is_plain_string_no_policy_leak() {
        let schema = schemars::schema_for!(SecretString);
        let v = serde_json::to_value(&schema).unwrap();
        assert_eq!(v["type"], serde_json::json!("string"));
        for forbidden in [
            "minLength",
            "maxLength",
            "pattern",
            "format",
            "example",
            "default",
            "enum",
        ] {
            assert!(
                v.get(forbidden).is_none(),
                "schema leaked `{forbidden}`: {v}"
            );
        }
        // Any description present must carry no example/secret value.
        if let Some(desc) = v.get("description").and_then(|d| d.as_str()) {
            assert!(!desc.contains("horse"));
        }
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
        // pass it to `region::lock` or panic.
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

    /// Regression: releasing a lock the OS reports as not held must not
    /// panic or abort. `region::LockGuard`, which [`PageLock`] exists to
    /// avoid, runs `debug_assert!(result.is_ok())` there and so aborts a
    /// debug build on exactly the `Err` this test provokes.
    ///
    /// Drives the failure deterministically by releasing a never-mapped
    /// page instead of a shared one: on Linux `munlock` *succeeds* on a
    /// mapped-but-unlocked region, so page sharing alone cannot produce
    /// the error here — it is `VirtualUnlock`/`ERROR_NOT_LOCKED` on Windows
    /// that does. Unmapped is the one input that fails everywhere, and it
    /// yields the very `Err` the old `debug_assert` fired on. The first
    /// assertion pins that trigger, so this cannot rot into a test that
    /// passes because the unlock quietly started succeeding.
    #[test]
    fn page_lock_release_failure_does_not_panic() {
        let page = region::page::size();
        // The second page: page-aligned and below the default
        // `mmap_min_addr` (64 KiB), so no allocation can ever land here.
        let unmapped = page;
        assert!(
            region::unlock(std::ptr::without_provenance::<u8>(unmapped), page).is_err(),
            "test needs a region whose unlock fails; {unmapped:#x} was accepted"
        );

        // Dropping a lock over that region must swallow the error.
        drop(PageLock {
            address: unmapped,
            size: page,
        });
    }

    /// Secrets small enough to share one locked page must all drop
    /// cleanly, whichever order they go in.
    ///
    /// A smoke test on Linux by construction (see
    /// `page_lock_release_failure_does_not_panic` for why the error
    /// cannot be provoked here); it is the Windows arm of CI that would
    /// catch a regression, where the second release of a shared page
    /// reports `ERROR_NOT_LOCKED`.
    #[test]
    fn secrets_sharing_a_page_drop_cleanly() {
        let a = SecretBytes::from_slice(&[0x11u8; 32]);
        let b = SecretBytes::from_slice(&[0x22u8; 32]);
        let c = SecretBytes::zeroed(16);
        // Reverse and interleaved drop order: no ordering may abort.
        drop(b);
        let d = SecretString::new("shares a page with the survivors");
        drop(a);
        drop(d);
        drop(c);

        // Many small secrets alive at once, then released in FIFO order.
        let many: Vec<SecretBytes> = (0..64u8)
            .map(|i| SecretBytes::from_slice(&[i; 8]))
            .collect();
        drop(many);
    }

    /// The locked region can never go stale, because no API reallocates
    /// the buffer: the address recorded at construction still addresses
    /// the live buffer after every in-place mutation.
    ///
    /// This is what stands in for a relock path — unlike egui-facing
    /// secret wrappers, neither type here exposes a growth API, so there
    /// is no window in which the lock could describe a freed allocation.
    /// A future `push`/`extend` method breaks this test, which is the
    /// point: it must arrive with a relock.
    #[test]
    fn locked_region_is_stable_across_mutation() {
        let mut b = SecretBytes::zeroed(64);
        let before = b.expose_secret().as_ptr();
        b.expose_secret_mut().copy_from_slice(&[0x5Au8; 64]);
        assert_eq!(before, b.expose_secret().as_ptr(), "in-place fill moved it");
        b.zeroize();
        assert_eq!(before, b.expose_secret().as_ptr(), "zeroize moved it");

        let mut s = SecretString::new("locked buffer must not move");
        let before = s.expose_secret().as_ptr();
        s.zeroize();
        assert_eq!(before, s.expose_secret().as_ptr(), "zeroize moved it");
    }

    /// Both wrappers are `Send + Sync`. `region::LockGuard` earns those
    /// with its own `unsafe impl`s; [`PageLock`] must match while holding
    /// only a `usize`, since this crate denies `unsafe_code`. Losing
    /// either would silently break cross-thread holders.
    #[test]
    fn secret_wrappers_stay_send_and_sync() {
        static_assertions::assert_impl_all!(SecretString: Send, Sync);
        static_assertions::assert_impl_all!(SecretBytes: Send, Sync);
        static_assertions::assert_impl_all!(PageLock: Send, Sync);
    }

    /// Proves zeroize wipes the buffer. Every read is on a STILL-LIVE
    /// value (no post-free deref / UB); the in-place slice wipe also
    /// proves the bytes go to zero with the length preserved.
    #[test]
    fn manual_zeroize_wipes_live_buffer() {
        let mut b = SecretBytes::from_slice(&[0xABu8; 64]);
        assert!(b.expose_secret().iter().any(|&x| x != 0));
        b.expose_secret_mut().zeroize();
        assert_eq!(b.len(), 64, "in-place wipe must preserve length");
        assert!(
            b.expose_secret().iter().all(|&x| x == 0),
            "SecretBytes buffer not zeroed by manual zeroize"
        );

        // SecretBytes wrapper-level zeroize empties the buffer.
        let mut b2 = SecretBytes::from_slice(&[0xCDu8; 32]);
        b2.zeroize();
        assert!(b2.is_empty(), "SecretBytes::zeroize must empty the buffer");

        // SecretString wrapper-level zeroize empties the buffer; the
        // exposed view holds no residual plaintext.
        let mut s = SecretString::new("sensitive_seed_material");
        assert!(!s.is_empty());
        s.zeroize();
        assert!(s.is_empty(), "SecretString::zeroize must empty the buffer");
        assert_eq!(s.expose_secret(), "");
    }
}
