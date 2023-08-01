mod v0;

pub use v0::*;

use crate::identity::signer::Signer;
use crate::identity::{Identity, KeyID, PartialIdentity};
use crate::prelude::AssetLockProof;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::{StateTransition, StateTransitionType};
use crate::version::{FeatureVersion, PlatformVersion};
use crate::{BlsModule, NonConsensusError, ProtocolError};
use platform_value::{Bytes32, Identifier};

impl IdentityCreateTransitionMethodsV0 for IdentityCreateTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer>(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        signer: &S,
        bls: &impl BlsModule,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
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
            )?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreateTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }

    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreate
    }
}
