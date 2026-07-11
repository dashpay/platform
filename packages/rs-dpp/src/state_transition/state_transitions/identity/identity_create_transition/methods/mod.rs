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
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use crate::{BlsModule, ProtocolError};
impl IdentityCreateTransitionMethodsV0 for IdentityCreateTransition {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer_and_private_key<S: Signer<IdentityPublicKey>>(
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
            .identity_to_identity_create_transition_with_signer
        {
            0 => Ok(
                IdentityCreateTransitionV0::try_from_identity_with_signer_and_private_key(
                    identity,
                    asset_lock_proof,
                    asset_lock_proof_private_key,
                    signer,
                    bls,
                    user_fee_increase,
                    platform_version,
                )
                .await?,
            ),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreateTransition version for try_from_identity_with_signer_and_private_key {v}"
            ))),
        }
    }

    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    #[allow(clippy::too_many_arguments)]
    async fn try_from_identity_with_signers<IS, AS>(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        identity_signer: &IS,
        asset_lock_signer: &AS,
        bls: &impl BlsModule,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        IS: Signer<IdentityPublicKey>,
        AS: ::key_wallet::signer::Signer,
    {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .identity_to_identity_create_transition_with_signer
        {
            0 => Ok(IdentityCreateTransitionV0::try_from_identity_with_signers(
                identity,
                asset_lock_proof,
                asset_lock_proof_path,
                identity_signer,
                asset_lock_signer,
                bls,
                user_fee_increase,
                platform_version,
            )
            .await?),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityCreateTransition version for try_from_identity_with_signers {v}"
            ))),
        }
    }

    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreate
    }
}
