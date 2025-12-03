use super::address::{classify_address_transfer, AddressTransferPlan};
use super::identity::{classify_identity_transfer, IdentityTransferSelection};
use super::types::{
    AddressSigner, IdentitySigner, IdentityTransferConfig, TransferInput, TransferOutput,
};
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::address_funds::AddressFundsFeeStrategy;
use dpp::errors::consensus::basic::state_transition::{
    OutputBelowMinimumError, TransitionNoInputsError, TransitionNoOutputsError,
};
use dpp::fee::Credits;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use std::collections::BTreeMap;

/// Aggregated credit transfer description created via [`CreditTransferBuilder`].
///
/// Supports the following state transition types:
/// - [IdentityCreditTransferTransition](dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition)
/// - [AddressFundsTransferTransition](dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition)
#[derive(Debug)]
pub struct CreditTransfer {
    /// Fully classified transfer plan captured during build.
    transfer_kind: TransferKind,
}

impl CreditTransfer {
    /// Creates a new builder instance.
    pub fn builder() -> CreditTransferBuilder {
        CreditTransferBuilder::default()
    }

    /// Build the appropriate state transition for the captured inputs and outputs.
    async fn build_state_transition(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        match &self.transfer_kind {
            TransferKind::Identity(selection) => {
                let user_fee_increase = settings
                    .as_ref()
                    .and_then(|settings| settings.user_fee_increase)
                    .unwrap_or_default();

                selection
                    .config
                    .state_transition(
                        sdk,
                        selection.plan.recipient_id,
                        selection.plan.amount,
                        user_fee_increase,
                        settings,
                    )
                    .await
            }
            TransferKind::Address(plan) => plan.build_state_transition(sdk, settings).await,
        }
    }
}

/// Enum describing the resolved transfer flow.
#[derive(Debug)]
enum TransferKind {
    /// Transfer between identities.
    Identity(IdentityTransferSelection),
    /// Transfer between Platform addresses.
    Address(AddressTransferPlan),
}

/// Builder used to configure `CreditTransfer` inputs and outputs.
#[derive(Default)]
pub struct CreditTransferBuilder {
    /// Funding inputs staged for the transfer.
    inputs: Vec<TransferInput>,
    /// Outputs aggregated by destination.
    outputs: BTreeMap<TransferOutput, Credits>,
    /// Signer used for Platform address transfers.
    address_signer: Option<AddressSigner>,
    /// Fee configuration when spending Platform addresses.
    address_fee_strategy: AddressFundsFeeStrategy,
}

impl CreditTransferBuilder {
    /// Adds a funding source to the transfer.
    pub fn input<S>(&mut self, source: S) -> Result<&mut Self, Error>
    where
        S: TryInto<TransferInput> + Send,
        <S as TryInto<TransferInput>>::Error: ToString,
    {
        let funding = source
            .try_into()
            .map_err(|err| Error::InvalidCreditTransfer(err.to_string()))?;
        self.inputs.push(funding);
        Ok(self)
    }

    /// Adds an identity funding source with its signer context.
    pub fn identity_input<S>(
        &mut self,
        identity: Identity,
        signer: S,
        signing_key: Option<IdentityPublicKey>,
    ) -> Result<&mut Self, Error>
    where
        S: Into<IdentitySigner>,
    {
        let config = IdentityTransferConfig::new(identity, signer, signing_key);
        self.inputs.push(TransferInput::Identity(config));
        Ok(self)
    }

    /// Sets the signer used when spending Platform addresses.
    pub fn address_signer<S>(&mut self, signer: S) -> &mut Self
    where
        S: Into<AddressSigner>,
    {
        self.address_signer = Some(signer.into());
        self
    }

    /// Configures the fee strategy for address-to-address transfers.
    pub fn address_fee_strategy(&mut self, strategy: AddressFundsFeeStrategy) -> &mut Self {
        self.address_fee_strategy = strategy;
        self
    }

    /// Adds an output destination with the specified amount.
    pub fn output<D>(&mut self, destination: D, amount: Credits) -> Result<&mut Self, Error>
    where
        D: TryInto<TransferOutput>,
        <D as TryInto<TransferOutput>>::Error: ToString,
    {
        if amount == 0 {
            return Err(Error::from(OutputBelowMinimumError::new(amount, 1)));
        }

        let transfer_output = destination
            .try_into()
            .map_err(|err| Error::InvalidCreditTransfer(err.to_string()))?;

        let entry = self.outputs.entry(transfer_output).or_insert(0);
        *entry = entry.saturating_add(amount);

        Ok(self)
    }

    /// Finalizes the builder and returns an immutable `CreditTransfer`.
    pub fn build(self) -> Result<CreditTransfer, Error> {
        if self.inputs.is_empty() {
            return Err(Error::from(TransitionNoInputsError::new()));
        }

        if self.outputs.is_empty() {
            return Err(Error::from(TransitionNoOutputsError::new()));
        }

        let CreditTransferBuilder {
            inputs,
            outputs,
            address_signer,
            address_fee_strategy,
        } = self;

        let outputs_are_identities = outputs
            .keys()
            .all(|output| matches!(output, TransferOutput::Identity(_)));
        if outputs_are_identities {
            let transfer_kind =
                TransferKind::Identity(classify_identity_transfer(&inputs, &outputs)?);
            drop(inputs);
            drop(outputs);
            return Ok(CreditTransfer { transfer_kind });
        }

        let outputs_are_addresses = outputs
            .keys()
            .all(|output| matches!(output, TransferOutput::PlatformAddress(_)));
        if outputs_are_addresses {
            let signer = address_signer.ok_or_else(|| {
                Error::InvalidCreditTransfer(
                    "address transfers require an address signer configuration".to_string(),
                )
            })?;
            let transfer_kind = TransferKind::Address(classify_address_transfer(
                &inputs,
                &outputs,
                signer,
                address_fee_strategy,
            )?);
            drop(inputs);
            drop(outputs);
            return Ok(CreditTransfer { transfer_kind });
        }

        Err(Error::InvalidCreditTransfer(
            "unsupported credit transfer outputs".to_string(),
        ))
    }
}

#[async_trait::async_trait]
impl BroadcastStateTransition for CreditTransfer {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<PutSettings>) -> Result<(), Error> {
        let state_transition = self.build_state_transition(sdk, settings.clone()).await?;
        state_transition.broadcast(sdk, settings).await
    }

    async fn wait_for_response<T: TryFrom<StateTransitionProofResult>>(
        &self,
        _sdk: &Sdk,
        _settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        Err(Error::InvalidCreditTransfer(
            "waiting for a previously broadcast credit transfer is not supported; \
use broadcast_and_wait instead"
                .to_string(),
        ))
    }

    async fn broadcast_and_wait<T: TryFrom<StateTransitionProofResult>>(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<T, Error> {
        let state_transition = self.build_state_transition(sdk, settings.clone()).await?;
        state_transition.broadcast_and_wait(sdk, settings).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::{AddressWitness, PlatformAddress};
    use dpp::identifier::Identifier;
    use dpp::identity::signer::Signer;
    use dpp::identity::v0::IdentityV0;
    use dpp::platform_value::BinaryData;
    use dpp::ProtocolError;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn identifier(index: u8) -> Identifier {
        let bytes = [index; 32];
        bytes.into()
    }

    #[test]
    fn identity_transfer_plan_succeeds() {
        let sender_id = identifier(1);
        let recipient_id = identifier(2);

        let transfer = build_identity_transfer(sender_id, recipient_id, 42);

        match &transfer.transfer_kind {
            TransferKind::Identity(selection) => {
                assert_eq!(selection.config.identity_id(), sender_id);
                assert_eq!(selection.plan.recipient_id, recipient_id);
                assert_eq!(selection.plan.amount, 42);
            }
            _ => panic!("builder produced unexpected transfer kind"),
        }
    }

    #[test]
    fn identity_transfer_plan_requires_identity_input() {
        let recipient_id = identifier(3);
        let mut builder = CreditTransfer::builder();
        builder
            .input((BTreeMap::<PlatformAddress, Credits>::new(), vec![]))
            .expect("input should be accepted");
        builder
            .output(recipient_id, 10)
            .expect("output should be accepted");

        let err = builder.build().unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    fn identity_transfer_plan_requires_identity_output() {
        let sender_id = identifier(4);
        let mut builder = CreditTransfer::builder();
        builder
            .identity_input(identity_with_id(sender_id), test_signer(), None)
            .expect("input should be accepted");
        builder
            .output(PlatformAddress::default(), 10)
            .expect("output should be accepted");

        let err = builder.build().unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    fn build_identity_transfer(
        sender_id: Identifier,
        recipient_id: Identifier,
        amount: Credits,
    ) -> CreditTransfer {
        let mut builder = CreditTransfer::builder();
        builder
            .identity_input(identity_with_id(sender_id), test_signer(), None)
            .expect("failed to add input");
        builder
            .output(recipient_id, amount)
            .expect("failed to add output");
        builder.build().expect("builder should produce transfer")
    }

    fn identity_with_id(identifier: Identifier) -> Identity {
        Identity::V0(IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    fn test_signer() -> Arc<TestIdentitySigner> {
        Arc::new(TestIdentitySigner)
    }

    #[derive(Clone, Debug)]
    struct TestIdentitySigner;

    impl Signer<IdentityPublicKey> for TestIdentitySigner {
        fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Ok(BinaryData::new(vec![]))
        }

        fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "not implemented for tests".to_string(),
            ))
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }
}
