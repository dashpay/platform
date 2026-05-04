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
//! On the HR path we read the value as an owned `String`, hex-decode it to
//! the 48-byte compressed-G1 representation, then construct
//! `G1Affine::from_compressed(...)`, lift it to `G1Projective` via `to_curve`,
//! and wrap into `PublicKey<Bls12381G2Impl>` directly (the inner field is
//! `pub`). This bypasses the entire upstream HR deserialize chain — including
//! the `<&str>::deserialize` call — without touching the upstream crate. On
//! the non-HR path we delegate straight to upstream, which already works
//! (it goes through `deserialize_tuple` of bytes; no borrow restriction).
//!
//! Note: `BlsPublicKey<Bls12381G2Impl>` carries a public key on the G1 curve
//! (the `Bls12381G2Impl` name refers to where signatures live, not keys).
//! Compressed G1 = 48 bytes = 96 hex chars.
//!
//! ## When to remove this
//!
//! TODO(blstrs_plus PR pending): once upstream `blstrs_plus` accepts owned
//! strings — either by switching its HR branch from `<&str>::deserialize` to
//! `<String>::deserialize`, or via a Visitor that supports `visit_str` /
//! `visit_string` — bump the dashcore dependency, then drop this wrapper and
//! the `serde(with = ...)` annotations on `Validator::public_key` and
//! `ValidatorSetV0::threshold_public_key`.

use crate::bls_signatures::inner_types::{G1Affine, GroupEncoding, PrimeCurveAffine};
use crate::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
    use serde::de::Error as _;

    if deserializer.is_human_readable() {
        // Read as owned String (works for every HR source, including Value
        // trees and ContentDeserializer-buffered enums). Then reconstruct
        // the public key from compressed-G1 bytes via the curve API,
        // bypassing the upstream `<&str>::deserialize` HR path entirely.
        let s: String = String::deserialize(deserializer)?;
        if s.len() != COMPRESSED_G1_LEN * 2 {
            return Err(D::Error::custom(format!(
                "expected {} hex chars for compressed G1 public key, got {}",
                COMPRESSED_G1_LEN * 2,
                s.len()
            )));
        }
        let mut compressed = <G1Affine as GroupEncoding>::Repr::default();
        let buf = compressed.as_mut();
        for (i, slot) in buf.iter_mut().enumerate() {
            let hi = hex_nibble(s.as_bytes()[i * 2]).map_err(D::Error::custom)?;
            let lo = hex_nibble(s.as_bytes()[i * 2 + 1]).map_err(D::Error::custom)?;
            *slot = (hi << 4) | lo;
        }
        let affine = Option::<G1Affine>::from(G1Affine::from_bytes(&compressed))
            .ok_or_else(|| D::Error::custom("not a valid compressed G1 point"))?;
        Ok(BlsPublicKey::<Bls12381G2Impl>(affine.to_curve()))
    } else {
        BlsPublicKey::<Bls12381G2Impl>::deserialize(deserializer)
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
