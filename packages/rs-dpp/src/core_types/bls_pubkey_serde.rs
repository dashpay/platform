//! Local serde wrapper for `BlsPublicKey<Bls12381G2Impl>` that tolerates
//! owned-string sources (`serde_json::Value`, `platform_value::Value`, and
//! anything routed through serde's `ContentDeserializer` for tagged-enum
//! buffering).
//!
//! ## Why this exists
//!
//! Upstream `blstrs_plus` 0.8.18 (`src/serde_impl.rs:119`) implements the
//! human-readable deserialize path as:
//!
//! ```ignore
//! if d.is_human_readable() {
//!     let hex_str = <&str>::deserialize(d)?;   // borrowed-only
//!     ...
//! }
//! ```
//!
//! `<&str>::deserialize` only succeeds when the deserializer's visitor
//! receives `visit_borrowed_str` — which `serde_json::from_slice` /
//! `serde_json::from_str` provide, but `serde_json::from_value`,
//! `platform_value::from_value`, and `ContentDeserializer` do **not** (they
//! produce owned `String`). Round-tripping a `BlsPublicKey` through any
//! `Value` representation therefore fails with
//! `"invalid type: string ..., expected a borrowed string"`.
//!
//! This is technically a serde compatibility quirk rather than a single
//! crate's bug — but the leaf type is the only place to patch. See plan
//! §10b "Common pattern: serde's `ContentDeserializer` HR-quirk" for
//! the broader narrative.
//!
//! ## How the workaround works
//!
//! A single `deserialize_any` visitor accepts BOTH wire forms upstream emits:
//! the 96-char hex string (`visit_str`) and the 48-byte compressed-G1 form as
//! raw bytes or a `u8` sequence (`visit_bytes` / `visit_seq`). Either way it
//! hex/byte-decodes to the compressed-G1 representation, lifts it to
//! `G1Projective` via `to_curve`, and wraps into `PublicKey<Bls12381G2Impl>`
//! directly (the inner field is `pub`).
//!
//! Driving it via `deserialize_any` (rather than branching on
//! `is_human_readable()`) is what makes it robust to serde's internal-tag
//! `Content` buffer: that buffer's `is_human_readable()` does not reliably
//! match the original deserializer, so a `Value` (non-HR) pubkey could arrive
//! as a byte *sequence* while the old HR branch expected a *string* — which is
//! exactly why the `ValidatorSet` value round-trip test used to be `#[ignore]`d.
//! Accepting both shapes in one visitor sidesteps that, and also sidesteps the
//! upstream borrowed-only `<&str>::deserialize` HR path. Both serde_json and
//! platform_value are self-describing, so `deserialize_any` is safe; bincode
//! never reaches here (it goes through the separate derived `Decode`).
//!
//! Note: `BlsPublicKey<Bls12381G2Impl>` carries a public key on the G1 curve
//! (the `Bls12381G2Impl` name refers to where signatures live, not keys).
//! Compressed G1 = 48 bytes = 96 hex chars.
//!
//! ## When to remove this
//!
//! This wrapper is now self-sufficient (no behavioral dependency on an upstream
//! fix). Once upstream `blstrs_plus` accepts owned strings AND a byte-sequence
//! HR form on its own `Deserialize`, this wrapper and the `serde(with = ...)`
//! annotations on `Validator::public_key` / `ValidatorSetV0::threshold_public_key`
//! can simply be dropped.

use crate::bls_signatures::inner_types::{G1Affine, GroupEncoding, PrimeCurveAffine};
use crate::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey};
use serde::de::Visitor;
use serde::{Deserializer, Serialize, Serializer};
use std::fmt;

/// Compressed-G1 wire size for BLS12-381 (where the public key lives in
/// `Bls12381G2Impl`).
const COMPRESSED_G1_LEN: usize = 48;

pub fn serialize<S: Serializer>(
    pk: &BlsPublicKey<Bls12381G2Impl>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    // Upstream serialize already produces a hex string in HR and a byte tuple
    // in non-HR; both are correct on the wire. Nothing to override here.
    pk.serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BlsPublicKey<Bls12381G2Impl>, D::Error> {
    // One visitor that accepts BOTH wire forms upstream emits: the
    // human-readable 96-char hex string, and the non-HR 48-byte compressed-G1
    // form (as `bytes` or a `u8` sequence). Driving it via `deserialize_any`
    // makes it robust to serde's internal-tag `Content` buffer, whose
    // `is_human_readable()` does not always match the original deserializer —
    // the bug that previously forced the ValidatorSet value round-trip test to
    // be `#[ignore]`d (the non-HR bytes arrived while the old code's HR branch
    // expected a string → "invalid type: sequence, expected a string"). It also
    // sidesteps the upstream `<&str>::deserialize` borrowed-only HR path (the
    // original reason this wrapper exists). Both serde_json and platform_value
    // are self-describing, so `deserialize_any` is safe; bincode never reaches
    // here (it goes through the separate derived `Decode`).
    deserializer.deserialize_any(BlsPublicKeyVisitor)
}

fn from_compressed_g1_bytes<E: serde::de::Error>(
    bytes: &[u8],
) -> Result<BlsPublicKey<Bls12381G2Impl>, E> {
    if bytes.len() != COMPRESSED_G1_LEN {
        return Err(E::custom(format!(
            "expected {COMPRESSED_G1_LEN} compressed-G1 bytes for public key, got {}",
            bytes.len()
        )));
    }
    let mut compressed = <G1Affine as GroupEncoding>::Repr::default();
    compressed.as_mut().copy_from_slice(bytes);
    let affine = Option::<G1Affine>::from(G1Affine::from_bytes(&compressed))
        .ok_or_else(|| E::custom("not a valid compressed G1 point"))?;
    Ok(BlsPublicKey::<Bls12381G2Impl>(affine.to_curve()))
}

struct BlsPublicKeyVisitor;

impl<'de> Visitor<'de> for BlsPublicKeyVisitor {
    type Value = BlsPublicKey<Bls12381G2Impl>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "a {}-char hex string or {} compressed-G1 bytes",
            COMPRESSED_G1_LEN * 2,
            COMPRESSED_G1_LEN
        )
    }

    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
        if s.len() != COMPRESSED_G1_LEN * 2 {
            return Err(E::custom(format!(
                "expected {} hex chars for compressed G1 public key, got {}",
                COMPRESSED_G1_LEN * 2,
                s.len()
            )));
        }
        let mut bytes = [0u8; COMPRESSED_G1_LEN];
        for (i, slot) in bytes.iter_mut().enumerate() {
            let hi = hex_nibble(s.as_bytes()[i * 2]).map_err(E::custom)?;
            let lo = hex_nibble(s.as_bytes()[i * 2 + 1]).map_err(E::custom)?;
            *slot = (hi << 4) | lo;
        }
        from_compressed_g1_bytes(&bytes)
    }

    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        from_compressed_g1_bytes(v)
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut bytes = Vec::with_capacity(COMPRESSED_G1_LEN);
        while let Some(b) = seq.next_element::<u8>()? {
            bytes.push(b);
        }
        from_compressed_g1_bytes(&bytes)
    }
}

fn hex_nibble(c: u8) -> Result<u8, &'static str> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err("invalid hex character in compressed G1 public key"),
    }
}

/// `Option<BlsPublicKey<Bls12381G2Impl>>` variant for fields like
/// `Validator::public_key`.
pub mod option {
    use super::*;

    pub fn serialize<S: Serializer>(
        opt: &Option<BlsPublicKey<Bls12381G2Impl>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // Option<T>'s built-in Serialize delegates to T's Serialize, which
        // is the upstream BlsPublicKey impl — already correct.
        opt.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<BlsPublicKey<Bls12381G2Impl>>, D::Error> {
        struct OptionVisitor;

        impl<'de> Visitor<'de> for OptionVisitor {
            type Value = Option<BlsPublicKey<Bls12381G2Impl>>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Option<BlsPublicKey<Bls12381G2Impl>>")
            }

            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D2: Deserializer<'de>>(
                self,
                inner: D2,
            ) -> Result<Self::Value, D2::Error> {
                super::deserialize(inner).map(Some)
            }
        }

        deserializer.deserialize_option(OptionVisitor)
    }
}
