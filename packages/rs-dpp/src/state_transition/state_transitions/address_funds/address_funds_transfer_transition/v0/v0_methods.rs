#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::signer::IdentitySigner,
    prelude::{KeyOfTypeNonce, UserFeeIncrease},
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyOfType;
use crate::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use platform_value::BinaryData;
use crate::identity::signer::Signer;

impl AddressFundsTransferTransitionMethodsV0 for AddressFundsTransferTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_inputs_with_signer<S: Signer<KeyOfType>>(
        inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
        outputs: BTreeMap<KeyOfType, Credits>,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        tracing::debug!("try_from_inputs_with_signer: Started");
        tracing::debug!(input_count = inputs.len(), output_count = outputs.len(), "try_from_inputs_with_signer");

        // Create the unsigned transition
        let transition = AddressFundsTransferTransitionV0 {
            inputs: inputs.clone(),
            outputs,
            user_fee_increase,
            input_witnesses: Vec::new(),
        };

        // TODO: Sign each input with the corresponding private key
        // For now, create empty witnesses as placeholder
        let mut input_witnesses = Vec::new();
        for (key_of_type, _) in &inputs {
            // Get the private key for this input
            let _private_key = input_private_keys.get(key_of_type).ok_or_else(|| {
                ProtocolError::Generic(format!(
                    "Missing private key for input: {}",
                    key_of_type
                ))
            })?;

            // TODO: Create signature for this input using the private key
            // For now, use empty signature
            input_witnesses.push(BinaryData::default());
        }

        let signed_transition = AddressFundsTransferTransitionV0 {
            input_witnesses,
            ..transition
        };

        tracing::debug!("try_from_inputs_with_signer: Successfully created transition");
        Ok(signed_transition.into())
    }
}
