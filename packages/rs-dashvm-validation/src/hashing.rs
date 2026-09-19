//! Domain-separated SHA-256 hashes of canonical bytes, prepared bytes and bundles.
//!
//! Canonical and prepared hashes are kept apart on purpose: the canonical hash names what the
//! author submitted and is consensus state, the prepared hash names what the engine compiles
//! and changes whenever the preparation generation changes. Every hash is domain separated by a
//! fixed prefix so a canonical module can never collide with a prepared module, a bundle or any
//! other object in the system that happens to hash the same bytes.

use sha2::{Digest, Sha256};
use std::fmt;

/// SHA-256 of the submitted canonical module bytes, before any transformation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonicalHash(pub [u8; 32]);

/// SHA-256 of the prepared (instrumented, custom-section-free) module bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PreparedHash(pub [u8; 32]);

/// SHA-256 over the canonical encoding of a bundle's names, hashes, bindings and entries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BundleDigest(pub [u8; 32]);

const CANONICAL_DOMAIN: &[u8] = b"dashvm-canonical-module-v1";
const PREPARED_DOMAIN: &[u8] = b"dashvm-prepared-module-v1";
const BUNDLE_DOMAIN: &[u8] = b"dashvm-bundle-v1";

impl CanonicalHash {
    /// Hashes submitted canonical bytes.
    pub fn of(canonical_bytes: &[u8]) -> Self {
        Self(domain_hash(CANONICAL_DOMAIN, canonical_bytes))
    }
}

impl PreparedHash {
    /// Hashes prepared bytes.
    pub fn of(prepared_bytes: &[u8]) -> Self {
        Self(domain_hash(PREPARED_DOMAIN, prepared_bytes))
    }
}

impl BundleDigest {
    /// Hashes a bundle's canonical encoding (see `PreparedBundle::digest_input`).
    pub(crate) fn of_encoding(encoding: &[u8]) -> Self {
        Self(domain_hash(BUNDLE_DOMAIN, encoding))
    }
}

fn domain_hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().into()
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

impl fmt::Debug for CanonicalHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CanonicalHash({})", hex(&self.0))
    }
}

impl fmt::Debug for PreparedHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PreparedHash({})", hex(&self.0))
    }
}

impl fmt::Debug for BundleDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BundleDigest({})", hex(&self.0))
    }
}

impl fmt::Display for CanonicalHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex(&self.0))
    }
}

impl fmt::Display for PreparedHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex(&self.0))
    }
}

impl fmt::Display for BundleDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_separate_domains_so_equal_bytes_hash_differently_per_role() {
        let bytes = b"\0asm\x01\0\0\0";
        assert_ne!(CanonicalHash::of(bytes).0, PreparedHash::of(bytes).0);
        assert_ne!(
            CanonicalHash::of(bytes).0,
            BundleDigest::of_encoding(bytes).0
        );
        assert_ne!(
            PreparedHash::of(bytes).0,
            BundleDigest::of_encoding(bytes).0
        );
    }

    #[test]
    fn should_be_stable_across_calls_and_sensitive_to_every_byte() {
        let bytes = b"\0asm\x01\0\0\0";
        assert_eq!(CanonicalHash::of(bytes), CanonicalHash::of(bytes));
        let mut flipped = *bytes;
        flipped[7] ^= 1;
        assert_ne!(CanonicalHash::of(bytes), CanonicalHash::of(&flipped));
    }

    #[test]
    fn should_pin_the_canonical_hash_of_the_empty_module() {
        // A golden value: the domain prefix and the length framing are part of the hash, so
        // changing either is a new hash domain and must be a deliberate change here too.
        let hash = CanonicalHash::of(b"\0asm\x01\0\0\0");
        assert_eq!(
            hash.to_string(),
            "8b5ff8e5db663f2826e72ed24581e875ddebca3916afc51f1bbb64c1be7e4ab7"
        );
    }
}
