#[cfg(feature = "state-transition-signing")]
use crate::{
    prelude::{KeyOfTypeNonce, UserFeeIncrease},
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, AddressWitness};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::core_script::CoreScript;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyOfType;
use crate::serialization::Signable;
use crate::state_transition::address_credit_withdrawal_transition::methods::AddressCreditWithdrawalTransitionMethodsV0;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::withdrawal::Pooling;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl AddressCreditWithdrawalTransitionMethodsV0 for AddressCreditWithdrawalTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<KeyOfType>>(
        inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
        fee_strategy: AddressFundsFeeStrategy,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        tracing::debug!("try_from_inputs_with_signer: Started");
        tracing::debug!(
            input_count = inputs.len(),
            core_fee_per_byte = core_fee_per_byte,
            "try_from_inputs_with_signer"
        );

        // Create the unsigned transition
        let mut address_credit_withdrawal_transition = AddressCreditWithdrawalTransitionV0 {
            inputs: inputs.clone(),
            fee_strategy,
            core_fee_per_byte,
            pooling,
            output_script,
            user_fee_increase,
            input_witnesses: Vec::new(),
        };

        let state_transition: StateTransition = address_credit_withdrawal_transition.clone().into();

        let signable_bytes = state_transition.signable_bytes()?;

        address_credit_withdrawal_transition.input_witnesses = inputs
            .iter()
            .map(|(key_of_type, _)| signer.sign_create_witness(key_of_type, &signable_bytes))
            .collect::<Result<Vec<AddressWitness>, ProtocolError>>()?;

        tracing::debug!("try_from_inputs_with_signer: Successfully created transition");
        Ok(address_credit_withdrawal_transition.into())
    }
}
