use super::types::{AddressSigner, TransferInput, TransferOutput};
use crate::platform::transition::address_inputs::fetch_inputs_with_nonce;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::prelude::AddressNonce;
use dpp::state_transition::address_credit_withdrawal_transition::methods::AddressCreditWithdrawalTransitionMethodsV0;
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::StateTransition;
use dpp::withdrawal::Pooling;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct AddressInputs {
    inputs_with_nonce: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    pending_inputs: BTreeMap<PlatformAddress, Credits>,
}

impl AddressInputs {
    async fn resolve(
        &self,
        sdk: &Sdk,
    ) -> Result<BTreeMap<PlatformAddress, (AddressNonce, Credits)>, Error> {
        resolve_inputs(sdk, self.inputs_with_nonce.clone(), &self.pending_inputs).await
    }
}

/// Fully resolved address transfer plan.
#[derive(Debug, Clone)]
pub(crate) struct AddressTransferPlan {
    /// Signer capable of authorizing address transfers.
    signer: AddressSigner,
    /// Fee strategy controlling extra funding requirements.
    fee_strategy: AddressFundsFeeStrategy,
    /// Classified funding inputs.
    inputs: AddressInputs,
    /// Outputs keyed by Platform address.
    outputs: BTreeMap<PlatformAddress, Credits>,
}

impl AddressTransferPlan {
    /// Construct a plan from signer, fee strategy, and classified inputs/outputs.
    fn new(
        signer: AddressSigner,
        fee_strategy: AddressFundsFeeStrategy,
        inputs: AddressInputs,
        outputs: BTreeMap<PlatformAddress, Credits>,
    ) -> Self {
        Self {
            signer,
            fee_strategy,
            inputs,
            outputs,
        }
    }

    /// Build the address-based state transition for this plan.
    pub(crate) async fn build_state_transition(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let inputs = self.inputs.resolve(sdk).await?;
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

/// Withdraw request captured by the builder prior to classification.
#[derive(Debug, Clone)]
pub(crate) struct AddressWithdrawalRequest {
    pub(crate) output_script: CoreScript,
    pub(crate) change_output: Option<(PlatformAddress, Credits)>,
    pub(crate) fee_strategy: AddressFundsFeeStrategy,
    pub(crate) core_fee_per_byte: u32,
    pub(crate) pooling: Pooling,
}

impl AddressWithdrawalRequest {
    pub(crate) fn new(output_script: CoreScript) -> Self {
        Self {
            output_script,
            change_output: None,
            fee_strategy: Vec::new(),
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
        }
    }
}

/// Fully resolved address withdrawal plan ready to produce a state transition.
#[derive(Debug, Clone)]
pub(crate) struct AddressWithdrawalPlan {
    signer: AddressSigner,
    fee_strategy: AddressFundsFeeStrategy,
    core_fee_per_byte: u32,
    pooling: Pooling,
    output_script: CoreScript,
    change_output: Option<(PlatformAddress, Credits)>,
    inputs: AddressInputs,
}

impl AddressWithdrawalPlan {
    fn new(
        signer: AddressSigner,
        request: AddressWithdrawalRequest,
        inputs: AddressInputs,
    ) -> Self {
        Self {
            signer,
            fee_strategy: request.fee_strategy,
            core_fee_per_byte: request.core_fee_per_byte,
            pooling: request.pooling,
            output_script: request.output_script,
            change_output: request.change_output,
            inputs,
        }
    }

    pub(crate) async fn build_state_transition(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let inputs = self.inputs.resolve(sdk).await?;
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        AddressCreditWithdrawalTransition::try_from_inputs_with_signer(
            inputs,
            self.change_output,
            self.fee_strategy.clone(),
            self.core_fee_per_byte,
            self.pooling,
            self.output_script.clone(),
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
    let inputs_classification = collect_address_inputs(inputs, "address transfer")?;
    let address_outputs = collect_address_outputs(outputs)?;
    Ok(AddressTransferPlan::new(
        signer,
        fee_strategy,
        inputs_classification,
        address_outputs,
    ))
}

/// Classify Platform address withdrawals by validating inputs and destination config.
pub(crate) fn classify_address_withdrawal(
    inputs: &[TransferInput],
    signer: AddressSigner,
    request: AddressWithdrawalRequest,
) -> Result<AddressWithdrawalPlan, Error> {
    let inputs_classification = collect_address_inputs(inputs, "address withdrawal")?;
    Ok(AddressWithdrawalPlan::new(
        signer,
        request,
        inputs_classification,
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

fn collect_address_inputs(inputs: &[TransferInput], context: &str) -> Result<AddressInputs, Error> {
    let mut pending_inputs = BTreeMap::new();
    let mut inputs_with_nonce = BTreeMap::new();
    let mut has_address_input = false;

    for funding in inputs {
        match funding {
            TransferInput::Addresses { inputs, .. } => {
                has_address_input = true;
                merge_without_nonce(&mut pending_inputs, &inputs_with_nonce, inputs)?;
            }
            TransferInput::AddressesWithNonce { inputs, .. } => {
                has_address_input = true;
                merge_with_nonce(&mut inputs_with_nonce, &pending_inputs, inputs)?;
            }
            _ => {
                return Err(Error::InvalidCreditTransfer(format!(
                    "{context} requires Platform address funding inputs",
                )))
            }
        }
    }

    if !has_address_input {
        return Err(Error::InvalidCreditTransfer(format!(
            "{context} requires at least one Platform address input",
        )));
    }

    Ok(AddressInputs {
        inputs_with_nonce,
        pending_inputs,
    })
}

async fn resolve_inputs(
    sdk: &Sdk,
    mut resolved: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    pending: &BTreeMap<PlatformAddress, Credits>,
) -> Result<BTreeMap<PlatformAddress, (AddressNonce, Credits)>, Error> {
    if !pending.is_empty() {
        let fetched = fetch_inputs_with_nonce(sdk, pending).await?;
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

#[cfg(test)]
impl AddressTransferPlan {
    /// Return pending inputs for assertions.
    pub(crate) fn pending_inputs_for_tests(&self) -> &BTreeMap<PlatformAddress, Credits> {
        &self.inputs.pending_inputs
    }

    /// Return inputs with nonce for assertions.
    pub(crate) fn inputs_with_nonce_for_tests(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        &self.inputs.inputs_with_nonce
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
        let context =
            classify_address_transfer(&inputs, &outputs, signer, AddressFundsFeeStrategy::new())
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
        let err =
            classify_address_transfer(&inputs, &outputs, signer, AddressFundsFeeStrategy::new())
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
