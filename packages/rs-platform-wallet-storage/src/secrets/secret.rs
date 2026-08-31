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
const DEFAULT_CAPACITY: usize = 4096 - 16;

/// Minimum post-trim byte length for a vault passphrase or Tier-2 password.
///
/// This defense-in-depth floor rejects trivially short inputs but is not a
/// strength estimator. Dictionary checks, UX feedback, and the real entropy
/// policy remain the consumer's responsibility (see `SECRETS.md`).
pub const MIN_PASSPHRASE_LEN: usize = 8;

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
pub struct SecretString {
    buf: GuardedBuf,
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
    /// intermediate unprotected allocation.
    fn from_plaintext(text: &str) -> Self {
        let mut buf = GuardedBuf::new(text.len().max(DEFAULT_CAPACITY));
        buf.as_mut_slice(text.len())
            .copy_from_slice(text.as_bytes());
        Self {
            buf,
            len: text.len(),
        }
    }

    /// An empty, capacity-padded, guarded buffer.
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
        std::str::from_utf8(self.buf.as_slice(self.len)).expect("SecretString holds valid UTF-8")
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

    /// Whether the trimmed secret is shorter than [`MIN_PASSPHRASE_LEN`].
    pub(crate) fn is_below_minimum_passphrase_len(&self) -> bool {
        self.expose_secret().trim().len() < MIN_PASSPHRASE_LEN
    }
}

impl Default for SecretString {
    fn default() -> Self {
        Self {
            buf: GuardedBuf::new(DEFAULT_CAPACITY),
            len: 0,
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
        self.buf.zeroize_all();
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
                // Copy the borrowed bytes straight into guarded memory —
                // no owned `String` is created, so none can linger.
                Ok(SecretString::from_plaintext(v))
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

    /// `Deserialize` routes the value into guarded memory; the result
    /// `ct_eq`s a directly-built secret and has the right length.
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
            regions.push((s.buf.addr(), s.buf.capacity(), format!("SecretString {i}")));
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
        let cap = s.buf.capacity();
        s.zeroize();
        assert!(
            s.buf.as_slice(cap).iter().all(|&b| b == 0),
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
