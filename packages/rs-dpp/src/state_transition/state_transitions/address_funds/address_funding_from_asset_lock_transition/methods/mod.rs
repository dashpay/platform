mod v0;

#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{AddressNonce, AssetLockProof};
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::UserFeeIncrease,
    state_transition::{
        address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0,
        StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl AddressFundingFromAssetLockTransitionMethodsV0 for AddressFundingFromAssetLockTransition {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_asset_lock_with_signer_and_private_key<S: Signer<PlatformAddress>>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Option<Credits>>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .address_funding_from_asset_lock_transition
        {
            0 => Ok(
                AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signer_and_private_key::<S>(
                    asset_lock_proof,
                    asset_lock_proof_private_key,
                    inputs,
                    outputs,
                    fee_strategy,
                    signer,
                    user_fee_increase,
                    platform_version,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method:
                    "AddressFundingFromAssetLockTransition::try_from_asset_lock_with_signer_and_private_key"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    async fn try_from_asset_lock_with_signers<S, AS>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Option<Credits>>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        asset_lock_signer: &AS,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        S: Signer<PlatformAddress>,
        AS: ::key_wallet::signer::Signer,
    {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .address_funding_from_asset_lock_transition
        {
            0 => Ok(
                AddressFundingFromAssetLockTransitionV0::try_from_asset_lock_with_signers::<S, AS>(
                    asset_lock_proof,
                    asset_lock_proof_path,
                    inputs,
                    outputs,
                    fee_strategy,
                    signer,
                    asset_lock_signer,
                    user_fee_increase,
                    platform_version,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "AddressFundingFromAssetLockTransition::try_from_asset_lock_with_signers"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
