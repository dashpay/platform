use crate::{prelude::Identifier, state_transition::StateTransitionType};
#[cfg(feature = "state-transition-signing")]
use crate::{BlsModule, ProtocolError};

#[cfg(feature = "state-transition-signing")]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::state_transition::AssetLockProved;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyType::ECDSA_HASH160;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use crate::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;

use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::PlatformVersion;

impl IdentityCreateTransitionMethodsV0 for IdentityCreateTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer>(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        signer: &S,
        bls: &impl BlsModule,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let mut identity_create_transition = IdentityCreateTransitionV0 {
            user_fee_increase,
            ..Default::default()
        };
        let public_keys = identity
            .public_keys()
            .iter()
            .map(|(_, public_key)| public_key.clone().into())
            .collect();
        identity_create_transition.set_public_keys(public_keys);

        identity_create_transition
            .set_asset_lock_proof(asset_lock_proof)
            .map_err(ProtocolError::from)?;

        //todo: remove clone
        let state_transition: StateTransition = identity_create_transition.clone().into();

        let key_signable_bytes = state_transition.signable_bytes()?;

        identity_create_transition
            .public_keys
            .iter_mut()
            .zip(identity.public_keys().iter())
            .try_for_each(|(public_key_with_witness, (_, public_key))| {
                if public_key.key_type().is_unique_key_type() {
                    let signature = signer.sign(public_key, &key_signable_bytes)?;
                    public_key_with_witness.set_signature(signature);
                }
                Ok::<(), ProtocolError>(())
            })?;

        let mut state_transition: StateTransition = identity_create_transition.into();

        state_transition.sign_by_private_key(asset_lock_proof_private_key, ECDSA_HASH160, bls)?;

        Ok(state_transition)
    }

    /// Get State Transition type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreate
    }
}

impl IdentityCreateTransitionAccessorsV0 for IdentityCreateTransitionV0 {
    /// Get identity public keys
    fn public_keys(&self) -> &[IdentityPublicKeyInCreation] {
        &self.public_keys
    }

    /// Get identity public keys
    fn public_keys_mut(&mut self) -> &mut Vec<IdentityPublicKeyInCreation> {
        &mut self.public_keys
    }

    /// Replaces existing set of public keys with a new one
    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>) {
        self.public_keys = public_keys;
    }

    /// Adds public keys to the existing public keys array
    fn add_public_keys(&mut self, public_keys: &mut Vec<IdentityPublicKeyInCreation>) {
        self.public_keys.append(public_keys);
    }

    /// Returns identity id
    fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    /// Returns Owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
