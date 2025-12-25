#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{AddressNonce, AssetLockProof};
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
use crate::state_transition::address_funding_from_asset_lock_transition::methods::AddressFundingFromAssetLockTransitionMethodsV0;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{prelude::UserFeeIncrease, state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use dashcore::signer;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl AddressFundingFromAssetLockTransitionMethodsV0 for AddressFundingFromAssetLockTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_asset_lock_with_signer<S: Signer<PlatformAddress>>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Option<Credits>>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        tracing::debug!("try_from_asset_lock_with_signer: Started");
        tracing::debug!(
            input_count = inputs.len(),
            output_count = outputs.len(),
            "try_from_asset_lock_with_signer"
        );

        // Create the unsigned transition
        let mut address_funding_transition = AddressFundingFromAssetLockTransitionV0 {
            asset_lock_proof,
            inputs: inputs.clone(),
            outputs,
            fee_strategy,
            user_fee_increase,
            signature: Default::default(),
            input_witnesses: Vec::new(),
        };

        let state_transition: StateTransition = address_funding_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        // Sign the asset lock proof
        let signature = signer::sign(&signable_bytes, asset_lock_proof_private_key)?;
        address_funding_transition.signature = signature.to_vec().into();

        // Sign with input witnesses
        address_funding_transition.input_witnesses = inputs
            .keys()
            .map(|address| signer.sign_create_witness(address, &signable_bytes))
            .collect::<Result<Vec<AddressWitness>, ProtocolError>>()?;

        tracing::debug!("try_from_asset_lock_with_signer: Successfully created transition");
        Ok(address_funding_transition.into())
    }
}
