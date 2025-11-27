mod v0;

pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use crate::{BlsModule, ProtocolError};
impl AddressFundingFromAssetLockTransitionMethodsV0 for AddressFundingFromAssetLockTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        signer: &S,
        bls: &impl BlsModule,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .identity_to_address_funding_from_asset_lock_transition_with_signer
        {
            0 => Ok(AddressFundingFromAssetLockTransitionV0::try_from_identity_with_signer(
                identity,
                asset_lock_proof,
                asset_lock_proof_private_key,
                signer,
                bls,
                user_fee_increase,
                platform_version,
            )?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown AddressFundingFromAssetLockTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }

    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreate
    }
}
