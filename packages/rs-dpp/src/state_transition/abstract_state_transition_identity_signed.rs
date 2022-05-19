use dashcore::{
    hashes::ripemd160,
    hashes::sha256,
    hashes::Hash,
    secp256k1::{PublicKey as RawPublicKey, SecretKey as RawSecretKey},
};

use anyhow::anyhow;
use bls_signatures::Serialize;
use std::convert::TryInto;

use crate::{
    identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel},
    prelude::*,
};

use super::StateTransitionLike;

pub trait StateTransitionIdentitySigned
where
    Self: StateTransitionLike,
{
    fn get_signature_public_key_id(&self) -> KeyID;
    fn set_signature_public_key_id(&self, key_id: KeyID);

    fn sign(
        &mut self,
        identity_public_key: &IdentityPublicKey,
        private_key: &[u8],
    ) -> Result<(), ProtocolError> {
        Self::verify_public_key_level_and_purpose(identity_public_key)?;

        match identity_public_key.get_type() {
            KeyType::ECDSA_SECP256K1 => {
                let public_key_compressed = get_compressed_public_ec_key(private_key)?;

                // we store compressed public key in the identity as base64 string (really?),
                // and here we compare the private key used to sing the state transition with
                // the compressed key stored in the identity

                if public_key_compressed.to_vec() != identity_public_key.get_data() {
                    return Err(ProtocolError::InvalidSignaturePublicKeyError {
                        public_key: identity_public_key.get_data().to_owned(),
                    });
                }

                self.sign_by_private_key(private_key, identity_public_key.get_type())
            }
            KeyType::ECDSA_HASH160 => {
                let public_key_compressed = get_compressed_public_ec_key(private_key)?;
                let pub_key_hash =
                    ripemd160::Hash::hash(&sha256::Hash::hash(&public_key_compressed)).to_vec();

                if pub_key_hash != identity_public_key.get_data() {
                    return Err(ProtocolError::InvalidSignaturePublicKeyError {
                        public_key: identity_public_key.get_data().to_owned(),
                    });
                }
                self.sign_by_private_key(private_key, identity_public_key.get_type())
            }
            KeyType::BLS12_381 => {
                let public_key = get_public_bls_key(private_key)?;

                if public_key != identity_public_key.get_data() {
                    return Err(ProtocolError::InvalidSignaturePublicKeyError {
                        public_key: identity_public_key.get_data().to_owned(),
                    });
                }
                self.sign_by_private_key(private_key, identity_public_key.get_type())
            }
        }
    }

    fn verify_signature(&self, public_key: &IdentityPublicKey) -> Result<(), ProtocolError> {
        Self::verify_public_key_level_and_purpose(public_key)?;

        let signature = self.get_signature();
        if signature.is_empty() {
            return Err(ProtocolError::StateTransitionIsNotIsSignedError {
                state_transition: self.clone().into(),
            });
        }

        if self.get_signature_public_key_id() != public_key.get_id() {
            return Err(ProtocolError::PublicKeyMismatchError {
                public_key: public_key.clone(),
            });
        }

        let public_key_buffer = public_key.get_data();
        match public_key.get_type() {
            KeyType::ECDSA_HASH160 => {
                self.verify_ecdsa_hash_160_signature_by_public_key_hash(public_key_buffer)
            }

            KeyType::ECDSA_SECP256K1 => {
                self.verify_ecdsa_signature_by_public_key(public_key_buffer)
            }

            KeyType::BLS12_381 => self.verify_bls_signature_by_public_key(public_key_buffer),
        }
    }

    /// Verifies that the supplied public key has the correct security level
    /// and purpose to sign the state transition
    fn verify_public_key_level_and_purpose(
        public_key: &IdentityPublicKey,
    ) -> Result<(), ProtocolError> {
        if Self::get_security_level_requirement() < public_key.get_security_level() {
            return Err(ProtocolError::PublicKeySecurityLevelNotMetError {
                public_key_security_level: public_key.get_security_level(),
                required_security_level: Self::get_security_level_requirement(),
            });
        }

        if public_key.get_purpose() != Purpose::AUTHENTICATION {
            return Err(ProtocolError::WrongPublicKeyPurposeError {
                public_key_purpose: public_key.get_purpose(),
                key_purpose_requirement: Purpose::AUTHENTICATION,
            });
        }
        Ok(())
    }

    fn get_security_level_requirement() -> SecurityLevel {
        SecurityLevel::MASTER
    }
}

pub fn get_compressed_public_ec_key(private_key: &[u8]) -> Result<[u8; 33], ProtocolError> {
    let sk = RawSecretKey::from_slice(private_key)
        .map_err(|e| anyhow!("Invalid ECDSA private key: {}", e))?;

    let secp = dashcore::secp256k1::Secp256k1::new();
    let public_key_compressed = RawPublicKey::from_secret_key(&secp, &sk).serialize();
    Ok(public_key_compressed)
}

pub fn get_public_bls_key(private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    let fixed_len_key: [u8; 32] = private_key
        .try_into()
        .map_err(|_| anyhow!("the BLS private key must be 32 bytes long"))?;
    let pk = bls_signatures::PrivateKey::from_bytes(&fixed_len_key).map_err(anyhow::Error::msg)?;
    let public_key = pk.public_key().as_bytes();
    Ok(public_key)
}
