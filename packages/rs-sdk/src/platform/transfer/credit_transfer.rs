use super::address::{
    classify_address_transfer, classify_address_withdrawal, AddressTransferPlan,
    AddressWithdrawalPlan, AddressWithdrawalRequest,
};
use super::identity::{classify_identity_transfer, IdentityTransferSelection};
use super::top_up::{classify_address_top_up, AddressTopUpPlan};
use super::types::{
    AddressSigner, IdentitySigner, IdentityTransferConfig, TransferInput, TransferOutput,
};
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::errors::consensus::basic::state_transition::{
    OutputBelowMinimumError, TransitionNoInputsError, TransitionNoOutputsError,
};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::withdrawal::Pooling;
use std::collections::BTreeMap;

/// Aggregated credit transfer description created via [`CreditTransferBuilder`].
///
/// Supports the following state transition types:
/// - [IdentityCreditTransferTransition](dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition)
/// - [AddressFundsTransferTransition](dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition)
/// - [AddressFundingFromAssetLockTransition](dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition)
/// - [AddressCreditWithdrawalTransition](dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition)
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
            TransferKind::AddressTopUp(plan) => plan.build_state_transition(sdk, settings).await,
            TransferKind::AddressWithdrawal(plan) => {
                plan.build_state_transition(sdk, settings).await
            }
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
    /// Top up Platform addresses using asset lock proofs.
    AddressTopUp(AddressTopUpPlan),
    /// Withdraw credits from Platform addresses to a Core script.
    AddressWithdrawal(AddressWithdrawalPlan),
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
    /// Optional address withdrawal configuration.
    withdrawal: Option<AddressWithdrawalRequest>,
}

impl std::fmt::Debug for CreditTransferBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreditTransferBuilder")
            .field("input_count", &self.inputs.len())
            .field("outputs", &self.outputs)
            .field("has_address_signer", &self.address_signer.is_some())
            .field("address_fee_strategy", &self.address_fee_strategy)
            .field("has_withdrawal", &self.withdrawal.is_some())
            .finish()
    }
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

    /// Set a custom fee strategy for an address withdrawal.
    pub fn withdrawal_fee_strategy(
        &mut self,
        strategy: AddressFundsFeeStrategy,
    ) -> Result<&mut Self, Error> {
        let config = self.withdrawal_config_mut()?;
        config.fee_strategy = strategy;
        Ok(self)
    }

    /// Override the Core chain fee-per-byte value for withdrawals.
    pub fn withdrawal_core_fee_per_byte(&mut self, fee_per_byte: u32) -> Result<&mut Self, Error> {
        let config = self.withdrawal_config_mut()?;
        config.core_fee_per_byte = fee_per_byte;
        Ok(self)
    }

    /// Update the pooling preference for a withdrawal transition.
    pub fn withdrawal_pooling(&mut self, pooling: Pooling) -> Result<&mut Self, Error> {
        let config = self.withdrawal_config_mut()?;
        config.pooling = pooling;
        Ok(self)
    }

    /// Configure a change output for transitions supporting it.
    pub fn change<D>(&mut self, destination: D, amount: Credits) -> Result<&mut Self, Error>
    where
        D: TryInto<TransferOutput>,
        <D as TryInto<TransferOutput>>::Error: ToString,
    {
        let transfer_output = destination
            .try_into()
            .map_err(|err| Error::InvalidCreditTransfer(err.to_string()))?;

        match transfer_output {
            TransferOutput::PlatformAddress(address) => {
                self.configure_withdrawal_change_output(address, amount)?
            }
            _ => {
                return Err(Error::InvalidCreditTransfer(
                    "change output currently supports only Platform addresses".to_string(),
                ))
            }
        }

        Ok(self)
    }

    /// Configure an optional Platform address to receive change after withdrawal.
    pub fn withdrawal_change_output(
        &mut self,
        address: PlatformAddress,
        amount: Credits,
    ) -> Result<&mut Self, Error> {
        self.change(address, amount)
    }

    /// Adds an output destination with the specified amount.
    pub fn output<D>(&mut self, destination: D, amount: Credits) -> Result<&mut Self, Error>
    where
        D: TryInto<TransferOutput>,
        <D as TryInto<TransferOutput>>::Error: ToString,
    {
        let transfer_output = destination
            .try_into()
            .map_err(|err| Error::InvalidCreditTransfer(err.to_string()))?;

        match transfer_output {
            TransferOutput::CoreScript(bytes) => {
                self.configure_withdrawal_destination(CoreScript::from_bytes(bytes))?;
                return Ok(self);
            }
            TransferOutput::DefaultWithdrawal => {
                return Err(Error::InvalidCreditTransfer(
                    "default withdrawal destination is not supported".to_string(),
                ))
            }
            other => {
                if self.withdrawal.is_some() {
                    return Err(Error::InvalidCreditTransfer(
                        "address withdrawals cannot define additional outputs".to_string(),
                    ));
                }

                if amount == 0 {
                    return Err(Error::from(OutputBelowMinimumError::new(amount, 1)));
                }

                let entry = self.outputs.entry(other).or_insert(0);
                *entry = entry.saturating_add(amount);
            }
        }

        Ok(self)
    }

    fn withdrawal_config_mut(&mut self) -> Result<&mut AddressWithdrawalRequest, Error> {
        self.withdrawal.as_mut().ok_or_else(|| {
            Error::InvalidCreditTransfer(
                "configure a withdrawal destination before customizing settings".to_string(),
            )
        })
    }

    fn configure_withdrawal_change_output(
        &mut self,
        address: PlatformAddress,
        amount: Credits,
    ) -> Result<(), Error> {
        if amount == 0 {
            return Err(Error::from(OutputBelowMinimumError::new(amount, 1)));
        }
        let config = self.withdrawal_config_mut()?;
        config.change_output = Some((address, amount));
        Ok(())
    }

    fn configure_withdrawal_destination(&mut self, script: CoreScript) -> Result<(), Error> {
        if !self.outputs.is_empty() {
            return Err(Error::InvalidCreditTransfer(
                "address withdrawals cannot define standard outputs".to_string(),
            ));
        }

        if self.withdrawal.is_some() {
            return Err(Error::InvalidCreditTransfer(
                "address withdrawal already configured".to_string(),
            ));
        }

        self.withdrawal = Some(AddressWithdrawalRequest::new(script));
        Ok(())
    }

    /// Finalizes the builder and returns an immutable `CreditTransfer`.
    pub fn build(self) -> Result<CreditTransfer, Error> {
        if self.inputs.is_empty() {
            return Err(Error::from(TransitionNoInputsError::new()));
        }

        let has_withdrawal = self.withdrawal.is_some();
        if self.outputs.is_empty() && !has_withdrawal {
            return Err(Error::from(TransitionNoOutputsError::new()));
        }

        let CreditTransferBuilder {
            inputs,
            outputs,
            address_signer,
            address_fee_strategy,
            withdrawal,
        } = self;

        if let Some(withdrawal_config) = withdrawal {
            if !outputs.is_empty() {
                return Err(Error::InvalidCreditTransfer(
                    "address withdrawals cannot define standard outputs".to_string(),
                ));
            }

            let signer = address_signer.ok_or_else(|| {
                Error::InvalidCreditTransfer(
                    "address transfers require an address signer configuration".to_string(),
                )
            })?;

            let transfer_kind = TransferKind::AddressWithdrawal(classify_address_withdrawal(
                &inputs,
                signer,
                withdrawal_config,
            )?);
            return Ok(CreditTransfer { transfer_kind });
        }

        let outputs_are_identities = outputs
            .keys()
            .all(|output| matches!(output, TransferOutput::Identity(_)));
        if outputs_are_identities {
            let transfer_kind =
                TransferKind::Identity(classify_identity_transfer(&inputs, &outputs)?);
            return Ok(CreditTransfer { transfer_kind });
        }

        let outputs_are_addresses = outputs
            .keys()
            .all(|output| matches!(output, TransferOutput::PlatformAddress(_)));
        if outputs_are_addresses {
            if inputs
                .iter()
                .any(|input| matches!(input, TransferInput::AssetLock { .. }))
            {
                let signer = address_signer.clone().ok_or_else(|| {
                    Error::InvalidCreditTransfer(
                        "address transfers require an address signer configuration".to_string(),
                    )
                })?;
                let transfer_kind = TransferKind::AddressTopUp(classify_address_top_up(
                    &inputs,
                    &outputs,
                    signer,
                    address_fee_strategy.clone(),
                )?);
                return Ok(CreditTransfer { transfer_kind });
            }

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
            return Ok(CreditTransfer { transfer_kind });
        }

        if outputs.is_empty() {
            Err(Error::from(TransitionNoOutputsError::new()))
        } else {
            Err(Error::InvalidCreditTransfer(
                "unsupported credit transfer outputs".to_string(),
            ))
        }
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
    use dpp::dashcore::{Network, PrivateKey};
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::identity::signer::Signer;
    use dpp::identity::v0::IdentityV0;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::AssetLockProof;
    use dpp::ProtocolError;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn identifier(index: u8) -> Identifier {
        let bytes = [index; 32];
        bytes.into()
    }

    fn platform_address(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    fn asset_lock_input() -> TransferInput {
        TransferInput::from_asset_lock(AssetLockProof::default(), test_asset_lock_private_key())
    }

    fn test_asset_lock_private_key() -> PrivateKey {
        PrivateKey::from_byte_array(&[11u8; 32], Network::Testnet).expect("private key")
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

    #[test]
    fn change_requires_destination_configuration() {
        let mut builder = CreditTransfer::builder();
        let err = builder.change(platform_address(10), 5).unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    fn withdrawal_plan_requires_signer() {
        let mut builder = CreditTransfer::builder();
        builder
            .input(TransferInput::from_addresses(
                BTreeMap::from([(platform_address(1), 10)]),
                vec![],
            ))
            .expect("input should be accepted");
        builder
            .output(CoreScript::from_bytes(vec![0u8; 1]), 0)
            .expect("withdrawal destination should be configured");

        let err = builder.build().unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    fn withdrawal_plan_succeeds() {
        let mut builder = CreditTransfer::builder();
        builder
            .input(TransferInput::from_addresses(
                BTreeMap::from([(platform_address(2), 10)]),
                vec![],
            ))
            .expect("input should be accepted");
        builder.address_signer(Arc::new(TestAddressSigner));
        builder
            .output(CoreScript::from_bytes(vec![1u8; 2]), 0)
            .expect("withdrawal destination should be configured");
        builder
            .change(platform_address(3), 5)
            .expect("change output should be set");

        let transfer = builder.build().expect("builder should produce transfer");
        match transfer.transfer_kind {
            TransferKind::AddressWithdrawal(_) => {}
            _ => panic!("expected address withdrawal variant"),
        }
    }

    #[test]
    fn address_top_up_requires_signer() {
        let mut builder = CreditTransfer::builder();
        builder
            .input(asset_lock_input())
            .expect("input should be accepted");
        builder
            .output(platform_address(6), 12)
            .expect("output should be accepted");

        let err = builder.build().unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    fn address_top_up_plan_succeeds() {
        let mut builder = CreditTransfer::builder();
        builder
            .input(asset_lock_input())
            .expect("input should be accepted");
        builder.address_signer(Arc::new(TestAddressSigner));
        builder
            .output(platform_address(7), 18)
            .expect("output should be accepted");

        let transfer = builder.build().expect("transfer should be built");
        match transfer.transfer_kind {
            TransferKind::AddressTopUp(_) => {}
            _ => panic!("expected address top up variant"),
        }
    }

    #[test]
    fn change_rejects_unsupported_destination() {
        let mut builder = CreditTransfer::builder();
        builder
            .input(TransferInput::from_addresses(
                BTreeMap::from([(platform_address(4), 10)]),
                vec![],
            ))
            .expect("input should be accepted");
        builder.address_signer(Arc::new(TestAddressSigner));
        builder
            .output(CoreScript::from_bytes(vec![6u8; 2]), 0)
            .expect("withdrawal destination should be configured");

        let err = builder.change(identifier(1), 5).unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    fn withdraw_cannot_mix_with_standard_outputs() {
        let mut builder = CreditTransfer::builder();
        builder
            .output(CoreScript::from_bytes(vec![5u8; 2]), 0)
            .expect("configured withdrawal");
        let err = builder.output(identifier(9), 10).unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    /// Example: transfer 10 credits between two identities.
    fn example_identity_transfer_flow_showcases_builder_usage() {
        let sender_id = identifier(11);
        let recipient_id = identifier(22);

        let mut builder = CreditTransfer::builder();
        // 1. Provide identity balance and signer context.
        builder
            .identity_input(identity_with_id(sender_id), test_signer(), None)
            .expect("identity funding should be accepted");
        // 2. Describe the output target and amount.
        builder
            .output(recipient_id, 10)
            .expect("identity output should be accepted");

        let transfer = builder.build().expect("builder should succeed");
        match transfer.transfer_kind {
            TransferKind::Identity(_) => {}
            _ => panic!("identity flow should produce an Identity transfer"),
        }

        // Real-world broadcast (commented out to keep tests offline):
        // let sdk = acquire_sdk();
        // sdk.sync(|sdk| async move {
        //     transfer
        //         .broadcast_and_wait::<StateTransitionProofResult>(sdk, None)
        //         .await
        //         .expect("state transition should be accepted");
        // });
    }

    #[test]
    /// Example: transfer 25 credits between Platform addresses.
    fn example_address_transfer_flow_showcases_platform_inputs() {
        let mut builder = CreditTransfer::builder();
        builder
            .input(TransferInput::from_addresses(
                BTreeMap::from([(platform_address(8), 50)]),
                vec![],
            ))
            .expect("address funding should be accepted");
        builder.address_signer(Arc::new(TestAddressSigner));
        builder
            .output(platform_address(9), 25)
            .expect("platform address output should be accepted");

        let transfer = builder.build().expect("builder should succeed");
        match transfer.transfer_kind {
            TransferKind::Address(_) => {}
            _ => panic!("address flow should produce an Address transfer"),
        }

        // Offline-friendly example:
        // transfer.broadcast(&sdk, None).await?;
    }

    #[test]
    ///  Example: withdraw to a Core script with change sent back to Platform.
    fn example_withdrawal_flow_showcases_core_withdrawals() {
        let mut builder = CreditTransfer::builder();
        builder
            .input(TransferInput::from_addresses(
                BTreeMap::from([(platform_address(10), 75)]),
                vec![],
            ))
            .expect("funding should be accepted");
        builder.address_signer(Arc::new(TestAddressSigner));
        builder
            .output(CoreScript::from_bytes(vec![0x51]), 0) // simple OP_TRUE script for illustration
            .expect("core script should configure withdrawal");
        builder
            .change(platform_address(11), 5)
            .expect("change output should be accepted");

        let transfer = builder.build().expect("builder should succeed");
        match transfer.transfer_kind {
            TransferKind::AddressWithdrawal(_) => {}
            _ => panic!("withdrawal flow should produce AddressWithdrawal transfer"),
        }

        // After funding the Core chain fee account, you could broadcast:
        // transfer.broadcast_and_wait::<StateTransitionProofResult>(&sdk, None).await?;
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
