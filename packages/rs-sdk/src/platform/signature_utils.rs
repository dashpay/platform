//! Utilities for working with ECDSA signatures and recovery IDs.
//!
//! This module provides helper functions for signature operations
//! commonly needed in blockchain applications, including safe
//! conversion methods for RecoveryId values.

use dpp::dashcore::secp256k1;
use dpp::dashcore::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use dpp::dashcore::secp256k1::{Message, Secp256k1, SecretKey};

/// Error type for signature operations
#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    /// Invalid recovery ID value (must be 0-3)
    #[error("Invalid recovery ID {0}, must be 0-3")]
    InvalidRecoveryId(u8),

    /// Secp256k1 library error
    #[error("Secp256k1 error: {0}")]
    Secp256k1(#[from] secp256k1::Error),
}

/// Extension trait for RecoveryId to provide conversion methods
pub trait RecoveryIdExt {
    /// Convert RecoveryId to u8 (0-3)
    fn to_u8(&self) -> u8;

    /// Create RecoveryId from u8 (must be 0-3)
    fn from_u8(v: u8) -> Result<RecoveryId, SignatureError>;
}

impl RecoveryIdExt for RecoveryId {
    fn to_u8(&self) -> u8 {
        // Use the standard From<RecoveryId> for i32 trait
        let i: i32 = (*self).into();
        debug_assert!(i >= 0 && i <= 3, "RecoveryId out of range");
        i as u8
    }

    fn from_u8(v: u8) -> Result<RecoveryId, SignatureError> {
        if v > 3 {
            return Err(SignatureError::InvalidRecoveryId(v));
        }
        // Use the standard TryFrom<i32> for RecoveryId trait
        RecoveryId::try_from(v as i32).map_err(|_| SignatureError::InvalidRecoveryId(v))
    }
}

/// Sign a message with a secret key and return signature components
///
/// Returns the signature split into r, s components and recovery ID as u8.
///
/// # Arguments
///
/// * `message` - The 32-byte message hash to sign
/// * `secret_key` - The secret key to sign with
///
/// # Returns
///
/// A tuple containing:
/// - `r`: The r component of the signature (32 bytes)
/// - `s`: The s component of the signature (32 bytes)  
/// - `v`: The recovery ID (0-3)
///
/// # Example
///
/// ```rust,no_run
/// use dash_sdk::platform::signature_utils::sign_message_recoverable;
/// use dpp::dashcore::secp256k1::SecretKey;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let secret_key = SecretKey::from_slice(&[1u8; 32])?;
/// let message_hash = [2u8; 32];
///
/// let (r, s, v) = sign_message_recoverable(&message_hash, &secret_key)?;
///
/// println!("Signature r: {:?}", r);
/// println!("Signature s: {:?}", s);
/// println!("Recovery ID: {}", v);
/// # Ok(())
/// # }
/// ```
pub fn sign_message_recoverable(
    message: &[u8; 32],
    secret_key: &SecretKey,
) -> Result<([u8; 32], [u8; 32], u8), SignatureError> {
    let secp = Secp256k1::new();
    let message = Message::from_digest_slice(message)?;

    // Sign the message
    let recoverable_sig = secp.sign_ecdsa_recoverable(&message, &secret_key);

    // Extract components
    let (recovery_id, signature) = recoverable_sig.serialize_compact();

    // Split signature into r and s components
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&signature[0..32]);
    s.copy_from_slice(&signature[32..64]);

    // Convert RecoveryId to u8 using our extension trait
    let v = recovery_id.to_u8();

    Ok((r, s, v))
}

/// Recover a public key from a signature and message
///
/// # Arguments
///
/// * `message` - The 32-byte message hash that was signed
/// * `r` - The r component of the signature (32 bytes)
/// * `s` - The s component of the signature (32 bytes)
/// * `v` - The recovery ID (0-3)
///
/// # Returns
///
/// The recovered public key as compressed bytes (33 bytes)
///
/// # Example
///
/// ```rust,no_run
/// use dash_sdk::platform::signature_utils::recover_public_key;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let message_hash = [0u8; 32];
/// let r = [1u8; 32];
/// let s = [2u8; 32];
/// let v = 0u8;
///
/// let public_key = recover_public_key(&message_hash, &r, &s, v)?;
/// println!("Recovered public key: {:?}", public_key);
/// # Ok(())
/// # }
/// ```
pub fn recover_public_key(
    message: &[u8; 32],
    r: &[u8; 32],
    s: &[u8; 32],
    v: u8,
) -> Result<Vec<u8>, SignatureError> {
    use crate::platform::signature_utils::RecoveryIdExt;

    let secp = Secp256k1::new();
    let message = Message::from_digest_slice(message)?;

    // Reconstruct signature
    let mut signature_bytes = [0u8; 64];
    signature_bytes[0..32].copy_from_slice(r);
    signature_bytes[32..64].copy_from_slice(s);

    // Create RecoveryId from u8
    let recovery_id = RecoveryId::from_u8(v)?;

    // Create recoverable signature
    let recoverable_sig = RecoverableSignature::from_compact(&signature_bytes, recovery_id)?;

    // Recover public key
    let public_key = secp.recover_ecdsa(&message, &recoverable_sig)?;

    // Return as compressed bytes (33 bytes)
    Ok(public_key.serialize().to_vec())
}

/// Helper to convert signature components to the format needed for contracts
///
/// This is useful when preparing signature data for proofs.
///
/// # Arguments
///
/// * `r` - The r component of the signature
/// * `s` - The s component of the signature
/// * `v` - The recovery ID
/// * `public_key` - The public key that created the signature
///
/// # Returns
///
/// A tuple suitable for contract membership proof:
/// - `signature_r`: Vec<u8> (32 bytes)
/// - `signature_s`: Vec<u8> (32 bytes)
/// - `signature_v`: u8 (recovery ID)
/// - `public_key`: Vec<u8> (33 bytes compressed)
pub fn prepare_signature_for_proof(
    r: &[u8; 32],
    s: &[u8; 32],
    v: u8,
    public_key: &[u8],
) -> (Vec<u8>, Vec<u8>, u8, Vec<u8>) {
    (r.to_vec(), s.to_vec(), v, public_key.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_id_conversion() {
        // Test valid recovery IDs
        for v in 0..=3u8 {
            let recovery_id = RecoveryId::from_u8(v).expect("Should create RecoveryId");
            assert_eq!(recovery_id.to_u8(), v);
        }

        // Test invalid recovery IDs
        assert!(RecoveryId::from_u8(4).is_err());
        assert!(RecoveryId::from_u8(255).is_err());
    }

    #[test]
    fn test_sign_and_recover() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ])
        .expect("Valid secret key");

        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let message = [0x42u8; 32];

        // Sign the message
        let (r, s, v) =
            sign_message_recoverable(&message, &secret_key).expect("Should sign message");

        // Recover public key
        let recovered = recover_public_key(&message, &r, &s, v).expect("Should recover public key");

        // Check it matches
        assert_eq!(recovered, public_key.serialize().to_vec());
    }
}
