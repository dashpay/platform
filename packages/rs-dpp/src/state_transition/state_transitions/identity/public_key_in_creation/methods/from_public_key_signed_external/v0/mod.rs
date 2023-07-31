use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::{IdentityPublicKey, KeyType};
use crate::identity::signer::Signer;
use crate::ProtocolError;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

impl IdentityPublicKeyInCreation {
    pub(super) fn from_public_key_signed_external_v0<S: Signer>(
        public_key: IdentityPublicKey,
        state_transition_bytes: &[u8],
        signer: &S,
    ) -> Result<Self, ProtocolError>  {
        let mut public_key_with_witness: Self = public_key.clone().into();
        match public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::BLS12_381 => {
                public_key_with_witness.set_signature(
                    signer.sign(&public_key, state_transition_bytes)?);
            }
            KeyType::ECDSA_HASH160 | KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                // don't sign (on purpose)
            }
        }
        // dbg!(format!("signed signature {} data {} public key {}",hex::encode(public_key_with_witness.signature.as_slice()),  hex::encode(data), hex::encode(public_key.data.as_slice())));
        Ok(public_key_with_witness)
    }
}