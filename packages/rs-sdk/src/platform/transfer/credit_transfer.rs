use super::identity::classify_identity_transfer;
use super::types::{DynIdentitySigner, IdentityTransferConfig, TransferInput, TransferOutput};
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::errors::consensus::basic::state_transition::{
    OutputBelowMinimumError, TransitionNoInputsError, TransitionNoOutputsError,
};
use dpp::fee::Credits;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Aggregated credit transfer description created via [`CreditTransferBuilder`].
pub struct CreditTransfer {
    inputs: Vec<TransferInput>,
    outputs: BTreeMap<TransferOutput, Credits>,
}

impl CreditTransfer {
    /// Creates a new builder instance.
    pub fn builder() -> CreditTransferBuilder {
        CreditTransferBuilder::default()
    }

    /// Funding sources participating in this transfer.
    pub fn inputs(&self) -> &[TransferInput] {
        &self.inputs
    }

    /// Outputs and aggregated credit amounts.
    pub fn outputs(&self) -> &BTreeMap<TransferOutput, Credits> {
        &self.outputs
    }

    /// Decompose the transfer into owned inputs and outputs.
    pub fn into_parts(self) -> (Vec<TransferInput>, BTreeMap<TransferOutput, Credits>) {
        (self.inputs, self.outputs)
    }

    #[cfg(test)]
    fn from_parts_for_tests(
        inputs: Vec<TransferInput>,
        outputs: BTreeMap<TransferOutput, Credits>,
    ) -> Self {
        CreditTransfer { inputs, outputs }
    }

    /// Execute a credit transfer between two identities using the configured plan.
    pub async fn broadcast_and_wait(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<(u64, u64), Error> {
        let identity_context = classify_identity_transfer(&self.inputs, &self.outputs)?;

        identity_context
            .config
            .execute(
                sdk,
                identity_context.plan.recipient_id,
                identity_context.plan.amount,
                settings,
            )
            .await
    }

    async fn build_state_transition(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let identity_context = classify_identity_transfer(&self.inputs, &self.outputs)?;
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        identity_context
            .config
            .state_transition(
                sdk,
                identity_context.plan.recipient_id,
                identity_context.plan.amount,
                user_fee_increase,
                settings,
            )
            .await
    }
}

/// Builder used to configure `CreditTransfer` inputs and outputs.
#[derive(Default)]
pub struct CreditTransferBuilder {
    inputs: Vec<TransferInput>,
    outputs: BTreeMap<TransferOutput, Credits>,
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
    pub fn identity_input(
        &mut self,
        identity: Identity,
        signer: Arc<DynIdentitySigner>,
        signing_key: Option<IdentityPublicKey>,
    ) -> Result<&mut Self, Error> {
        let config = IdentityTransferConfig::new(identity, signer, signing_key);
        self.inputs.push(TransferInput::Identity(config));
        Ok(self)
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

        Ok(CreditTransfer {
            inputs: self.inputs,
            outputs: self.outputs,
        })
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
    use super::super::identity::classify_identity_transfer;
    use super::super::types::{DynIdentitySigner, IdentityTransferConfig};
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

        let context = classify_identity_transfer(&transfer.inputs, &transfer.outputs)
            .expect("plan should build");

        assert_eq!(context.config.identity_id(), sender_id);
        assert_eq!(context.plan.recipient_id, recipient_id);
        assert_eq!(context.plan.amount, 42);
    }

    #[test]
    fn identity_transfer_plan_requires_identity_input() {
        let recipient_id = identifier(3);
        let transfer = CreditTransfer::from_parts_for_tests(
            vec![TransferInput::from_addresses(BTreeMap::new(), vec![])],
            BTreeMap::from([(TransferOutput::Identity(recipient_id), 10)]),
        );

        let err = classify_identity_transfer(&transfer.inputs, &transfer.outputs).unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    fn identity_transfer_plan_requires_identity_output() {
        let sender_id = identifier(4);
        let transfer = CreditTransfer::from_parts_for_tests(
            vec![identity_input(sender_id)],
            BTreeMap::from([(
                TransferOutput::PlatformAddress(PlatformAddress::default()),
                10,
            )]),
        );

        let err = classify_identity_transfer(&transfer.inputs, &transfer.outputs).unwrap_err();
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

    fn identity_input(identifier: Identifier) -> TransferInput {
        let identity = identity_with_id(identifier);
        let signer = test_signer();
        TransferInput::Identity(IdentityTransferConfig::new(identity, signer, None))
    }

    fn identity_with_id(identifier: Identifier) -> Identity {
        Identity::V0(IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    fn test_signer() -> Arc<DynIdentitySigner> {
        Arc::new(TestIdentitySigner) as Arc<DynIdentitySigner>
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
