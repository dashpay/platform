#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::shield_transition::methods::ShieldTransitionMethodsV0;
use crate::state_transition::shield_transition::v0::ShieldTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::{AddressNonce, UserFeeIncrease},
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldTransitionMethodsV0 for ShieldTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_bundle_with_signer<S: Signer<PlatformAddress>>(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        // Create the unsigned transition (empty witnesses)
        let mut shield_transition = ShieldTransitionV0 {
            inputs: inputs.clone(),
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            fee_strategy,
            user_fee_increase,
            input_witnesses: Vec::new(),
        };

        // Compute signable bytes (excludes input_witnesses which are marked exclude_from_sig_hash)
        let state_transition: StateTransition = shield_transition.clone().into();
        let signable_bytes = state_transition.signable_bytes()?;

        // Sign each input address
        shield_transition.input_witnesses = inputs
            .keys()
            .map(|address| signer.sign_create_witness(address, &signable_bytes))
            .collect::<Result<Vec<AddressWitness>, ProtocolError>>()?;

        Ok(shield_transition.into())
    }
}
