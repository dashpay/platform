use crate::error::Error;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::types::execution_operation::ValidationOperation;
use dpp::consensus::state::address_funds::{AddressDoesNotExistError, AddressTooLittleFundsError};
use dpp::fee::Credits;
use dpp::identity::{KeyCount, KeyOfType};
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

/// This will validate that all addresses have at least the minimum required balance in the state
pub(super) fn validate_addresses_for_balances_and_nonces_v0(
    minimum_balances: &BTreeMap<KeyOfType, Credits>,
    drive: &Drive,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<()>, Error> {
    // If there are no addresses to validate, return early
    if minimum_balances.is_empty() {
        return Ok(ConsensusValidationResult::new());
    }

    execution_context.add_operation(ValidationOperation::RetrieveKeyOfTypeNonceAndBalance(minimum_balances.len() as KeyCount));

    // Fetch the actual balances and nonces from the state
    let actual_balances = drive.fetch_balances_with_nonces(
        minimum_balances.keys(),
        transaction,
        platform_version,
    )?;

    // Check that each address has at least the minimum required balance
    for (key_of_type, minimum_balance) in minimum_balances {
        match actual_balances.get(key_of_type) {
            Some(Some((_nonce, actual_balance))) => {
                // Address exists in state, check if balance is sufficient
                if actual_balance < minimum_balance {
                    return Ok(ConsensusValidationResult::new_with_error(
                        AddressTooLittleFundsError::new(
                            key_of_type.clone(),
                            *actual_balance,
                            *minimum_balance,
                        )
                        .into(),
                    ));
                }
            }
            Some(None) | None => {
                // Address not found in state
                return Ok(ConsensusValidationResult::new_with_error(
                    AddressDoesNotExistError::new(key_of_type.clone()).into(),
                ));
            }
        }
    }

    // All addresses have sufficient balances
    Ok(ConsensusValidationResult::new())
}
