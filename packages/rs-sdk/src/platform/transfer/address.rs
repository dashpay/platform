use super::types::{AddressSigner, TransferInput, TransferOutput};
use crate::platform::transition::address_inputs::fetch_inputs_with_nonce;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::StateTransition;
use std::collections::BTreeMap;

/// Fully resolved address transfer plan.
#[derive(Debug, Clone)]
pub(crate) struct AddressTransferPlan {
    /// Signer capable of authorizing address transfers.
    signer: AddressSigner,
    /// Fee strategy controlling extra funding requirements.
    fee_strategy: AddressFundsFeeStrategy,
    /// Inputs already accompanied by nonces.
    inputs_with_nonce: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// Inputs missing nonce information and requiring RPC lookup.
    pending_inputs: BTreeMap<PlatformAddress, Credits>,
    /// Outputs keyed by Platform address.
    outputs: BTreeMap<PlatformAddress, Credits>,
}

impl AddressTransferPlan {
    /// Construct a plan from signer, fee strategy, and classified inputs/outputs.
    fn new(
        signer: AddressSigner,
        fee_strategy: AddressFundsFeeStrategy,
        inputs_with_nonce: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        pending_inputs: BTreeMap<PlatformAddress, Credits>,
        outputs: BTreeMap<PlatformAddress, Credits>,
    ) -> Self {
        Self {
            signer,
            fee_strategy,
            inputs_with_nonce,
            pending_inputs,
            outputs,
        }
    }

    /// Resolve missing nonce info by querying Drive when needed.
    async fn resolve_inputs(
        &self,
        sdk: &Sdk,
    ) -> Result<BTreeMap<PlatformAddress, (AddressNonce, Credits)>, Error> {
        let mut resolved = self.inputs_with_nonce.clone();
        if !self.pending_inputs.is_empty() {
            let fetched = fetch_inputs_with_nonce(sdk, &self.pending_inputs).await?;
            for (address, entry) in fetched {
                if resolved.insert(address, entry).is_some() {
                    return Err(Error::InvalidCreditTransfer(format!(
                        "input for {} provided with and without nonce",
                        address
                    )));
                }
            }
        }

        Ok(resolved)
    }

    /// Build the address-based state transition for this plan.
    pub(crate) async fn build_state_transition(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let inputs = self.resolve_inputs(sdk).await?;
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        AddressFundsTransferTransition::try_from_inputs_with_signer(
            inputs,
            self.outputs.clone(),
            self.fee_strategy.clone(),
            &self.signer,
            user_fee_increase,
            sdk.version(),
        )
        .map_err(Error::from)
    }
}

/// Classify Platform address transfers by validating inputs and outputs.
pub(crate) fn classify_address_transfer(
    inputs: &[TransferInput],
    outputs: &BTreeMap<TransferOutput, Credits>,
    signer: AddressSigner,
    fee_strategy: AddressFundsFeeStrategy,
) -> Result<AddressTransferPlan, Error> {
    let mut pending_inputs = BTreeMap::new();
    let mut inputs_with_nonce = BTreeMap::new();
    let mut has_address_input = false;

    for funding in inputs {
        match funding {
            TransferInput::Addresses {
                inputs,
                input_private_keys: _input_private_keys,
            } => {
                has_address_input = true;
                merge_without_nonce(&mut pending_inputs, &inputs_with_nonce, inputs)?
            }
            TransferInput::AddressesWithNonce {
                inputs,
                input_private_keys: _input_private_keys,
            } => {
                has_address_input = true;
                merge_with_nonce(&mut inputs_with_nonce, &pending_inputs, inputs)?
            }
            _ => {
                return Err(Error::InvalidCreditTransfer(
                    "address transfer requires Platform address funding inputs".to_string(),
                ))
            }
        }
    }

    if !has_address_input {
        return Err(Error::InvalidCreditTransfer(
            "address transfer requires at least one Platform address input".to_string(),
        ));
    }

    let address_outputs = collect_address_outputs(outputs)?;
    Ok(AddressTransferPlan::new(
        signer,
        fee_strategy,
        inputs_with_nonce,
        pending_inputs,
        address_outputs,
    ))
}

/// Merge inputs lacking nonce data into the aggregate map.
fn merge_without_nonce(
    target: &mut BTreeMap<PlatformAddress, Credits>,
    inputs_with_nonce: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    source: &BTreeMap<PlatformAddress, Credits>,
) -> Result<(), Error> {
    for (address, amount) in source {
        if target.contains_key(address) || inputs_with_nonce.contains_key(address) {
            return Err(Error::InvalidCreditTransfer(format!(
                "input for {} provided multiple times",
                address
            )));
        }
        target.insert(*address, *amount);
    }
    Ok(())
}

/// Merge inputs that already include nonce data.
fn merge_with_nonce(
    target: &mut BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    pending_inputs: &BTreeMap<PlatformAddress, Credits>,
    source: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
) -> Result<(), Error> {
    for (address, value) in source {
        if target.contains_key(address) || pending_inputs.contains_key(address) {
            return Err(Error::InvalidCreditTransfer(format!(
                "input for {} provided multiple times",
                address
            )));
        }
        target.insert(*address, *value);
    }
    Ok(())
}

/// Extract Platform address outputs and validate presence.
fn collect_address_outputs(
    outputs: &BTreeMap<TransferOutput, Credits>,
) -> Result<BTreeMap<PlatformAddress, Credits>, Error> {
    let mut address_outputs = BTreeMap::new();
    for (output, amount) in outputs {
        match output {
            TransferOutput::PlatformAddress(address) => {
                address_outputs.insert(*address, *amount);
            }
            _ => {
                return Err(Error::InvalidCreditTransfer(
                    "address transfer outputs must be Platform addresses".to_string(),
                ))
            }
        }
    }

    if address_outputs.is_empty() {
        Err(Error::InvalidCreditTransfer(
            "address transfer requires at least one output".to_string(),
        ))
    } else {
        Ok(address_outputs)
    }
}

#[cfg(test)]
impl AddressTransferPlan {
    /// Return pending inputs for assertions.
    pub(crate) fn pending_inputs_for_tests(&self) -> &BTreeMap<PlatformAddress, Credits> {
        &self.pending_inputs
    }

    /// Return inputs with nonce for assertions.
    pub(crate) fn inputs_with_nonce_for_tests(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        &self.inputs_with_nonce
    }

    /// Return outputs for assertions.
    pub(crate) fn outputs_for_tests(&self) -> &BTreeMap<PlatformAddress, Credits> {
        &self.outputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::AddressWitness;
    use dpp::identity::signer::Signer;
    use dpp::platform_value::BinaryData;
    use dpp::ProtocolError;
    use std::sync::Arc;

    fn address(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    #[test]
    fn classify_address_transfer_collects_inputs_and_outputs() {
        let inputs = vec![
            TransferInput::from_addresses(BTreeMap::from([(address(1), 10)]), vec![]),
            TransferInput::from_addresses_with_nonce(
                BTreeMap::from([(address(2), (5, 15))]),
                vec![],
            ),
        ];

        let outputs =
            BTreeMap::from([(TransferOutput::PlatformAddress(address(3)), 25 as Credits)]);

        let signer: AddressSigner = Arc::new(TestAddressSigner).into();
        let context = classify_address_transfer(
            &inputs,
            &outputs,
            signer,
            AddressFundsFeeStrategy::new(),
        )
        .expect("valid context");

        assert_eq!(context.pending_inputs_for_tests().len(), 1);
        assert_eq!(context.inputs_with_nonce_for_tests().len(), 1);
        assert_eq!(context.outputs_for_tests().len(), 1);
    }

    #[test]
    fn classify_address_transfer_rejects_non_platform_outputs() {
        let inputs = vec![TransferInput::from_addresses(
            BTreeMap::from([(address(1), 10)]),
            vec![],
        )];
        let outputs =
            BTreeMap::from([(TransferOutput::Identity(Default::default()), 5 as Credits)]);

        let signer: AddressSigner = Arc::new(TestAddressSigner).into();
        let err = classify_address_transfer(
            &inputs,
            &outputs,
            signer,
            AddressFundsFeeStrategy::new(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[derive(Debug)]
    struct TestAddressSigner;

    impl Signer<PlatformAddress> for TestAddressSigner {
        fn sign(&self, _key: &PlatformAddress, _data: &[u8]) -> Result<BinaryData, ProtocolError> {
            Err(ProtocolError::Generic(
                "sign should not be called in tests".to_string(),
            ))
        }

        fn sign_create_witness(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "sign_create_witness should not be called in tests".to_string(),
            ))
        }

        fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
            true
        }
    }
}
