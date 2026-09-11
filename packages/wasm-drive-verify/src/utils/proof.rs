//! GroveDB proof-envelope policy for public WASM verification entry points.

use crate::utils::error::{format_error, ErrorCategory};
use wasm_bindgen::JsValue;

const CURRENT_GROVEDB_PROOF_ENVELOPE_VERSION: u32 = 1;

pub(crate) fn validate_current_grovedb_proof(proof: &[u8]) -> Result<(), String> {
    let config = bincode::config::standard()
        .with_big_endian()
        .with_limit::<16>();
    let (version, _): (u32, usize) = bincode::decode_from_slice(proof, config)
        .map_err(|error| format!("invalid GroveDB proof envelope: {error}"))?;

    if version != CURRENT_GROVEDB_PROOF_ENVELOPE_VERSION {
        return Err(format!(
            "unsupported GroveDB proof envelope version {version}: current-state responses require V1"
        ));
    }

    Ok(())
}

pub(crate) fn current_grovedb_proof(proof: &[u8]) -> Result<&[u8], JsValue> {
    validate_current_grovedb_proof(proof)
        .map_err(|error| format_error(ErrorCategory::VerificationError, &error))?;
    Ok(proof)
}

#[cfg(test)]
mod tests {
    use super::validate_current_grovedb_proof;

    #[test]
    fn rejects_legacy_and_unknown_proof_envelopes() {
        let config = bincode::config::standard().with_big_endian();

        for version in [0u32, 2u32] {
            let proof = bincode::encode_to_vec(version, config).expect("encode proof version");
            assert!(validate_current_grovedb_proof(&proof).is_err());
        }
    }

    #[test]
    fn accepts_v1_proof_envelope() {
        let proof = bincode::encode_to_vec(1u32, bincode::config::standard().with_big_endian())
            .expect("encode proof version");

        assert!(validate_current_grovedb_proof(&proof).is_ok());
    }
}
