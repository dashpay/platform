//! BLS (Boneh-Lynn-Shacham) signature operations for WASM
//!
//! This module provides BLS signature functionality for the WASM SDK,
//! including key generation, signing, and verification.

use wasm_bindgen::prelude::*;
use web_sys::js_sys::Uint8Array;
// use crate::error::to_js_error; // Currently unused

#[cfg(feature = "bls-signatures")]
use dpp::bls_signatures::{
    Bls12381G2Impl, Pairing, PublicKey, SecretKey, Signature, SignatureSchemes,
};

/// Generate a BLS private key
#[wasm_bindgen(js_name = generateBlsPrivateKey)]
pub fn generate_bls_private_key() -> Result<Uint8Array, JsError> {
    // Generate a random 32-byte private key
    let mut private_key = [0u8; 32];
    getrandom::getrandom(&mut private_key)
        .map_err(|e| JsError::new(&format!("Failed to generate random bytes: {}", e)))?;

    Ok(Uint8Array::from(&private_key[..]))
}

/// Derive a BLS public key from a private key
#[wasm_bindgen(js_name = blsPrivateKeyToPublicKey)]
pub fn bls_private_key_to_public_key(private_key: &[u8]) -> Result<Uint8Array, JsError> {
    #[cfg(feature = "bls-signatures")]
    {
        if private_key.len() != 32 {
            return Err(JsError::new("Private key must be 32 bytes"));
        }

        // Convert private key bytes to SecretKey
        let secret_key = SecretKey::<Bls12381G2Impl>::try_from(private_key)
            .map_err(|e| JsError::new(&format!("Invalid private key: {}", e)))?;

        // Get public key
        let public_key = secret_key.public_key();
        let public_key_bytes = public_key.0.to_compressed().to_vec();

        Ok(Uint8Array::from(&public_key_bytes[..]))
    }
    #[cfg(not(feature = "bls-signatures"))]
    {
        Err(JsError::new("BLS signatures feature not enabled"))
    }
}

/// Sign data with a BLS private key
#[wasm_bindgen(js_name = blsSign)]
pub fn bls_sign(data: &[u8], private_key: &[u8]) -> Result<Uint8Array, JsError> {
    #[cfg(feature = "bls-signatures")]
    {
        if private_key.len() != 32 {
            return Err(JsError::new("Private key must be 32 bytes"));
        }

        // Convert private key to SecretKey
        let secret_key = SecretKey::<Bls12381G2Impl>::try_from(private_key)
            .map_err(|e| JsError::new(&format!("Invalid private key: {}", e)))?;

        // Sign the data
        let sig = secret_key
            .sign(SignatureSchemes::Basic, data)
            .map_err(|e| JsError::new(&format!("Failed to sign: {}", e)))?;
        let signature_bytes = sig.as_raw_value().to_compressed().to_vec();

        Ok(Uint8Array::from(&signature_bytes[..]))
    }
    #[cfg(not(feature = "bls-signatures"))]
    {
        Err(JsError::new("BLS signatures feature not enabled"))
    }
}

/// Verify a BLS signature
#[wasm_bindgen(js_name = blsVerify)]
pub fn bls_verify(signature: &[u8], data: &[u8], public_key: &[u8]) -> Result<bool, JsError> {
    #[cfg(feature = "bls-signatures")]
    {
        if signature.len() != 96 {
            return Err(JsError::new("Signature must be 96 bytes"));
        }
        if public_key.len() != 48 {
            return Err(JsError::new("Public key must be 48 bytes"));
        }

        // Parse public key
        let pk = PublicKey::<Bls12381G2Impl>::try_from(public_key)
            .map_err(|e| JsError::new(&format!("Invalid public key: {}", e)))?;

        // Parse signature
        let signature_96_bytes: [u8; 96] = signature
            .try_into()
            .map_err(|_| JsError::new("Signature must be exactly 96 bytes"))?;

        let g2_element =
            <Bls12381G2Impl as Pairing>::Signature::from_compressed(&signature_96_bytes)
                .into_option()
                .ok_or_else(|| JsError::new("Invalid signature format"))?;
        let sig = Signature::<Bls12381G2Impl>::Basic(g2_element);

        // Verify the signature
        let result = sig.verify(&pk, data);

        Ok(result.is_ok())
    }
    #[cfg(not(feature = "bls-signatures"))]
    {
        Err(JsError::new("BLS signatures feature not enabled"))
    }
}

/// Validate a BLS public key
#[wasm_bindgen(js_name = validateBlsPublicKey)]
pub fn validate_bls_public_key(public_key: &[u8]) -> Result<bool, JsError> {
    #[cfg(feature = "bls-signatures")]
    {
        if public_key.len() != 48 {
            return Ok(false);
        }

        // Try to parse the public key
        let result = PublicKey::<Bls12381G2Impl>::try_from(public_key).is_ok();

        Ok(result)
    }
    #[cfg(not(feature = "bls-signatures"))]
    {
        Err(JsError::new("BLS signatures feature not enabled"))
    }
}

/// Aggregate multiple BLS signatures
#[wasm_bindgen(js_name = blsAggregateSignatures)]
pub fn bls_aggregate_signatures(signatures: JsValue) -> Result<Uint8Array, JsError> {
    #[cfg(feature = "bls-signatures")]
    {
        // Parse signatures from JavaScript array
        let signatures = if signatures.is_array() {
            let array = signatures
                .dyn_ref::<js_sys::Array>()
                .ok_or_else(|| JsError::new("Expected an array of signatures"))?;

            let mut sigs = Vec::new();
            for i in 0..array.length() {
                let sig_value = array.get(i);
                let sig_array = sig_value
                    .dyn_ref::<Uint8Array>()
                    .ok_or_else(|| JsError::new("Signature must be a Uint8Array"))?;
                sigs.push(sig_array.to_vec());
            }
            sigs
        } else {
            return Err(JsError::new("signatures must be an array"));
        };

        if signatures.is_empty() {
            return Err(JsError::new("At least one signature is required"));
        }

        // For now, we don't have direct access to signature aggregation in DPP
        // This would require exposing more BLS functionality
        Err(JsError::new(
            "BLS signature aggregation not yet implemented",
        ))
    }
    #[cfg(not(feature = "bls-signatures"))]
    {
        Err(JsError::new("BLS signatures feature not enabled"))
    }
}

/// Create a BLS threshold signature share
#[wasm_bindgen(js_name = blsCreateThresholdShare)]
pub fn bls_create_threshold_share(
    data: &[u8],
    private_key_share: &[u8],
    share_id: u32,
) -> Result<Uint8Array, JsError> {
    #[cfg(feature = "bls-signatures")]
    {
        // For threshold signatures, we would need additional BLS functionality
        // This is a placeholder for future implementation
        let _ = (data, private_key_share, share_id);
        Err(JsError::new("BLS threshold signatures not yet implemented"))
    }
    #[cfg(not(feature = "bls-signatures"))]
    {
        Err(JsError::new("BLS signatures feature not enabled"))
    }
}

/// Get the size of a BLS signature in bytes
#[wasm_bindgen(js_name = getBlsSignatureSize)]
pub fn get_bls_signature_size() -> u32 {
    96 // BLS12-381 signatures are 96 bytes
}

/// Get the size of a BLS public key in bytes
#[wasm_bindgen(js_name = getBlsPublicKeySize)]
pub fn get_bls_public_key_size() -> u32 {
    48 // BLS12-381 G1 public keys are 48 bytes
}

/// Get the size of a BLS private key in bytes
#[wasm_bindgen(js_name = getBlsPrivateKeySize)]
pub fn get_bls_private_key_size() -> u32 {
    32 // BLS12-381 private keys are 32 bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bls_sizes() {
        assert_eq!(get_bls_signature_size(), 96);
        assert_eq!(get_bls_public_key_size(), 48);
        assert_eq!(get_bls_private_key_size(), 32);
    }
}
