//! Backward-compatibility shim for proof bytes that carry one or more
//! harmless trailing bytes past the canonical bincode envelope.
//!
//! # Background
//!
//! Before grovedb#661 the proof verifier called
//! `bincode::decode_from_slice(proof, config)` and silently discarded any
//! leftover bytes. The new release added
//! `decode_grovedb_proof_canonical`, which rejects proofs with trailing
//! bytes by returning
//! `Error::CorruptedData("proof has N trailing bytes after the encoded
//! envelope")`. The grovedb docstring itself notes that the trailing
//! bytes are *"harmless for the chain-bound correctness guarantee"* — the
//! decoded `GroveDBProof` and the resulting root hash are unchanged.
//!
//! Recorded proof fixtures captured against drive-abci builds that
//! pre-date the canonical-encoder fix carry exactly such a trailing byte
//! (typically one byte). Any older drive-abci node still in the wild that
//! has not picked up the canonical encoder would emit the same shape.
//! Production drive-abci on this grovedb revision emits canonical proofs
//! (verified by the `should_prove_and_verify_pre_programmed_distributions`
//! round-trip test), so the lenient path is only exercised by stale
//! proof bytes — never by anything produced on this branch.
//!
//! # What this helper does
//!
//! [`canonicalize_grovedb_proof`] runs the same bincode decode the new
//! verifier does, then returns the prefix of `proof` that bincode actually
//! consumed. The returned slice is bit-for-bit equivalent to what the
//! canonical encoder would have produced, so callers can hand it
//! straight to `GroveDb::verify_*` without further processing — the
//! strict canonical check inside grovedb then passes by construction.
//!
//! Callers must wrap every proof byte slice fed into a `GroveDb::verify_*`
//! entrypoint with this helper, e.g.
//! `GroveDb::verify_query(&canonicalize_grovedb_proof(proof)?, ...)`.

use std::borrow::Cow;

use grovedb::operations::proof::GroveDBProof;

use crate::error::proof::ProofError;
use crate::error::Error;

/// Trim trailing bytes past the canonical bincode envelope of a grovedb
/// proof, returning the prefix that the new strict
/// `decode_grovedb_proof_canonical` will accept verbatim.
///
/// Borrows when the input is already canonical (no allocation).
/// Returns a borrowed shorter slice when trailing bytes are present.
/// Returns a `ProofError::CorruptedProof` if the bincode envelope itself
/// is malformed — the caller sees the same shape of error grovedb would
/// have raised, just framed in platform's `Error` type.
pub fn canonicalize_grovedb_proof(proof: &[u8]) -> Result<Cow<'_, [u8]>, Error> {
    // Same bincode config the new `decode_grovedb_proof_canonical` uses.
    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<{ 256 * 1024 * 1024 }>();

    match bincode::decode_from_slice::<GroveDBProof, _>(proof, config) {
        Ok((_, consumed)) if consumed == proof.len() => Ok(Cow::Borrowed(proof)),
        Ok((_, consumed)) => {
            // One-line breadcrumb so we can detect any source still
            // emitting non-canonical proofs.
            tracing::trace!(
                trailing = proof.len() - consumed,
                "trimmed legacy trailing bytes from grovedb proof"
            );
            Ok(Cow::Borrowed(&proof[..consumed]))
        }
        Err(e) => Err(Error::Proof(ProofError::CorruptedProof(format!(
            "unable to bincode-decode grovedb proof envelope: {}",
            e
        )))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A canonical (no-trailing-bytes) proof captured from drive's own
    /// round-trip test passes through unchanged.
    #[test]
    fn canonical_proof_is_borrowed_unchanged() {
        // Smallest valid V1 envelope we can hand-roll for a sanity check:
        // we just need bincode to round-trip the same byte count, which
        // happens when we encode an actual `GroveDBProof` value.
        use grovedb::operations::proof::{GroveDBProofV1, LayerProof, ProofBytes};

        let proof = GroveDBProof::V1(GroveDBProofV1 {
            root_layer: LayerProof {
                merk_proof: ProofBytes::Merk(vec![]),
                lower_layers: Default::default(),
            },
        });
        let config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<{ 256 * 1024 * 1024 }>();
        let encoded = bincode::encode_to_vec(&proof, config).unwrap();

        let canonical = canonicalize_grovedb_proof(&encoded).unwrap();
        assert!(matches!(canonical, Cow::Borrowed(_)));
        assert_eq!(canonical.as_ref(), encoded.as_slice());
    }

    /// A canonical proof with one (or more) appended bytes is trimmed
    /// back to the canonical prefix.
    #[test]
    fn trailing_bytes_are_trimmed() {
        use grovedb::operations::proof::{GroveDBProofV1, LayerProof, ProofBytes};

        let proof = GroveDBProof::V1(GroveDBProofV1 {
            root_layer: LayerProof {
                merk_proof: ProofBytes::Merk(vec![1, 2, 3, 4, 5]),
                lower_layers: Default::default(),
            },
        });
        let config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<{ 256 * 1024 * 1024 }>();
        let canonical_bytes = bincode::encode_to_vec(&proof, config).unwrap();

        // Append a junk byte (mirrors what older drive-abci builds emitted).
        let mut with_trailing = canonical_bytes.clone();
        with_trailing.push(0xAB);

        let canonical = canonicalize_grovedb_proof(&with_trailing).unwrap();
        assert_eq!(canonical.as_ref(), canonical_bytes.as_slice());
    }

    /// Outright garbage that can't even bincode-decode surfaces as a
    /// `ProofError::CorruptedProof`.
    #[test]
    fn malformed_proof_errors() {
        let err = canonicalize_grovedb_proof(&[0xff; 4]).unwrap_err();
        assert!(
            matches!(err, Error::Proof(ProofError::CorruptedProof(_))),
            "expected CorruptedProof, got: {err:?}"
        );
    }
}
