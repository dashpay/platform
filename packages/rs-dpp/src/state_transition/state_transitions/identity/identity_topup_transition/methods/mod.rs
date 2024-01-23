mod v0;

pub use v0::*;

use crate::identity::Identity;
use crate::prelude::AssetLockProof;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::{BlsModule, ProtocolError};

use crate::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
use platform_version::version::PlatformVersion;

impl IdentityTopUpTransitionMethodsV0 for IdentityTopUpTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        bls: &impl BlsModule,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match version.unwrap_or(
            platform_version
                .dpp
                .state_transition_serialization_versions
                .identity_top_up_state_transition
                .default_current_version,
        ) {
            0 => Ok(IdentityTopUpTransitionV0::try_from_identity(
                identity,
                asset_lock_proof,
                asset_lock_proof_private_key,
                bls,
                platform_version,
                version,
            )?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityTopUpTransition version for try_from_identity {v}"
            ))),
        }
    }

    fn get_minimal_asset_lock_value(
        platform_version: &PlatformVersion,
    ) -> Result<u64, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .process_state_transition
        {
            0 => IdentityTopUpTransitionV0::get_minimal_asset_lock_value(platform_version),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreateTransition version for minimal_asset_lock_value {v}"
            ))),
        }
    }
}
