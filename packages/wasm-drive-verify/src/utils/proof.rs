//! GroveDB proof-envelope policy for public WASM verification entry points.

use crate::utils::error::{format_error, ErrorCategory};
use dpp::version::PlatformVersion;
use wasm_bindgen::JsValue;

/// Reject GroveDB proof envelopes older than the floor the protocol version
/// sets in `SystemLimits::minimum_grovedb_proof_envelope_version`. Newer
/// discriminants pass here and fail in GroveDB's own decoder if this build
/// does not know them.
pub(crate) fn validate_supported_grovedb_proof(
    proof: &[u8],
    platform_version: &PlatformVersion,
) -> Result<(), String> {
    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<16>();
    let (version, _): (u32, usize) = bincode::decode_from_slice(proof, config)
        .map_err(|error| format!("invalid GroveDB proof envelope: {error}"))?;

    let minimum = platform_version
        .system_limits
        .minimum_grovedb_proof_envelope_version;
    if version < minimum {
        return Err(format!(
            "GroveDB proof envelope version {version} is below the minimum {minimum} required by protocol version {}",
            platform_version.protocol_version
        ));
    }

    Ok(())
}

pub(crate) fn supported_grovedb_proof<'a>(
    proof: &'a [u8],
    platform_version: &PlatformVersion,
) -> Result<&'a [u8], JsValue> {
    validate_supported_grovedb_proof(proof, platform_version)
        .map_err(|error| format_error(ErrorCategory::VerificationError, &error))?;
    Ok(proof)
}

#[cfg(test)]
mod tests {
    use super::validate_supported_grovedb_proof;
    use dpp::version::PlatformVersion;

    fn envelope(version: u32) -> Vec<u8> {
        bincode::encode_to_vec(version, bincode::config::standard().with_big_endian())
            .expect("encode proof version")
    }

    #[test]
    fn rejects_legacy_v0_envelope_at_the_latest_protocol_version() {
        let error = validate_supported_grovedb_proof(&envelope(0), PlatformVersion::latest())
            .expect_err("V0 must be rejected");

        assert!(
            error.contains("version 0 is below the minimum 1"),
            "unexpected message: {error}"
        );
    }

    #[test]
    fn accepts_legacy_v0_envelope_before_protocol_version_14() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");

        validate_supported_grovedb_proof(&envelope(0), platform_version)
            .expect("protocol version 13 accepts V0 envelopes");
    }

    #[test]
    fn accepts_envelopes_at_or_above_the_minimum() {
        for version in [1u32, 2u32] {
            validate_supported_grovedb_proof(&envelope(version), PlatformVersion::latest())
                .unwrap_or_else(|error| panic!("envelope {version} must pass: {error}"));
        }
    }
}
