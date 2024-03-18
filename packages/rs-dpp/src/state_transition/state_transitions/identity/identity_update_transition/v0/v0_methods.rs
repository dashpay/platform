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
    fn try_from_identity_with_signer<'a, S: Signer>(
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
            revision: identity.revision(),
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

    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
