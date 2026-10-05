//! GroveDB proof-envelope policy for public WASM verification entry points.

use crate::utils::error::{format_error, ErrorCategory};
use drive::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use wasm_bindgen::JsValue;

/// Refuse a GroveDB proof envelope older than Drive's
/// `MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION` before its bytes reach Drive.
pub(crate) fn supported_grovedb_proof(proof: &[u8]) -> Result<&[u8], JsValue> {
    require_supported_grovedb_proof_envelope(proof, "proof")
        .map_err(|error| format_error(ErrorCategory::VerificationError, &error.to_string()))?;
    Ok(proof)
}
