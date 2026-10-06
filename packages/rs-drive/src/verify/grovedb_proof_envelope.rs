use crate::error::proof::ProofError;
use crate::error::Error;

/// Lowest GroveDB proof envelope version a client accepts.
///
/// V0's item binding lets a prover return different item bytes under the
/// same authenticated root, so a quorum signature on the root does not make a
/// V0 payload safe. GroveDB emits V1 from grove version 3, which Platform
/// selects from protocol version 12, so every live network serves V1.
///
/// This is client policy, not consensus: no node reads it, so it holds for
/// every protocol version a client verifies with and is not a version-table
/// entry.
pub const MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION: u32 = 1;

/// Reject a GroveDB proof envelope older than
/// [`MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION`] before its bytes reach Drive.
///
/// GroveDB's envelope enum is encoded by bincode as a `u32` discriminant
/// followed by the version-specific payload; newer discriminants pass here
/// and fail in GroveDB's own decoder if this build does not know them.
/// `label` names the proof being read in the error: "proof", "predecessor
/// proof" or "forward proof".
pub fn require_supported_grovedb_proof_envelope(
    proof: &[u8],
    label: &'static str,
) -> Result<(), Error> {
    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<16>();
    let (version, _): (u32, usize) =
        bincode::decode_from_slice(proof, config).map_err(|error| {
            Error::Proof(ProofError::InvalidGroveDBProofEnvelope {
                proof: label,
                reason: error.to_string(),
            })
        })?;

    if version < MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION {
        return Err(Error::Proof(
            ProofError::UnsupportedGroveDBProofEnvelopeVersion {
                proof: label,
                version,
                minimum: MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION,
            },
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{require_supported_grovedb_proof_envelope, MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION};
    use crate::error::proof::ProofError;
    use crate::error::Error;

    fn envelope(version: u32) -> Vec<u8> {
        bincode::encode_to_vec(version, bincode::config::standard().with_big_endian())
            .expect("encode envelope version")
    }

    #[test]
    fn should_reject_a_v0_envelope() {
        let error = require_supported_grovedb_proof_envelope(&envelope(0), "forward proof")
            .expect_err("V0 must be rejected");

        assert!(
            matches!(
                error,
                Error::Proof(ProofError::UnsupportedGroveDBProofEnvelopeVersion {
                    proof: "forward proof",
                    version: 0,
                    minimum: MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION,
                })
            ),
            "unexpected error: {error}"
        );
        assert_eq!(
            error.to_string(),
            "proof: unsupported GroveDB proof envelope version 0 in the forward proof: at least version 1 is required"
        );
    }

    #[test]
    fn should_reject_bytes_without_an_envelope_discriminant() {
        let error = require_supported_grovedb_proof_envelope(&[], "proof")
            .expect_err("empty bytes carry no envelope");

        assert!(
            matches!(
                error,
                Error::Proof(ProofError::InvalidGroveDBProofEnvelope { proof: "proof", .. })
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn should_pass_envelopes_at_or_above_the_minimum() {
        for version in [1u32, 2u32] {
            require_supported_grovedb_proof_envelope(&envelope(version), "proof")
                .unwrap_or_else(|error| panic!("envelope {version} must pass: {error}"));
        }
    }
}
