//! Platform ECDSA verification, including consensus-stable parsing and errors.

use dashcore::hashes::{hash160, sha256d, Hash};
use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use secp256k1::{Message, PublicKey};
use thiserror::Error;

pub use dashcore::signer::{double_sha, sign, sign_hash};

/// Errors whose display text is included in Platform consensus errors.
#[derive(Debug, Error)]
pub enum EcdsaVerificationError {
    #[error("length not 65 bytes")]
    SignatureLength,
    #[error("invalid encoding")]
    SignatureEncoding(#[source] secp256k1::Error),
    #[error("key secp256k1 error")]
    PublicKeyEncoding(#[source] secp256k1::Error),
    #[error("length {0} invalid for this base58 type")]
    PublicKeyLength(usize),
    #[error("key prefix invalid: {0}")]
    PublicKeyPrefix(u8),
    #[error(transparent)]
    Crypto(#[from] secp256k1::Error),
    #[error("Invalid hash length")]
    HashLength,
    #[error("the signature isn't valid")]
    WrongKeyHash,
}

/// Decode a compact signature using the historical Platform acceptance rules.
pub fn parse_compact_signature(
    bytes: &[u8],
) -> Result<RecoverableSignature, EcdsaVerificationError> {
    if bytes.len() != 65 {
        return Err(EcdsaVerificationError::SignatureLength);
    }
    if !(27..=34).contains(&bytes[0]) {
        return Err(EcdsaVerificationError::SignatureEncoding(
            secp256k1::Error::InvalidRecoveryId,
        ));
    }
    let id = RecoveryId::try_from(i32::from((bytes[0] - 27) & 3))
        .map_err(EcdsaVerificationError::SignatureEncoding)?;
    RecoverableSignature::from_compact(&bytes[1..], id)
        .map_err(EcdsaVerificationError::SignatureEncoding)
}

/// Decode a SEC1 key, rejecting hybrid keys and preserving consensus error text.
pub fn parse_public_key(bytes: &[u8]) -> Result<PublicKey, EcdsaVerificationError> {
    match bytes.len() {
        33 => (),
        65 if bytes[0] == 4 => (),
        65 => return Err(EcdsaVerificationError::PublicKeyPrefix(bytes[0])),
        len => return Err(EcdsaVerificationError::PublicKeyLength(len)),
    }
    PublicKey::from_slice(bytes).map_err(EcdsaVerificationError::PublicKeyEncoding)
}

/// Verify a double-SHA256 message signature without changing historical errors.
pub fn verify_data_signature(
    data: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), EcdsaVerificationError> {
    let message = Message::from_digest(sha256d::Hash::hash(data).to_byte_array());
    let signature = parse_compact_signature(signature)?;
    let public_key = parse_public_key(public_key)?;
    signature.to_standard().verify(message, &public_key)?;
    Ok(())
}

/// Recover and compare the compressed key hash, irrespective of the header's compression bit.
pub fn verify_hash_signature(
    hash: &[u8],
    signature: &[u8],
    public_key_hash: &[u8],
) -> Result<(), EcdsaVerificationError> {
    let signature = parse_compact_signature(signature)?;
    let digest = hash
        .try_into()
        .map_err(|_| EcdsaVerificationError::HashLength)?;
    let public_key = signature.recover_ecdsa(Message::from_digest(digest))?;
    if hash160::Hash::hash(&public_key.serialize()).as_byte_array() != public_key_hash {
        return Err(EcdsaVerificationError::WrongKeyHash);
    }
    Ok(())
}
