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
use dpp::state_transition::shield_transition::ShieldTransition;
use drive::drive::Drive;
use drive::error::Error;
use drive::grovedb::TransactionArg;
use dpp::version::PlatformVersion;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};

pub(crate) trait StateTransitionAddressBalancesAndNoncesInnerValidation:
    StateTransitionWitnessSigned
{
    #[allow(clippy::type_complexity)]
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
impl StateTransitionAddressBalancesAndNoncesInnerValidation for ShieldTransition {}

/// Trait for validating address balances and nonces in state transitions.
pub trait StateTransitionAddressBalancesAndNoncesValidation {
    /// Returns true if this state transition requires address balance and nonce validation.
    fn has_addresses_balances_and_nonces_validation(&self) -> bool;

    /// Validates that input addresses have sufficient balance and correct nonces.
    /// Returns the remaining balances after the transition would consume funds.
    #[allow(clippy::type_complexity)]
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
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::Shield(_) => true,
            StateTransition::DataContractCreate(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::Batch(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_) => false,
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
            StateTransition::Shield(st) => st
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
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_) => {
                Ok(ConsensusValidationResult::new_with_data(BTreeMap::new()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_data_contract_create_st() -> StateTransition {
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::TryIntoPlatformVersioned;
        let platform_version = platform_version::version::PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, 1, platform_version.protocol_version);
        let transition: dpp::state_transition::data_contract_create_transition::DataContractCreateTransition =
            created_data_contract.try_into_platform_versioned(platform_version).unwrap();
        transition.into()
    }

    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
    use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
    use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::AddressFundsTransferTransition;
    use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
    use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
    use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
    use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
    use dpp::state_transition::shield_transition::ShieldTransition;
    use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;

    mod has_addresses_balances_and_nonces_validation {
        use super::*;

        #[test]
        fn should_return_true_for_address_based_transitions() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                (
                    "IdentityCreateFromAddresses",
                    StateTransition::IdentityCreateFromAddresses(
                        IdentityCreateFromAddressesTransition::V0(
                            IdentityCreateFromAddressesTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "AddressFundsTransfer",
                    StateTransition::AddressFundsTransfer(AddressFundsTransferTransition::V0(
                        AddressFundsTransferTransitionV0::default(),
                    )),
                ),
                (
                    "AddressFundingFromAssetLock",
                    StateTransition::AddressFundingFromAssetLock(
                        AddressFundingFromAssetLockTransition::V0(
                            AddressFundingFromAssetLockTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "AddressCreditWithdrawal",
                    StateTransition::AddressCreditWithdrawal(
                        AddressCreditWithdrawalTransition::V0(
                            AddressCreditWithdrawalTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "IdentityTopUpFromAddresses",
                    StateTransition::IdentityTopUpFromAddresses(
                        IdentityTopUpFromAddressesTransition::V0(
                            IdentityTopUpFromAddressesTransitionV0::default(),
                        ),
                    ),
                ),
                (
                    "Shield",
                    StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                        inputs: Default::default(),
                        actions: vec![],
                        amount: 0,
                        anchor: [0u8; 32],
                        proof: vec![],
                        binding_signature: [0u8; 64],
                        fee_strategy: vec![],
                        user_fee_increase: 0,
                        input_witnesses: vec![],
                    })),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    st.has_addresses_balances_and_nonces_validation(),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_identity_based_transitions() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                ("DataContractCreate", make_data_contract_create_st()),
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityTopUp",
                    StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
                        IdentityTopUpTransitionV0::default(),
                    )),
                ),
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
                (
                    "MasternodeVote",
                    StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                        MasternodeVoteTransitionV0::default(),
                    )),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    !st.has_addresses_balances_and_nonces_validation(),
                    "expected false for {}",
                    name
                );
            }
        }
    }
}
