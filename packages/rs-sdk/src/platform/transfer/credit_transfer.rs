use super::address::{
    classify_address_transfer, classify_address_withdrawal, AddressTransferPlan,
    AddressWithdrawalPlan, AddressWithdrawalRequest,
};
use super::identity::{classify_identity_transfer, IdentityTransferSelection};
use super::top_up::{classify_address_top_up, AddressTopUpPlan};
#[cfg(test)]
use super::types::IdentitySigner;
use super::types::{AddressSigner, TransferInput, TransferOutput, TransferSigner};
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::errors::consensus::basic::state_transition::{
    OutputBelowMinimumError, TransitionNoInputsError, TransitionNoOutputsError,
};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
#[cfg(test)]
use dpp::identity::Identity;
#[cfg(test)]
use dpp::identity::IdentityPublicKey;
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
    /// Prepared state transition validated during build.
    state_transition: StateTransition,
}

impl CreditTransfer {
    /// Creates a new builder instance.
    pub fn builder() -> CreditTransferBuilder {
        CreditTransferBuilder::default()
    }

    /// Borrow the prepared state transition.
    fn state_transition(&self) -> &StateTransition {
        &self.state_transition
    }
}

fn finalize_inputs(inputs: Vec<TransferInput>) -> Result<Vec<TransferInput>, Error> {
    let identity_count = inputs
        .iter()
        .filter(|input| matches!(input, TransferInput::Identity(_)))
        .count();

    if identity_count > 1 {
        return Err(Error::InvalidCreditTransfer(
            "multiple identity inputs are not supported".to_string(),
        ));
    }

    Ok(inputs)
}

fn require_address_signer(
    signer: &mut Option<TransferSigner>,
    context: &str,
) -> Result<AddressSigner, Error> {
    match signer.take() {
        Some(TransferSigner::Address(address_signer)) => Ok(address_signer),
        Some(TransferSigner::Identity { .. }) => Err(Error::InvalidCreditTransfer(format!(
            "{context} requires a Platform address signer configuration",
        ))),
        Some(TransferSigner::PrivateKeys(_)) => Err(Error::InvalidCreditTransfer(
            "address private key signers are not supported yet".to_string(),
        )),
        None => Err(Error::InvalidCreditTransfer(format!(
            "{context} requires a signer configuration",
        ))),
    }
}

fn require_identity_signer(
    signer: &mut Option<TransferSigner>,
) -> Result<TransferSigner, Error> {
    match signer.take() {
        Some(identity_signer @ TransferSigner::Identity { .. }) => Ok(identity_signer),
        Some(other) => Err(Error::InvalidCreditTransfer(format!(
            "identity transfers require an identity signer, received {other:?}",
        ))),
        None => Err(Error::InvalidCreditTransfer(
            "identity transfers require a signer configuration".to_string(),
        )),
    }
}

fn require_private_keys(
    signer: &mut Option<TransferSigner>,
    context: &str,
) -> Result<Vec<Vec<u8>>, Error> {
    match signer.take() {
        Some(TransferSigner::PrivateKeys(keys)) if !keys.is_empty() => Ok(keys),
        Some(TransferSigner::PrivateKeys(_)) => Err(Error::InvalidCreditTransfer(
            "private key signer must include at least one key".to_string(),
        )),
        Some(_) => Err(Error::InvalidCreditTransfer(format!(
            "{context} requires raw private keys signer",
        ))),
        None => Err(Error::InvalidCreditTransfer(format!(
            "{context} requires a signer configuration",
        ))),
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

impl TransferKind {
    async fn build_state_transition(
        &self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        match self {
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

/// Builder used to configure `CreditTransfer` inputs and outputs.
#[derive(Default)]
pub struct CreditTransferBuilder {
    /// Funding inputs staged for the transfer.
    inputs: Vec<TransferInput>,
    /// Outputs aggregated by destination.
    outputs: BTreeMap<TransferOutput, Credits>,
    /// Signer used for the transfer (identity or address context).
    signer: Option<TransferSigner>,
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
            .field("has_signer", &self.signer.is_some())
            .field("address_fee_strategy", &self.address_fee_strategy)
            .field("has_withdrawal", &self.withdrawal.is_some())
            .finish()
    }
}

impl CreditTransferBuilder {
    /// Adds a funding source to the transfer.
    pub fn input<S>(&mut self, source: S) -> &mut Self
    where
        S: Into<TransferInput>,
    {
        self.inputs.push(source.into());
        self
    }

    /// Registers a signer used for the current transfer context.
    pub fn with_signer<S>(&mut self, signer: S) -> Result<&mut Self, Error>
    where
        S: Into<TransferSigner>,
    {
        self.signer = Some(signer.into());
        Ok(self)
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

    /// Finalizes the builder, constructs, and validates the state transition.
    pub async fn build(
        self,
        sdk: &Sdk,
        settings: Option<PutSettings>,
    ) -> Result<CreditTransfer, Error> {
        let transfer_kind = self.into_transfer_kind()?;
        let state_transition = transfer_kind.build_state_transition(sdk, settings).await?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
        Ok(CreditTransfer { state_transition })
    }

    fn into_transfer_kind(self) -> Result<TransferKind, Error> {
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
            signer,
            address_fee_strategy,
            withdrawal,
            ..
        } = self;
        let mut signer = signer;
        let inputs = finalize_inputs(inputs)?;

        if let Some(withdrawal_config) = withdrawal {
            if !outputs.is_empty() {
                return Err(Error::InvalidCreditTransfer(
                    "address withdrawals cannot define standard outputs".to_string(),
                ));
            }

            let signer = require_address_signer(&mut signer, "address withdrawal")?;
            let transfer_kind = TransferKind::AddressWithdrawal(classify_address_withdrawal(
                &inputs,
                signer,
                withdrawal_config,
            )?);
            return Ok(transfer_kind);
        }

        let outputs_are_identities = outputs
            .keys()
            .all(|output| matches!(output, TransferOutput::Identity(_)));
        if outputs_are_identities {
            let identity_signer = require_identity_signer(&mut signer)?;
            let transfer_kind = TransferKind::Identity(classify_identity_transfer(
                &inputs,
                &outputs,
                identity_signer,
            )?);
            return Ok(transfer_kind);
        }

        let outputs_are_addresses = outputs
            .keys()
            .all(|output| matches!(output, TransferOutput::PlatformAddress(_)));
        if outputs_are_addresses {
            if inputs
                .iter()
                .any(|input| matches!(input, TransferInput::AssetLock { .. }))
            {
                let mut private_keys = require_private_keys(&mut signer, "address top up")?;
                if private_keys.len() != 1 {
                    return Err(Error::InvalidCreditTransfer(
                        "address top up requires exactly one asset lock private key".to_string(),
                    ));
                }
                let asset_lock_private_key = private_keys.remove(0);
                let transfer_kind = TransferKind::AddressTopUp(classify_address_top_up(
                    &inputs,
                    &outputs,
                    asset_lock_private_key,
                    address_fee_strategy.clone(),
                )?);
                return Ok(transfer_kind);
            }

            let signer = require_address_signer(&mut signer, "address transfer")?;
            let transfer_kind = TransferKind::Address(classify_address_transfer(
                &inputs,
                &outputs,
                signer,
                address_fee_strategy,
            )?);
            return Ok(transfer_kind);
        }

        if outputs.is_empty() {
            Err(Error::from(TransitionNoOutputsError::new()))
        } else {
            Err(Error::InvalidCreditTransfer(
                "unsupported credit transfer outputs".to_string(),
            ))
        }
    }

    #[cfg(test)]
    fn build_transfer_kind(self) -> Result<TransferKind, Error> {
        self.into_transfer_kind()
    }
}

#[async_trait::async_trait]
impl BroadcastStateTransition for CreditTransfer {
    async fn broadcast(&self, sdk: &Sdk, settings: Option<PutSettings>) -> Result<(), Error> {
        self.state_transition().broadcast(sdk, settings).await
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
        self.state_transition()
            .broadcast_and_wait(sdk, settings)
            .await
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
        TransferInput::from_asset_lock(AssetLockProof::default())
    }

    fn test_asset_lock_private_key() -> PrivateKey {
        PrivateKey::from_byte_array(&[11u8; 32], Network::Testnet).expect("private key")
    }

    fn asset_lock_private_key_bytes() -> Vec<u8> {
        test_asset_lock_private_key()
            .inner
            .as_ref()
            .to_vec()
    }

    #[test]
    fn identity_transfer_plan_succeeds() {
        let sender_id = identifier(1);
        let recipient_id = identifier(2);

        let transfer_kind = build_identity_transfer(sender_id, recipient_id, 42);

        match transfer_kind {
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
        builder.input(TransferInput::from_addresses(
            BTreeMap::<PlatformAddress, Credits>::new(),
        ));
        builder
            .output(recipient_id, 10)
            .expect("output should be accepted");

        let err = builder.build_transfer_kind().unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    fn identity_transfer_plan_requires_identity_output() {
        let sender_id = identifier(4);
        let mut builder = CreditTransfer::builder();
        builder.input(identity_with_id(sender_id));
        builder
            .with_signer(TransferSigner::new(test_signer()))
            .expect("signer should be configured");
        builder
            .output(PlatformAddress::default(), 10)
            .expect("output should be accepted");

        let err = builder.build_transfer_kind().unwrap_err();
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
        builder.input(TransferInput::from_addresses(
            BTreeMap::from([(platform_address(1), 10)]),
        ));
        builder
            .output(CoreScript::from_bytes(vec![0u8; 1]), 0)
            .expect("withdrawal destination should be configured");

        let err = builder.build_transfer_kind().unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    fn withdrawal_plan_succeeds() {
        let mut builder = CreditTransfer::builder();
        builder.input(TransferInput::from_addresses(
            BTreeMap::from([(platform_address(2), 10)]),
        ));
        builder
            .with_signer(TransferSigner::address_signer(Arc::new(TestAddressSigner)))
            .expect("signer should be configured");
        builder
            .output(CoreScript::from_bytes(vec![1u8; 2]), 0)
            .expect("withdrawal destination should be configured");
        builder
            .change(platform_address(3), 5)
            .expect("change output should be set");

        let transfer_kind = builder
            .build_transfer_kind()
            .expect("builder should produce transfer");
        match transfer_kind {
            TransferKind::AddressWithdrawal(_) => {}
            _ => panic!("expected address withdrawal variant"),
        }
    }

    #[test]
    fn address_top_up_requires_signer() {
        let mut builder = CreditTransfer::builder();
        builder.input(asset_lock_input());
        builder
            .output(platform_address(6), 12)
            .expect("output should be accepted");

        let err = builder.build_transfer_kind().unwrap_err();
        assert!(matches!(err, Error::InvalidCreditTransfer(_)));
    }

    #[test]
    fn address_top_up_plan_succeeds() {
        let mut builder = CreditTransfer::builder();
        builder.input(asset_lock_input());
        builder
            .with_signer(TransferSigner::private_keys(vec![
                asset_lock_private_key_bytes(),
            ]))
            .expect("signer should be configured");
        builder
            .output(platform_address(7), 18)
            .expect("output should be accepted");

        let transfer_kind = builder
            .build_transfer_kind()
            .expect("transfer should be built");
        match transfer_kind {
            TransferKind::AddressTopUp(_) => {}
            _ => panic!("expected address top up variant"),
        }
    }

    #[test]
    fn change_rejects_unsupported_destination() {
        let mut builder = CreditTransfer::builder();
        builder.input(TransferInput::from_addresses(
            BTreeMap::from([(platform_address(4), 10)]),
        ));
        builder
            .with_signer(TransferSigner::address_signer(Arc::new(TestAddressSigner)))
            .expect("signer should be configured");
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
        builder.input(identity_with_id(sender_id));
        builder
            .with_signer(TransferSigner::new(test_signer()))
            .expect("signer should be configured");
        // 2. Describe the output target and amount.
        builder
            .output(recipient_id, 10)
            .expect("identity output should be accepted");

        let transfer_kind = builder
            .build_transfer_kind()
            .expect("builder should succeed");
        match transfer_kind {
            TransferKind::Identity(_) => {}
            _ => panic!("identity flow should produce an Identity transfer"),
        }

        // Real-world build/broadcast (commented out to keep tests offline):
        // let transfer = builder.build(&sdk, None).await?;
        // transfer
        //     .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
        //     .await?;
    }

    #[test]
    /// Example: transfer 25 credits between Platform addresses.
    fn example_address_transfer_flow_showcases_platform_inputs() {
        let mut builder = CreditTransfer::builder();
        builder.input(TransferInput::from_addresses(
            BTreeMap::from([(platform_address(8), 50)]),
        ));
        builder
            .with_signer(TransferSigner::address_signer(Arc::new(TestAddressSigner)))
            .expect("signer should be configured");
        builder
            .output(platform_address(9), 25)
            .expect("platform address output should be accepted");

        let transfer_kind = builder
            .build_transfer_kind()
            .expect("builder should succeed");
        match transfer_kind {
            TransferKind::Address(_) => {}
            _ => panic!("address flow should produce an Address transfer"),
        }

        // Offline-friendly example:
        // let transfer = builder.build(&sdk, None).await?;
        // transfer.broadcast(&sdk, None).await?;
    }

    #[test]
    /// Example: top up a Platform address using an asset lock funding source.
    fn example_address_top_up_flow_showcases_asset_lock_usage() {
        let mut builder = CreditTransfer::builder();
        // 1. Provide the asset lock proof/private key as the funding source.
        builder.input(asset_lock_input());
        builder
            .with_signer(TransferSigner::private_keys(vec![
                asset_lock_private_key_bytes(),
            ]))
            .expect("signer should be configured");
        // 2. Point the builder at the Platform address that should receive funds.
        builder
            .output(platform_address(12), 30)
            .expect("platform address output should be accepted");

        let transfer_kind = builder
            .build_transfer_kind()
            .expect("builder should succeed");
        match transfer_kind {
            TransferKind::AddressTopUp(_) => {}
            _ => panic!("top up flow should produce AddressTopUp transfer"),
        }

        // To submit the transition for real, call:
        // let transfer = builder.build(&sdk, None).await?;
        // transfer.broadcast_and_wait::<StateTransitionProofResult>(&sdk, None).await?;
    }

    #[test]
    ///  Example: withdraw to a Core script with change sent back to Platform.
    fn example_withdrawal_flow_showcases_core_withdrawals() {
        let mut builder = CreditTransfer::builder();
        builder.input(TransferInput::from_addresses(
            BTreeMap::from([(platform_address(10), 75)]),
        ));
        builder
            .with_signer(TransferSigner::address_signer(Arc::new(TestAddressSigner)))
            .expect("signer should be configured");
        builder
            .output(CoreScript::from_bytes(vec![0x51]), 0) // simple OP_TRUE script for illustration
            .expect("core script should configure withdrawal");
        builder
            .change(platform_address(11), 5)
            .expect("change output should be accepted");

        let transfer_kind = builder
            .build_transfer_kind()
            .expect("builder should succeed");
        match transfer_kind {
            TransferKind::AddressWithdrawal(_) => {}
            _ => panic!("withdrawal flow should produce AddressWithdrawal transfer"),
        }

        // After funding the Core chain fee account, you could broadcast:
        // let transfer = builder.build(&sdk, None).await?;
        // transfer.broadcast_and_wait::<StateTransitionProofResult>(&sdk, None).await?;
    }

    fn build_identity_transfer(
        sender_id: Identifier,
        recipient_id: Identifier,
        amount: Credits,
    ) -> TransferKind {
        let mut builder = CreditTransfer::builder();
        builder.input(identity_with_id(sender_id));
        builder
            .with_signer(TransferSigner::new(test_signer()))
            .expect("signer should be configured");
        builder
            .output(recipient_id, amount)
            .expect("failed to add output");
        builder
            .build_transfer_kind()
            .expect("builder should produce transfer")
    }

    fn identity_with_id(identifier: Identifier) -> Identity {
        Identity::V0(IdentityV0 {
            id: identifier,
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    fn test_signer() -> IdentitySigner {
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
