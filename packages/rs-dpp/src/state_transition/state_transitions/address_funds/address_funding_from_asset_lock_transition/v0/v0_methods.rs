#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyOfType;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
use crate::state_transition::address_funding_from_asset_lock_transition::methods::AddressFundingFromAssetLockTransitionMethodsV0;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{prelude::UserFeeIncrease, state_transition::StateTransition, ProtocolError};
use dashcore::signer;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

impl AddressFundingFromAssetLockTransitionMethodsV0 for AddressFundingFromAssetLockTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_asset_lock(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        outputs: BTreeMap<KeyOfType, Credits>,
        output_paying_fees: u16,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        tracing::debug!("try_from_asset_lock_with_signer: Started");
        tracing::debug!(
            output_count = outputs.len(),
            output_paying_fees = output_paying_fees,
            "try_from_asset_lock_with_signer"
        );

        // Create the unsigned transition
        let address_funding_transition = AddressFundingFromAssetLockTransitionV0 {
            asset_lock_proof,
            outputs,
            output_paying_fees,
            user_fee_increase,
            ..Default::default()
        };

        let mut state_transition: StateTransition = address_funding_transition.clone().into();

        let data = state_transition.signable_bytes()?;

        let signature = signer::sign(&data, asset_lock_proof_private_key)?;
        if !state_transition.set_signature(signature.to_vec().into()) {
            return Err(ProtocolError::InvalidVerificationWrongNumberOfElements {
                needed: state_transition.required_number_of_private_keys(),
                using: 1,
                msg: "failed to set ECDSA signature",
            });
        };

        tracing::debug!("try_from_asset_lock_with_signer: Successfully created transition");
        Ok(state_transition)
    }
}
