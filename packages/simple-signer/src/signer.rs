use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use dashcore_rpc::dashcore::signer;
use dpp::bincode::{Decode, Encode};
use dpp::ed25519_dalek::Signer as BlsSigner;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::platform_value::BinaryData;
use dpp::state_transition::errors::InvalidIdentityPublicKeyTypeError;
use dpp::{bls_signatures, ed25519_dalek, ProtocolError};
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};

/// This simple signer is only to be used in tests
#[derive(Default, Clone, PartialEq, Encode, Decode)]
pub struct SimpleSigner {
    /// Private keys is a map from the public key to the Private key bytes
    pub private_keys: BTreeMap<IdentityPublicKey, Vec<u8>>,
    /// Private keys to be added at the end of a block
    pub private_keys_in_creation: BTreeMap<IdentityPublicKey, Vec<u8>>,
}

impl Debug for SimpleSigner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleSigner")
            .field(
                "private_keys",
                &self
                    .private_keys
                    .iter()
                    .map(|(k, v)| (k, format!("sk: {}", BASE64_STANDARD.encode(v))))
                    .collect::<BTreeMap<_, _>>(),
            )
            .field(
                "private_keys_in_creation",
                &self
                    .private_keys_in_creation
                    .iter()
                    .map(|(k, v)| (k, format!("sk: {}", BASE64_STANDARD.encode(v))))
                    .collect::<BTreeMap<_, _>>(),
            )
            .finish()
    }
}

impl SimpleSigner {
    /// Add a key to the signer
    pub fn add_key(&mut self, public_key: IdentityPublicKey, private_key: Vec<u8>) {
        self.private_keys.insert(public_key, private_key);
    }

    /// Add keys to the signer
    pub fn add_keys<I: IntoIterator<Item = (IdentityPublicKey, Vec<u8>)>>(&mut self, keys: I) {
        self.private_keys.extend(keys)
    }

    /// Commit keys in creation
    pub fn commit_block_keys(&mut self) {
        self.private_keys.append(&mut self.private_keys_in_creation)
    }
}

impl Signer for SimpleSigner {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let private_key = self
            .private_keys
            .get(identity_public_key)
            .or_else(|| self.private_keys_in_creation.get(identity_public_key))
            .ok_or(ProtocolError::Generic(format!(
                "{:?} not found in {:?}",
                identity_public_key, self
            )))?;
        match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = signer::sign(data, private_key)?;
                Ok(signature.to_vec().into())
            }
            KeyType::BLS12_381 => {
                let pk =
                    bls_signatures::PrivateKey::from_bytes(private_key, false).map_err(|_e| {
                        ProtocolError::Generic(
                            "bls private key from bytes isn't correct".to_string(),
                        )
                    })?;
                Ok(pk.sign(data).to_bytes().to_vec().into())
            }
            KeyType::EDDSA_25519_HASH160 => {
                let key: [u8; 32] = private_key.clone().try_into().expect("expected 32 bytes");
                let pk = ed25519_dalek::SigningKey::try_from(&key).map_err(|_e| {
                    ProtocolError::Generic(
                        "eddsa 25519 private key from bytes isn't correct".to_string(),
                    )
                })?;
                Ok(pk.sign(data).to_vec().into())
            }
            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L187
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH => Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
                InvalidIdentityPublicKeyTypeError::new(identity_public_key.key_type()),
            )),
        }
    }
}
