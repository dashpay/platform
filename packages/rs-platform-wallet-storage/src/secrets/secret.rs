//! Zeroizing secret wrappers: [`SecretString`] for UTF-8 secrets and
//! [`SecretBytes`] for byte secrets (seeds, xprivs, KDF output, AEAD
//! keys, decrypted plaintext). Both have a redacting `Debug`, no
//! `Display`/`Deref`/`Serialize`, a full buffer wipe on drop, and live in
//! guard-paged, `mlock`ed memory (CWE-316) supplied by
//! [`GuardedBuf`](super::guarded::GuardedBuf).
//!
//! Each secret owns its data pages outright, so no two live secrets share
//! a page and freeing one can never unlock memory another still holds.
//! The cost is at least a page per secret, which is what sizes
//! [`MAX_SECRET_LEN`](super::MAX_SECRET_LEN) against a constrained
//! `RLIMIT_MEMLOCK`.

use std::fmt;

use subtle::ConstantTimeEq;
use zeroize::Zeroize;

use super::guarded::GuardedBuf;

/// Pre-allocation capacity for [`SecretString`] buffers.
///
/// `memsec` prefixes every allocation with a 16-byte canary and rounds
/// the total up to whole pages, so 4080 bytes is the largest payload that
/// still fits a single 4 KiB data page — ample for any passphrase or
/// 24-word mnemonic, and one page is the minimum a guarded allocation can
/// cost regardless.
///
/// A guarded page is also the minimum a *non-empty* secret can cost, so a
/// 32-byte key occupies one whole locked page. That overhead is inherent
/// to memsec's page-granular isolation and is accepted as the price of the
/// no-shared-page guarantee; the only case worth avoiding is the empty
/// one, which allocates nothing at all.
const DEFAULT_CAPACITY: usize = 4096 - 16;

/// Minimum post-trim byte length for a vault passphrase or Tier-2 password.
///
/// This defense-in-depth floor rejects trivially short inputs but is not a
/// strength estimator. Dictionary checks, UX feedback, and the real entropy
/// policy remain the consumer's responsibility (see `SECRETS.md`).
pub const MIN_PASSPHRASE_LEN: usize = 8;

/// Maximum byte length for a vault passphrase or Tier-2 object password.
///
/// Equal to the largest payload that still fits one guarded page.
/// Passphrases are held resident for a store's whole lifetime and up to
/// three are live at once during a re-protect, so this ceiling is what
/// keeps them a fixed one-page row in the locked-memory budget documented
/// at [`MAX_SECRET_LEN`](crate::secrets::MAX_SECRET_LEN) instead of an
/// unbounded one. Far above any human-typed passphrase.
pub const MAX_PASSPHRASE_LEN: usize = DEFAULT_CAPACITY;

/// Zeroize-on-drop wrapper for secret UTF-8 strings (BIP-39 mnemonic,
/// `EncryptedFileStore` passphrase).
///
/// Read access is [`expose_secret`] only; equality goes through
/// [`subtle::ConstantTimeEq`] (`==` is forbidden so bridge code cannot
/// inherit a non-constant-time path). `Display`/`Deref`/`Serialize`/`Eq`
/// are deliberately absent, `Debug` is redacted, and the buffer wipes
/// over its full capacity on drop and lives in guarded, `mlock`ed pages.
///
/// [`expose_secret`]: SecretString::expose_secret
///
/// ```compile_fail
/// use platform_wallet_storage::secrets::SecretString;
/// let a = SecretString::new("pw");
/// let b = SecretString::new("pw");
/// let _ = a == b; // `==` on SecretString is forbidden; use ConstantTimeEq::ct_eq
/// ```
#[derive(Default)]
pub struct SecretString {
    /// `None` for an empty secret: a guarded allocation costs a whole
    /// page, which is pure waste when there is nothing to protect.
    buf: Option<GuardedBuf>,
    /// Byte length of the UTF-8 plaintext held at the start of `buf`.
    len: usize,
}

impl SecretString {
    /// Wrap a string, copying it into guarded memory and zeroizing the
    /// source so no unprotected copy outlives the call.
    pub fn new(s: impl Into<String>) -> Self {
        let mut source: String = s.into();
        let secret = Self::from_plaintext(&source);
        // Do not remove: wipes the moved-in plaintext source before it
        // drops. A direct freed-buffer scan would be a use-after-free, so
        // the test `secret_string_new_zeroizes_string_source` pins the
        // `String::zeroize` primitive and this call site instead.
        source.zeroize();
        secret
    }

    /// Copy `text` straight into a fresh guarded buffer, with no
    /// intermediate unprotected allocation. Empty text allocates nothing.
    fn from_plaintext(text: &str) -> Self {
        if text.is_empty() {
            return Self::default();
        }
        let mut buf = GuardedBuf::new(text.len().max(DEFAULT_CAPACITY));
        buf.as_mut_slice(text.len())
            .copy_from_slice(text.as_bytes());
        Self {
            buf: Some(buf),
            len: text.len(),
        }
    }

    /// An empty secret, holding no allocation.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Borrow the plaintext. The only read path.
    ///
    /// # Panics
    ///
    /// Panics if the buffer does not hold valid UTF-8. Only whole `&str`
    /// values are ever written into it, so that signals a bug in this
    /// module rather than a recoverable condition.
    pub fn expose_secret(&self) -> &str {
        let Some(buf) = &self.buf else {
            return "";
        };
        std::str::from_utf8(buf.as_slice(self.len)).expect("SecretString holds valid UTF-8")
    }

    /// Secret length in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the secret is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// A new `SecretString` holding the whitespace-trimmed content,
    /// keeping the trimmed copy inside the wrapper.
    pub fn trimmed(&self) -> Self {
        Self::from_plaintext(self.expose_secret().trim())
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
        self.expose_secret().trim().is_empty()
    }

    /// Replace the plaintext bytes in `range` with `replacement`, growing
    /// the guarded buffer when it does not fit.
    ///
    /// The type's single mutation primitive, mirroring
    /// [`String::replace_range`]: insertion is an empty range, deletion an
    /// empty `replacement`, whole-buffer replacement `..`. Bytes vacated
    /// by a shrinking edit are wiped rather than merely orphaned past the
    /// length, and a buffer outgrown by a growing edit is wiped before it
    /// is freed. Byte offsets, not character indices — derive them from
    /// [`expose_secret`](Self::expose_secret).
    ///
    /// Capacity only grows: a shrinking edit keeps its allocation, since
    /// giving it back would cost another copy and another lock cycle for
    /// no security gain, and the trailing capacity is wiped regardless.
    ///
    /// # Panics
    ///
    /// Panics if `range` is inverted, ends past [`len`](Self::len), or has
    /// an endpoint off a UTF-8 character boundary — each a caller bug, as
    /// for [`String::replace_range`]. The secret is left unmodified, and
    /// the panic message names only indices, never a byte of plaintext.
    ///
    /// # Memory
    ///
    /// No length ceiling is applied here: a value type cannot report a
    /// refusal, and both real trust boundaries already enforce one — the
    /// UI that accepts the input, and
    /// [`MAX_PLAINTEXT_LEN`](crate::secrets::MAX_PLAINTEXT_LEN) at the
    /// vault write. The consequence is that growth driven by untrusted
    /// input (a paste into a text field) is unbounded `mlock`ed,
    /// page-rounded memory, and the locks fail open once `RLIMIT_MEMLOCK`
    /// runs out. Bound such input at your own boundary.
    ///
    /// ```
    /// use platform_wallet_storage::secrets::SecretString;
    /// let mut s = SecretString::new("hello");
    /// s.replace_range(5.., " world");
    /// assert_eq!(s.expose_secret(), "hello world");
    /// ```
    pub fn replace_range<R: std::ops::RangeBounds<usize>>(&mut self, range: R, replacement: &str) {
        let (start, end) = self.resolve_range(range);
        let replacement = replacement.as_bytes();
        let old_len = self.len;
        // `resolve_range` guarantees `start <= end <= old_len`.
        let new_len = old_len - (end - start) + replacement.len();
        self.reserve(new_len);

        let Some(buf) = &mut self.buf else {
            // `reserve` allocates whenever `new_len > 0`, so an absent
            // buffer means the edit was a no-op on an empty secret.
            return;
        };
        let bytes = buf.as_mut_slice(old_len.max(new_len));
        bytes.copy_within(end..old_len, start + replacement.len());
        bytes[start..start + replacement.len()].copy_from_slice(replacement);
        // A shrinking edit leaves the old tail above the new length; wipe
        // it now rather than wait for an overwrite that may never come.
        if new_len < old_len {
            bytes[new_len..old_len].zeroize();
        }
        self.len = new_len;
    }

    /// Resolve `range` against the live plaintext, panicking on any shape
    /// [`String::replace_range`] would reject.
    fn resolve_range<R: std::ops::RangeBounds<usize>>(&self, range: R) -> (usize, usize) {
        use std::ops::Bound;
        let start = match range.start_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&i) => i.saturating_add(1),
            Bound::Excluded(&i) => i,
            Bound::Unbounded => self.len,
        };
        check_edit_range(self.expose_secret(), start, end);
        (start, end)
    }

    /// Grow the buffer to hold at least `needed` bytes, preserving the
    /// live plaintext. A no-op when it already fits.
    fn reserve(&mut self, needed: usize) {
        let capacity = self.buf.as_ref().map_or(0, GuardedBuf::capacity);
        if needed == 0 || needed <= capacity {
            return;
        }
        let mut grown =
            GuardedBuf::new(needed.max(capacity.saturating_mul(2)).max(DEFAULT_CAPACITY));
        if let Some(old) = &self.buf {
            grown
                .as_mut_slice(self.len)
                .copy_from_slice(old.as_slice(self.len));
        }
        // Assigning drops the outgrown buffer, wiping it before
        // `memsec::free` hands its pages back.
        self.buf = Some(grown);
    }

    /// Whether the trimmed secret is shorter than [`MIN_PASSPHRASE_LEN`].
    pub(crate) fn is_below_minimum_passphrase_len(&self) -> bool {
        self.expose_secret().trim().len() < MIN_PASSPHRASE_LEN
    }

    /// Whether the secret is longer than [`MAX_PASSPHRASE_LEN`].
    ///
    /// Untrimmed, unlike the floor: the whole value occupies guarded
    /// pages whether or not its edges are whitespace, and it is the page
    /// cost this ceiling exists to bound.
    pub(crate) fn exceeds_maximum_passphrase_len(&self) -> bool {
        self.len > MAX_PASSPHRASE_LEN
    }
}

/// Reject an inverted, out-of-bounds, or non-character-boundary edit
/// range over `text`.
///
/// **Every message carries indices only.** Slicing `text` to let std
/// raise the error instead — `&text[start..end]`, `str::split_at`,
/// `String::replace_range` — would print the surrounding characters,
/// which here are plaintext, onto stderr and into every log capture
/// (CWE-209/CWE-532). That is why the bounds are hand-rolled from
/// [`str::is_char_boundary`], and why a refactor back to slicing would
/// be a vulnerability rather than a simplification.
///
/// Takes the text rather than a `SecretString` so it never calls
/// `expose_secret` itself, keeping `tests/secrets_guard.rs`'s
/// sink-near-plaintext scan honest instead of merely quiet.
fn check_edit_range(text: &str, start: usize, end: usize) {
    assert!(
        start <= end,
        "secret edit range start {start} exceeds end {end}"
    );
    assert!(
        end <= text.len(),
        "secret edit range end {end} exceeds secret length {}",
        text.len()
    );
    assert!(
        text.is_char_boundary(start),
        "secret edit range start {start} is not a character boundary"
    );
    assert!(
        text.is_char_boundary(end),
        "secret edit range end {end} is not a character boundary"
    );
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
        if let Some(buf) = &mut self.buf {
            buf.zeroize_all();
        }
        self.len = 0;
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecretString {
    /// Copies straight into guarded memory, with no transient `String`.
    fn from(s: &str) -> Self {
        Self::from_plaintext(s)
    }
}

/// Deserialize a UTF-8 secret (a vault passphrase or a Tier-2 object
/// password arriving via config) straight into guarded memory, so no
/// intermediate plaintext buffer **we own** lingers (CWE-316).
///
/// Gated behind the default-off `serde` feature, which gates the IMPL and
/// not the dep — `secrets` already compiles serde for the vault format, so
/// a consumer that has not asked for this simply does not get it. There is
/// deliberately **no** `Serialize` companion (a secret is read-from-config,
/// never written back / round-tripped / logged), so this type cannot leak
/// out through serde under any feature combination.
///
/// **Residual (documented, not closeable here):** the deserializer's own
/// input buffer holds the cleartext before this visitor runs and is
/// outside `SecretString`'s ownership, so it cannot be wiped here — feed
/// secrets from a zeroizing source. Mirrors the Argon2 `Block` residual
/// noted at `crypto::derive_key`.
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for SecretString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        /// Refuse a value past [`MAX_PASSPHRASE_LEN`] before it reaches
        /// guarded memory.
        ///
        /// Config is the one construction path this crate does not
        /// control the size of, and an over-long value here would lock
        /// page-rounded memory that the budget at `MAX_SECRET_LEN` does
        /// not account for. Reports the length only — never the value.
        fn reject_oversized<E: serde::de::Error>(len: usize) -> Result<(), E> {
            if len > MAX_PASSPHRASE_LEN {
                return Err(E::invalid_length(
                    len,
                    &"a secret within MAX_PASSPHRASE_LEN",
                ));
            }
            Ok(())
        }

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
                reject_oversized(v.len())?;
                // Copy the borrowed bytes straight into guarded memory —
                // no owned `String` is created, so none can linger.
                Ok(SecretString::from_plaintext(v))
            }

            fn visit_string<E>(self, mut v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if let Err(e) = reject_oversized(v.len()) {
                    // The rejected value is still an unprotected copy we
                    // own; wipe it rather than let the error path drop it
                    // intact.
                    v.zeroize();
                    return Err(e);
                }
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
/// Unconditional under `secrets`: the schema carries neither a policy nor
/// a value, so there is nothing to opt out of. Pulls in no
/// `Serialize`/`Display` path.
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
/// and lives in guarded, `mlock`ed pages.
///
/// Unlike [`SecretString`] the buffer is sized exactly to the secret:
/// this type never grows, and key material is small and known, so padding
/// every 32-byte key out to a full page would buy nothing. An empty
/// `SecretBytes` holds no allocation at all.
///
/// ```compile_fail
/// use platform_wallet_storage::secrets::SecretBytes;
/// let a = SecretBytes::new(vec![0u8; 32]);
/// let b = SecretBytes::new(vec![0u8; 32]);
/// let _ = a == b; // `==` on SecretBytes is forbidden; use ConstantTimeEq::ct_eq
/// ```
pub struct SecretBytes {
    /// `None` for an empty secret: a guarded allocation costs a whole
    /// page, which is pure waste when there is nothing to protect.
    buf: Option<GuardedBuf>,
    len: usize,
}

impl SecretBytes {
    /// Wrap a byte vector, copying it into guarded memory and zeroizing
    /// the source.
    ///
    /// The copy is unavoidable: guarded memory comes from a dedicated
    /// allocator, so a `Vec`'s own allocation can never *become* the
    /// protected buffer. Wiping `bytes` is therefore load-bearing — the
    /// caller's plaintext would otherwise be left on the ordinary heap.
    pub fn new(mut bytes: Vec<u8>) -> Self {
        let secret = Self::from_slice(&bytes);
        // Do not remove: without this the general-purpose heap keeps an
        // unprotected copy of every secret that passes through here.
        bytes.zeroize();
        secret
    }

    /// A zeroed buffer of `len` bytes in guarded memory — for in-place
    /// fills (KDF output, decrypt target).
    pub fn zeroed(len: usize) -> Self {
        Self {
            buf: (len > 0).then(|| GuardedBuf::new(len)),
            len,
        }
    }

    /// Copy a borrowed slice into a fresh wrapper. Deliberate, explicit
    /// copy — the only way to duplicate secret bytes.
    pub fn from_slice(bytes: &[u8]) -> Self {
        let mut secret = Self::zeroed(bytes.len());
        secret.expose_secret_mut().copy_from_slice(bytes);
        secret
    }

    /// Borrow the plaintext bytes. The only read path.
    pub fn expose_secret(&self) -> &[u8] {
        match &self.buf {
            Some(buf) => buf.as_slice(self.len),
            None => &[],
        }
    }

    /// Mutably borrow the plaintext bytes (in-place KDF/decrypt fill).
    pub fn expose_secret_mut(&mut self) -> &mut [u8] {
        let len = self.len;
        match &mut self.buf {
            Some(buf) => buf.as_mut_slice(len),
            None => &mut [],
        }
    }

    /// Secret length in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the secret is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl ConstantTimeEq for SecretBytes {
    /// Constant-time compare, no length early-return. Unequal lengths
    /// yield `0` without leaking *where* they differ; only the non-secret
    /// length is observable.
    fn ct_eq(&self, other: &Self) -> subtle::Choice {
        self.expose_secret().ct_eq(other.expose_secret())
    }
}

impl Zeroize for SecretBytes {
    /// Wipe the buffer in place on a live value. `Drop` runs the same
    /// wipe automatically; this lets a holder zeroize early.
    fn zeroize(&mut self) {
        if let Some(buf) = &mut self.buf {
            buf.zeroize_all();
        }
        self.len = 0;
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretBytes([REDACTED; {}])", self.len)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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

    /// Two sound checks (the moved-in source is gone by the time `new`
    /// returns, so scanning it would be a use-after-free): (1)
    /// `String::zeroize` empties a buffer — the primitive `new` relies on;
    /// (2) `new` copies the content into the wrapper faithfully. That
    /// `new` actually calls `source.zeroize()` is pinned by the
    /// do-not-remove comment at that call site, not asserted here.
    #[test]
    fn secret_string_new_zeroizes_string_source() {
        let mut source = String::from("super secret seed material");
        source.zeroize();
        assert!(source.is_empty(), "String::zeroize must empty the source");
        let s = SecretString::new(String::from("super secret seed material"));
        assert_eq!(s.expose_secret(), "super secret seed material");
    }

    /// The `SecretBytes::new` counterpart. Guarded memory cannot adopt a
    /// `Vec`'s allocation, so `new` copies and must wipe the original;
    /// without that an unprotected duplicate of every secret would stay on
    /// the ordinary heap. Pinned the same way as the `String` source.
    #[test]
    fn secret_bytes_new_zeroizes_vec_source() {
        let mut source = vec![0xABu8; 64];
        source.zeroize();
        assert!(source.is_empty(), "Vec::zeroize must empty the source");
        let b = SecretBytes::new(vec![0xABu8; 64]);
        assert_eq!(b.expose_secret(), &[0xABu8; 64]);
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

    /// Mirrors `empty_secret_bytes_constructs_without_allocating`: the two
    /// sibling wrappers must answer "what does an empty secret cost?" the
    /// same way. `open_unprotected` holds an empty passphrase resident for
    /// a whole store's lifetime, so a padded buffer here would lock a page
    /// per keyless vault to protect nothing.
    #[test]
    fn empty_secret_string_constructs_without_allocating() {
        for s in [
            SecretString::empty(),
            SecretString::default(),
            SecretString::new(""),
            SecretString::from(""),
        ] {
            assert!(s.is_empty());
            assert_eq!(s.expose_secret(), "");
            assert!(s.buf.is_none(), "an empty secret must hold no allocation");
        }
        // Wiping a populated secret keeps its allocation; only
        // construction decides whether one exists.
        let mut s = SecretString::new("something worth protecting");
        s.zeroize();
        assert!(s.is_empty());
        assert!(s.buf.is_some());
    }

    /// The passphrase ceiling counts raw bytes, not trimmed ones — the
    /// whole value occupies the guarded pages the ceiling bounds.
    #[test]
    fn passphrase_ceiling_boundary_is_inclusive() {
        let at_cap = SecretString::new("x".repeat(MAX_PASSPHRASE_LEN));
        let over = SecretString::new("x".repeat(MAX_PASSPHRASE_LEN + 1));
        let padded = SecretString::new(format!("{}  ", "x".repeat(MAX_PASSPHRASE_LEN - 1)));
        assert!(!at_cap.exceeds_maximum_passphrase_len());
        assert!(over.exceeds_maximum_passphrase_len());
        assert!(
            padded.exceeds_maximum_passphrase_len(),
            "whitespace still costs guarded bytes, so it counts"
        );
    }

    /// `Send`/`Sync` exercised for real, not just asserted: a secret is
    /// built on one thread, moved to another, read and dropped there,
    /// while a second secret is shared by reference across two more.
    #[test]
    fn secrets_cross_thread_boundaries() {
        let moved = SecretString::new("moved across a thread boundary");
        let handle = std::thread::spawn(move || {
            assert_eq!(moved.expose_secret(), "moved across a thread boundary");
            assert_eq!(moved.len(), 30);
            // Dropped here: the wipe and free run off the allocating thread.
        });
        handle.join().expect("moved secret must survive the send");

        let shared = std::sync::Arc::new(SecretBytes::from_slice(&[0x7Eu8; 64]));
        let readers: Vec<_> = (0..2)
            .map(|_| {
                let shared = std::sync::Arc::clone(&shared);
                std::thread::spawn(move || {
                    assert!(bool::from(
                        shared.ct_eq(&SecretBytes::from_slice(&[0x7Eu8; 64]))
                    ));
                })
            })
            .collect();
        for reader in readers {
            reader.join().expect("shared secret must survive the share");
        }
        assert_eq!(shared.expose_secret(), &[0x7Eu8; 64]);
    }

    /// Multi-byte content survives the copy into guarded memory intact.
    /// The buffer is raw bytes now, so a botched length would corrupt
    /// UTF-8 and trip `expose_secret`'s validity check.
    #[test]
    fn secret_string_handles_multibyte_utf8() {
        let text = "héllo wörld 🦡";
        let s = SecretString::new(text);
        assert_eq!(s.expose_secret(), text);
        assert_eq!(s.len(), text.len());
        assert!(s.len() > text.chars().count(), "multi-byte chars expected");
    }

    /// A secret larger than the padded default still round-trips: the
    /// buffer sizes to the content instead of truncating it.
    #[test]
    fn secret_string_larger_than_default_capacity() {
        let long = "x".repeat(DEFAULT_CAPACITY + 1234);
        let s = SecretString::new(long.clone());
        assert_eq!(s.expose_secret(), long);
        assert_eq!(s.len(), DEFAULT_CAPACITY + 1234);
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

    /// Regression: the `serde` DEP is on under `secrets`, yet the
    /// `Deserialize` IMPL stays ABSENT because the `serde` FEATURE gates
    /// the impl rather than the dep — proving the default-off gate is
    /// satisfiable even while serde is compiled.
    #[cfg(not(feature = "serde"))]
    #[test]
    fn deserialize_absent_without_the_serde_feature_even_though_the_dep_is_on() {
        static_assertions::assert_not_impl_any!(
            SecretString: serde::de::DeserializeOwned
        );
    }

    /// With the `serde` feature on, the `Deserialize` impl is present (and
    /// `Serialize` is still absent — see the always-on test).
    #[cfg(feature = "serde")]
    #[test]
    fn deserialize_present_with_the_serde_feature() {
        static_assertions::assert_impl_all!(SecretString: serde::de::DeserializeOwned);
        static_assertions::assert_not_impl_any!(SecretString: serde::Serialize);
    }

    /// `Deserialize` routes the value into guarded memory; the result
    /// `ct_eq`s a directly-built secret and has the right length.
    #[cfg(feature = "serde")]
    #[test]
    fn deserialize_routes_value_through_zeroizing_constructor() {
        let s: SecretString = serde_json::from_str("\"correct horse battery staple\"").unwrap();
        assert!(bool::from(
            s.ct_eq(&SecretString::new("correct horse battery staple"))
        ));
        assert_eq!(s.len(), 28);
    }

    /// Config is untrusted input, so `Deserialize` refuses a value past
    /// [`MAX_PASSPHRASE_LEN`] before it reaches guarded memory — and the
    /// error message carries the length only, never the value.
    #[cfg(feature = "serde")]
    #[test]
    fn deserialize_rejects_oversized_value() {
        let at_cap = format!("\"{}\"", "a".repeat(MAX_PASSPHRASE_LEN));
        let s: SecretString = serde_json::from_str(&at_cap).expect("the cap itself is accepted");
        assert_eq!(s.len(), MAX_PASSPHRASE_LEN);

        let over = format!("\"{}\"", "z".repeat(MAX_PASSPHRASE_LEN + 1));
        let err = serde_json::from_str::<SecretString>(&over)
            .expect_err("a value past the cap must be refused");
        let rendered = err.to_string();
        assert!(
            rendered.contains(&(MAX_PASSPHRASE_LEN + 1).to_string()),
            "{rendered}"
        );
        assert!(
            !rendered.contains("zzz"),
            "error leaked the value: {rendered}"
        );
    }

    /// `JsonSchema` renders a plain `string` and leaks no
    /// length/value policy — no `minLength`/`maxLength`/`pattern`/`format`,
    /// no `example`/`default`/`enum`.
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
    fn empty_secret_bytes_constructs_without_allocating() {
        // An empty secret has nothing to protect, so it must not burn a
        // guarded page (four pages of mapping) on it.
        let b = SecretBytes::new(Vec::new());
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
        assert_eq!(b.expose_secret(), &[] as &[u8]);
        assert!(b.buf.is_none(), "an empty secret must hold no allocation");
        let z = SecretBytes::zeroed(0);
        assert!(z.is_empty());
        assert!(z.buf.is_none());
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

    /// Both wrappers are `Send + Sync`. `GuardedBuf` owns the raw pointer
    /// and the `unsafe impl`s that earn these, so losing them here would
    /// silently break cross-thread holders.
    #[test]
    fn secret_wrappers_stay_send_and_sync() {
        static_assertions::assert_impl_all!(SecretString: Send, Sync);
        static_assertions::assert_impl_all!(SecretBytes: Send, Sync);
    }

    /// The structural guarantee: no two live secrets touch a common page.
    ///
    /// A shared page means one secret's lifetime governs its neighbour's
    /// anti-swap protection — freeing the first unlocks memory the second
    /// still holds. Guarded allocation makes that impossible rather than
    /// merely survivable, which is the point of the design. This fails on
    /// a `String`/`Vec`-backed layout, where the general-purpose allocator
    /// packs several secrets into one page.
    #[test]
    fn secrets_never_share_a_page() {
        let page = region::page::size();
        let strings: Vec<SecretString> = (0..32)
            .map(|i| SecretString::new(format!("s{i}")))
            .collect();
        let bytes: Vec<SecretBytes> = (0..32)
            .map(|i| SecretBytes::from_slice(&[i as u8; 48]))
            .collect();

        // (start address, payload capacity, label) for every live buffer.
        let mut regions: Vec<(usize, usize, String)> = Vec::new();
        for (i, s) in strings.iter().enumerate() {
            let buf = s.buf.as_ref().expect("a non-empty secret is allocated");
            regions.push((buf.addr(), buf.capacity(), format!("SecretString {i}")));
        }
        for (i, b) in bytes.iter().enumerate() {
            let buf = b.buf.as_ref().expect("a 48-byte secret is allocated");
            regions.push((buf.addr(), buf.capacity(), format!("SecretBytes {i}")));
        }

        let mut owner: HashMap<usize, String> = HashMap::new();
        for (start, cap, label) in regions {
            assert_eq!(
                (start + cap) % page,
                0,
                "{label}: the buffer must end on a page boundary, where memsec's guard page begins"
            );
            for page_index in (start / page)..=((start + cap - 1) / page) {
                if let Some(previous) = owner.insert(page_index, label.clone()) {
                    panic!("{previous} and {label} share page {page_index:#x}");
                }
            }
        }
    }

    /// Insertion, deletion and replacement in place, all within the
    /// pre-allocated capacity so no reallocation is involved.
    #[test]
    fn replace_range_inserts_deletes_and_replaces() {
        let mut s = SecretString::new("bcd");
        s.replace_range(0..0, "a");
        assert_eq!(s.expose_secret(), "abcd");
        s.replace_range(4..4, "e");
        assert_eq!(s.expose_secret(), "abcde");
        s.replace_range(2..2, "XY");
        assert_eq!(s.expose_secret(), "abXYcde");

        s.replace_range(2..4, "");
        assert_eq!(s.expose_secret(), "abcde");
        s.replace_range(1..4, "-");
        assert_eq!(s.expose_secret(), "a-e");
        assert_eq!(s.len(), 3);
    }

    /// One differential case: a label, the edit applied to `String`, and the
    /// same edit applied to `SecretString`.
    type DifferentialRangeCase = (
        &'static str,
        Box<dyn Fn(&mut String)>,
        Box<dyn Fn(&mut SecretString)>,
    );

    /// One rejected-range case: a label and the edit expected to panic.
    type InvalidRangeCase = (&'static str, Box<dyn Fn(&mut SecretString)>);

    /// Every `RangeBounds` shape resolves the way `String::replace_range`
    /// resolves it — differential, so the contract is pinned to std's
    /// rather than to this implementation's own behaviour.
    #[test]
    fn replace_range_bounds_match_std() {
        let cases: Vec<DifferentialRangeCase> = vec![
            (
                "..",
                Box::new(|s: &mut String| s.replace_range(.., "Z")),
                Box::new(|s: &mut SecretString| s.replace_range(.., "Z")),
            ),
            (
                "2..",
                Box::new(|s: &mut String| s.replace_range(2.., "Z")),
                Box::new(|s: &mut SecretString| s.replace_range(2.., "Z")),
            ),
            (
                "..3",
                Box::new(|s: &mut String| s.replace_range(..3, "Z")),
                Box::new(|s: &mut SecretString| s.replace_range(..3, "Z")),
            ),
            (
                "1..=3",
                Box::new(|s: &mut String| s.replace_range(1..=3, "Z")),
                Box::new(|s: &mut SecretString| s.replace_range(1..=3, "Z")),
            ),
            (
                "..=2",
                Box::new(|s: &mut String| s.replace_range(..=2, "Z")),
                Box::new(|s: &mut SecretString| s.replace_range(..=2, "Z")),
            ),
        ];
        for (label, on_std, on_secret) in cases {
            let mut std_string = String::from("abcdef");
            let mut secret = SecretString::new("abcdef");
            on_std(&mut std_string);
            on_secret(&mut secret);
            assert_eq!(
                secret.expose_secret(),
                std_string,
                "range `{label}` diverged"
            );
        }
    }

    /// A shrinking edit wipes the bytes it vacates instead of merely
    /// orphaning them past the length — otherwise a deleted passphrase
    /// character would sit in locked memory until the next overwrite.
    #[test]
    fn replace_range_wipes_the_bytes_it_vacates() {
        let mut s = SecretString::new("secret-tail-KEEPOUT");
        let cap = s.buf.as_ref().unwrap().capacity();
        s.replace_range(6.., "");
        assert_eq!(s.expose_secret(), "secret");

        let buf = s.buf.as_ref().unwrap();
        assert!(
            buf.as_slice(cap)[6..].iter().all(|&b| b == 0),
            "the vacated tail must be wiped, not orphaned past the length"
        );
    }

    /// Multi-byte characters survive splices on both sides of a 2-byte
    /// and a 4-byte character; a botched offset would corrupt UTF-8 and
    /// trip `expose_secret`'s validity check.
    #[test]
    fn replace_range_handles_multibyte_utf8() {
        // "é" is 2 bytes, "🦡" is 4.
        let mut s = SecretString::new("aébc🦡d");
        assert_eq!(s.len(), 1 + 2 + 2 + 4 + 1);
        s.replace_range(1..3, "É");
        assert_eq!(s.expose_secret(), "aÉbc🦡d");
        s.replace_range(5..9, "🦘");
        assert_eq!(s.expose_secret(), "aÉbc🦘d");
        s.replace_range(0..1, "ααα");
        assert_eq!(s.expose_secret(), "αααÉbc🦘d");
    }

    /// Growth past the pre-allocated capacity preserves the content —
    /// both as one large edit and as the long run of small appends a
    /// text widget actually produces.
    #[test]
    fn replace_range_grows_past_capacity() {
        let long = "x".repeat(DEFAULT_CAPACITY + 500);
        let mut one_shot = SecretString::new("seed:");
        one_shot.replace_range(5.., &long);
        // Built in its own statement: `tests/secrets_guard.rs` rejects a
        // formatting sink sharing a statement with `expose_secret`.
        let expected = format!("seed:{long}");
        assert_eq!(one_shot.expose_secret(), expected);

        // An empty secret holds no allocation, so the first edit is also
        // the first allocation.
        let mut typed = SecretString::empty();
        assert!(typed.buf.is_none());
        let mut expected = String::new();
        for i in 0..DEFAULT_CAPACITY + 100 {
            let ch = char::from(b'a' + (i % 26) as u8);
            let at = typed.len();
            typed.replace_range(at.., &ch.to_string());
            expected.push(ch);
        }
        assert_eq!(typed.expose_secret(), expected);
    }

    /// A buffer outgrown by a reallocation is wiped before it is freed,
    /// and the replacement still ends flush against its guard page — the
    /// page-isolation invariant must survive growth.
    #[test]
    fn growth_wipes_the_old_buffer_and_preserves_page_isolation() {
        let mut s = SecretString::new("needle-in-guarded-memory");
        let old_addr = s.buf.as_ref().unwrap().addr();
        let old_cap = s.buf.as_ref().unwrap().capacity();

        s.replace_range(s.len().., &"y".repeat(DEFAULT_CAPACITY));
        let buf = s.buf.as_ref().unwrap();
        assert_ne!(buf.addr(), old_addr, "the edit must have reallocated");
        assert!(buf.capacity() > old_cap);

        let page = region::page::size();
        assert_eq!(
            (buf.addr() + buf.capacity()) % page,
            0,
            "a grown buffer must still abut its guard page"
        );
        // The outgrown allocation was wiped by `GuardedBuf::drop` before
        // `memsec::free` returned its pages; reading it back would be a
        // use-after-free, so `Drop`'s wipe is pinned by
        // `zeroize_wipes_full_capacity_not_just_len` on a live value and
        // by this reallocation going through the same `Drop`.
        let expected = format!("needle-in-guarded-memory{}", "y".repeat(DEFAULT_CAPACITY));
        assert_eq!(s.expose_secret(), expected);
    }

    /// Each rejected range shape panics, matching `String::replace_range`.
    #[test]
    fn replace_range_panics_on_invalid_ranges() {
        let cases: Vec<InvalidRangeCase> = vec![
            (
                "inverted",
                // The inversion is the case under test — `String::replace_range`
                // panics on it, and so must this. Reversing the range would
                // delete the scenario.
                #[expect(clippy::reversed_empty_ranges, reason = "the case under test")]
                Box::new(|s: &mut SecretString| s.replace_range(3..1, "")),
            ),
            (
                "end past len",
                Box::new(|s: &mut SecretString| s.replace_range(0..99, "")),
            ),
            (
                "start off a char boundary",
                Box::new(|s: &mut SecretString| s.replace_range(2..4, "")),
            ),
            (
                "end off a char boundary",
                Box::new(|s: &mut SecretString| s.replace_range(1..2, "")),
            ),
        ];
        for (label, edit) in cases {
            // "é" occupies bytes 1..3, so 2 is mid-character.
            let mut s = SecretString::new("aéb");
            let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| edit(&mut s)));
            assert!(caught.is_err(), "`{label}` must panic");
        }
    }

    /// The regression guard for CWE-209/CWE-532: a rejected range must
    /// not print the secret. `str`'s own slicing panic embeds a snippet
    /// of the surrounding string, so reaching the panic through
    /// `&s[range]` — the obvious implementation — would put plaintext on
    /// stderr and into every log capture.
    #[test]
    fn replace_range_panic_message_carries_no_plaintext() {
        // Bound as a literal, never read back out of the secret: pairing
        // `expose_secret` with an assertion here would trip
        // `tests/secrets_guard.rs`.
        let plaintext = "correct-horse-battery-staple";
        let mut s = SecretString::new(plaintext);

        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            s.replace_range(0..9_999, "")
        }))
        .expect_err("an out-of-range edit must panic");
        std::panic::set_hook(previous);

        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|m| (*m).to_string()))
            .expect("panic payload must be a string");
        assert!(
            !message.contains(plaintext) && !message.contains("horse"),
            "panic message leaked the secret: {message}"
        );
        assert!(
            message.contains("9999"),
            "panic message should name the offending index: {message}"
        );
    }

    /// Validation runs before the first byte moves, so a caught panic
    /// leaves the secret exactly as it was — a `catch_unwind`-ing GUI
    /// host must never observe a half-spliced, invalid-UTF-8 buffer.
    #[test]
    fn a_rejected_edit_leaves_the_secret_untouched() {
        let mut s = SecretString::new("aéb");
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            s.replace_range(2..3, "zzz")
        }));
        std::panic::set_hook(previous);

        assert!(caught.is_err());
        assert_eq!(s.expose_secret(), "aéb");
        assert_eq!(s.len(), 4);
    }

    /// An empty replacement into an empty range is a no-op, including on
    /// an empty secret that holds no allocation at all.
    #[test]
    fn replace_range_empty_into_empty_is_a_no_op() {
        let mut empty = SecretString::empty();
        empty.replace_range(0..0, "");
        assert_eq!(empty.expose_secret(), "");
        assert!(
            empty.buf.is_none(),
            "a no-op edit must not allocate a guarded page"
        );

        let mut s = SecretString::new("unchanged");
        s.replace_range(4..4, "");
        assert_eq!(s.expose_secret(), "unchanged");
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

    /// The wipe clears the WHOLE buffer, not just the bytes below the old
    /// length. Guarded allocation is what makes this directly checkable —
    /// the buffer is still alive and inspectable through a safe accessor,
    /// where a freed `String` allocation could only be scanned via
    /// use-after-free.
    #[test]
    fn zeroize_wipes_full_capacity_not_just_len() {
        let mut s = SecretString::new("sensitive_seed_material");
        s.zeroize();
        let buf = s.buf.as_ref().expect("allocation survives zeroize");
        assert!(
            buf.as_slice(buf.capacity()).iter().all(|&b| b == 0),
            "zeroize left residue in the buffer's trailing capacity"
        );

        let mut b = SecretBytes::from_slice(&[0xEEu8; 200]);
        b.zeroize();
        let buf = b.buf.as_ref().expect("allocation survives zeroize");
        assert!(
            buf.as_slice(buf.capacity()).iter().all(|&x| x == 0),
            "zeroize left residue in SecretBytes' buffer"
        );
    }
}
