use std::collections::BTreeMap;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::state_transition::TransitionOverMaxInputsError;
use dpp::consensus::state::address_funds::{AddressDoesNotExistError, AddressInvalidNonceError, AddressNotEnoughFundsError};
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::{StateTransition, StateTransitionWitnessSigned};
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use drive::drive::Drive;
use drive::error::Error;
use drive::grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};

pub(crate) trait StateTransitionAddressBalancesAndNoncesInnerValidation:
    StateTransitionWitnessSigned
{
    fn validate_address_balances_and_nonces_internal_validation(
        &self,
        drive: &Drive,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<BTreeMap<PlatformAddress, (AddressNonce, Credits)>>, Error>
    {
        let inputs = self.inputs();
        if inputs.is_empty() {
            return Ok(ConsensusValidationResult::new_with_data(BTreeMap::new()));
        }
        tracing::trace!(
            inputs = ?inputs,
            "Validating input address balances and nonces for state transition"
        );

        // Validate maximum inputs, we need to do this here so we don't go and check too much data
        // in the state.
        if inputs.len() > platform_version.dpp.state_transitions.max_address_inputs as usize {
            return Ok(ConsensusValidationResult::new_with_error(
                BasicError::TransitionOverMaxInputsError(TransitionOverMaxInputsError::new(
                    inputs.len().min(u16::MAX as usize) as u16,
                    platform_version.dpp.state_transitions.max_address_inputs,
                ))
                .into(),
            ));
        }

        execution_context.add_operation(ValidationOperation::RetrieveAddressNonceAndBalance(
            inputs.len() as u16,
        ));

        // Fetch actual balances and nonces from state
        let actual_balances =
            drive.fetch_balances_with_nonces(inputs.keys(), transaction, platform_version)?;

        let mut remaining_balances = BTreeMap::new();

        for (address, (expected_nonce, requested_amount)) in inputs {
            // Check if address exists in state
            let (state_nonce, actual_balance) = match actual_balances.get(address) {
                Some(Some((nonce, balance))) => (*nonce, *balance),
                Some(None) | None => {
                    // Address does not exist in state
                    return Ok(ConsensusValidationResult::new_with_error(
                        AddressDoesNotExistError::new(*address).into(),
                    ));
                }
            };

            // Check that the nonce hasn't reached max (address can't be used anymore)
            if state_nonce == u32::MAX as AddressNonce {
                return Ok(ConsensusValidationResult::new_with_error(
                    AddressInvalidNonceError::new(
                        *address,
                        *expected_nonce,
                        state_nonce, // Can't increment past max
                    )
                    .into(),
                ));
            }

            // Check that the nonce is exactly state_nonce + 1
            let expected_next_nonce = state_nonce.saturating_add(1);
            if *expected_nonce != expected_next_nonce {
                tracing::debug!(
                    ?address,
                    expected_nonce = expected_next_nonce,
                    provided_nonce = *expected_nonce,
                    "Invalid nonce for address {:?}: expected {}, got {}",
                    address,
                    expected_next_nonce,
                    *expected_nonce
                );
                return Ok(ConsensusValidationResult::new_with_error(
                    AddressInvalidNonceError::new(*address, *expected_nonce, expected_next_nonce)
                        .into(),
                ));
            }

            // Check that the address has enough balance
            if actual_balance < *requested_amount {
                return Ok(ConsensusValidationResult::new_with_error(
                    AddressNotEnoughFundsError::new(*address, actual_balance, *requested_amount)
                        .into(),
                ));
            }

            // Calculate remaining balance with updated nonce
            let remaining_balance = actual_balance - requested_amount;
            remaining_balances.insert(*address, (*expected_nonce, remaining_balance));
        }

        Ok(ConsensusValidationResult::new_with_data(remaining_balances))
    }
}

impl StateTransitionAddressBalancesAndNoncesInnerValidation
    for IdentityCreateFromAddressesTransition
{
}
impl StateTransitionAddressBalancesAndNoncesInnerValidation
    for IdentityTopUpFromAddressesTransition
{
}
impl StateTransitionAddressBalancesAndNoncesInnerValidation for AddressFundsTransferTransition {}
impl StateTransitionAddressBalancesAndNoncesInnerValidation
    for AddressFundingFromAssetLockTransition
{
}
impl StateTransitionAddressBalancesAndNoncesInnerValidation for AddressCreditWithdrawalTransition {}

/// Trait for validating address balances and nonces in state transitions.
pub trait StateTransitionAddressBalancesAndNoncesValidation {
    /// Returns true if this state transition requires address balance and nonce validation.
    fn has_addresses_balances_and_nonces_validation(&self) -> bool;

    /// Validates that input addresses have sufficient balance and correct nonces.
    /// Returns the remaining balances after the transition would consume funds.
    fn validate_address_balances_and_nonces(
        &self,
        drive: &Drive,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<BTreeMap<PlatformAddress, (AddressNonce, Credits)>>, Error>;
}

impl StateTransitionAddressBalancesAndNoncesValidation for StateTransition {
    fn has_addresses_balances_and_nonces_validation(&self) -> bool {
        match self {
            StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::IdentityTopUpFromAddresses(_) => true,
            StateTransition::DataContractCreate(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreditTransferToAddresses(_) => false,
        }
    }

    fn validate_address_balances_and_nonces(
        &self,
        drive: &Drive,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<BTreeMap<PlatformAddress, (AddressNonce, Credits)>>, Error>
    {
        match self {
            StateTransition::IdentityCreateFromAddresses(st) => st
                .validate_address_balances_and_nonces_internal_validation(
                    drive,
                    execution_context,
                    transaction,
                    platform_version,
                ),
            StateTransition::IdentityTopUpFromAddresses(st) => st
                .validate_address_balances_and_nonces_internal_validation(
                    drive,
                    execution_context,
                    transaction,
                    platform_version,
                ),
            StateTransition::AddressFundsTransfer(st) => st
                .validate_address_balances_and_nonces_internal_validation(
                    drive,
                    execution_context,
                    transaction,
                    platform_version,
                ),
            StateTransition::AddressFundingFromAssetLock(st) => st
                .validate_address_balances_and_nonces_internal_validation(
                    drive,
                    execution_context,
                    transaction,
                    platform_version,
                ),
            StateTransition::AddressCreditWithdrawal(st) => st
                .validate_address_balances_and_nonces_internal_validation(
                    drive,
                    execution_context,
                    transaction,
                    platform_version,
                ),
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreditTransferToAddresses(_) => {
                Ok(ConsensusValidationResult::new_with_data(BTreeMap::new()))
            }
        }
    }
}
