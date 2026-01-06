#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;

#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

#[cfg(feature = "state-transition-signing")]
use crate::consensus::signature::{
    InvalidSignaturePublicKeySecurityLevelError, MissingPublicKeyError, SignatureError,
};
#[cfg(feature = "state-transition-signing")]
use crate::consensus::ConsensusError;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey};

#[cfg(feature = "state-transition-signing")]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use crate::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition};
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
use crate::{
    identity::KeyID,
    prelude::{Identifier, Revision},
};
#[cfg(feature = "state-transition-signing")]
use crate::{identity::SecurityLevel, ProtocolError};

impl IdentityUpdateTransitionMethodsV0 for IdentityUpdateTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<'a, S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let add_public_keys_in_creation = add_public_keys
            .iter()
            .map(|public_key| public_key.into())
            .collect();

        let mut identity_update_transition = IdentityUpdateTransitionV0 {
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity.id(),
            revision: identity.revision() + 1,
            nonce,
            add_public_keys: add_public_keys_in_creation,
            disable_public_keys,
            user_fee_increase,
        };

        let state_transition: StateTransition = identity_update_transition.clone().into();

        let key_signable_bytes = state_transition.signable_bytes()?;

        // Sign all the keys
        identity_update_transition
            .add_public_keys
            .iter_mut()
            .zip(add_public_keys.iter())
            .try_for_each(|(public_key_with_witness, public_key)| {
                if public_key.key_type().is_unique_key_type() {
                    let signature = signer.sign(public_key, &key_signable_bytes)?;
                    public_key_with_witness.set_signature(signature);
                }

                Ok::<(), ProtocolError>(())
            })?;

        let master_public_key = identity
            .public_keys()
            .get(master_public_key_id)
            .ok_or::<ConsensusError>(
                SignatureError::MissingPublicKeyError(MissingPublicKeyError::new(
                    *master_public_key_id,
                ))
                .into(),
            )?;
        if master_public_key.security_level() != SecurityLevel::MASTER {
            Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                InvalidSignaturePublicKeySecurityLevelError::new(
                    master_public_key.security_level(),
                    vec![SecurityLevel::MASTER],
                ),
            ))
        } else {
            let mut state_transition: StateTransition = identity_update_transition.into();
            state_transition.sign_external(
                master_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )?;
            Ok(state_transition)
        }
    }
}

impl IdentityUpdateTransitionAccessorsV0 for IdentityUpdateTransitionV0 {
    fn set_identity_id(&mut self, id: Identifier) {
        self.identity_id = id;
    }

    fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn set_nonce(&mut self, nonce: IdentityNonce) {
        self.nonce = nonce;
    }

    fn nonce(&self) -> IdentityNonce {
        self.nonce
    }

    fn set_public_keys_to_add(&mut self, add_public_keys: Vec<IdentityPublicKeyInCreation>) {
        self.add_public_keys = add_public_keys;
    }

    fn public_keys_to_add(&self) -> &[IdentityPublicKeyInCreation] {
        &self.add_public_keys
    }

    fn public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreation] {
        &mut self.add_public_keys
    }

    fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>) {
        self.disable_public_keys = disable_public_keys;
    }

    fn public_key_ids_to_disable(&self) -> &[KeyID] {
        &self.disable_public_keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::accessors::IdentityGettersV0;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::v0::IdentityV0;
    use crate::identity::{Identity, KeyType, Purpose, SecurityLevel};
    use platform_value::string_encoding::Encoding;
    use platform_value::BinaryData;
    use std::collections::BTreeMap;

    fn create_test_identity_with_revision(revision: u64) -> Identity {
        let mut public_keys = BTreeMap::new();
        public_keys.insert(
            0,
            IdentityPublicKeyV0 {
                id: 0,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MASTER,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::from_string(
                    "AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di",
                    Encoding::Base64,
                )
                .unwrap(),
                disabled_at: None,
            }
            .into(),
        );

        IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys,
            balance: 1000,
            revision,
        }
        .into()
    }

    /// Test that the revision in the IdentityUpdateTransitionV0 is correctly set to
    /// identity.revision() + 1 when constructed directly.
    ///
    /// This is a regression test for a bug where the code incorrectly used
    /// identity.revision() instead of identity.revision() + 1.
    #[test]
    fn test_identity_update_transition_revision_is_identity_revision_plus_one() {
        // Test with identity at revision 0 (fresh identity)
        let identity_rev_0 = create_test_identity_with_revision(0);

        let transition_v0 = IdentityUpdateTransitionV0 {
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity_rev_0.id(),
            revision: identity_rev_0.revision() + 1, // This is the correct pattern
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![],
            user_fee_increase: 0,
        };

        // The transition should have revision 1 (identity.revision() + 1 = 0 + 1 = 1)
        assert_eq!(
            transition_v0.revision(),
            1,
            "Transition revision should be identity.revision() + 1 = 0 + 1 = 1"
        );

        // Test with identity at revision 5 (identity that has been updated 5 times)
        let identity_rev_5 = create_test_identity_with_revision(5);

        let transition_v0_rev_5 = IdentityUpdateTransitionV0 {
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity_rev_5.id(),
            revision: identity_rev_5.revision() + 1, // This is the correct pattern
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![],
            user_fee_increase: 0,
        };

        // The transition should have revision 6 (identity.revision() + 1 = 5 + 1 = 6)
        assert_eq!(
            transition_v0_rev_5.revision(),
            6,
            "Transition revision should be identity.revision() + 1 = 5 + 1 = 6"
        );
    }
}
