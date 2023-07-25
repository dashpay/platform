use crate::identity::signer::Signer;
use crate::identity::{Identity, KeyID, PartialIdentity};
use crate::prelude::AssetLockProof;
use crate::state_transition::identity_create_transition::v0::v0_methods::IdentityCreateTransitionV0Methods;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::StateTransitionType;
use crate::version::{FeatureVersion, PlatformVersion};
use crate::{BlsModule, NonConsensusError, ProtocolError};
use platform_value::{Bytes32, Identifier};

impl IdentityCreateTransitionV0Methods for IdentityCreateTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer>(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        signer: &S,
        bls: &impl BlsModule,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .identity_to_identity_create_transition_with_signer
        {
            0 => Ok(IdentityCreateTransitionV0::try_from_identity_with_signer(
                identity,
                asset_lock_proof,
                asset_lock_proof_private_key,
                signer,
                bls,
                platform_version,
            )?
            .into()),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreateTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }

    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreate
    }

    fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), NonConsensusError> {
        match self {
            IdentityCreateTransition::V0(transition) => {
                transition.set_asset_lock_proof(asset_lock_proof)
            }
        }
    }

    fn get_asset_lock_proof(&self) -> &AssetLockProof {
        match self {
            IdentityCreateTransition::V0(transition) => transition.get_asset_lock_proof(),
        }
    }

    fn get_public_keys(&self) -> &[IdentityPublicKeyInCreation] {
        match self {
            IdentityCreateTransition::V0(transition) => transition.get_public_keys(),
        }
    }

    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>) {
        match self {
            IdentityCreateTransition::V0(transition) => transition.set_public_keys(public_keys),
        }
    }

    fn add_public_keys(&mut self, public_keys: &mut Vec<IdentityPublicKeyInCreation>) {
        match self {
            IdentityCreateTransition::V0(transition) => transition.add_public_keys(public_keys),
        }
    }

    fn get_identity_id(&self) -> &Identifier {
        match self {
            IdentityCreateTransition::V0(transition) => transition.get_identity_id(),
        }
    }

    fn get_owner_id(&self) -> &Identifier {
        match self {
            IdentityCreateTransition::V0(transition) => transition.get_owner_id(),
        }
    }
}
