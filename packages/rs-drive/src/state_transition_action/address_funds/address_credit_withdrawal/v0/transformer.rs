use crate::state_transition_action::address_funds::address_credit_withdrawal::v0::AddressCreditWithdrawalTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::state::address_funds::address_does_not_exist_error::AddressDoesNotExistError;
use dpp::consensus::state::address_funds::address_not_enough_funds_error::AddressNotEnoughFundsError;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::{Document, DocumentV0};
use dpp::fee::Credits;
use dpp::platform_value::platform_value;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use std::collections::BTreeMap;

impl AddressCreditWithdrawalTransitionActionV0 {
    /// Transforms the state transition into an action by validating inputs against provided balances.
    ///
    /// For each input address:
    /// 1. Validates the address exists in the provided balances
    /// 2. Validates there is sufficient balance for the claimed spend amount
    /// 3. Computes the remaining balance after the withdrawal
    /// 4. Creates the prepared withdrawal document
    pub fn try_from_transition(
        value: &AddressCreditWithdrawalTransitionV0,
        input_balances: BTreeMap<PlatformAddress, Credits>,
        creation_time_ms: u64,
    ) -> ConsensusValidationResult<Self> {
        let AddressCreditWithdrawalTransitionV0 {
            inputs,
            output,
            fee_strategy,
            core_fee_per_byte,
            pooling,
            output_script,
            user_fee_increase,
            ..
        } = value;

        // Validate each input and compute remaining balances
        let mut inputs_with_remaining_balance = BTreeMap::new();
        let mut total_withdrawal_amount: Credits = 0;

        for (address, (expected_nonce, spend_amount)) in inputs {
            match input_balances.get(address) {
                Some(actual_balance) => {
                    // Address exists, check if there's enough balance
                    if *actual_balance < *spend_amount {
                        return ConsensusValidationResult::new_with_error(
                            AddressNotEnoughFundsError::new(
                                *address,
                                *actual_balance,
                                *spend_amount,
                            )
                            .into(),
                        );
                    }

                    // Compute remaining balance after the withdrawal
                    let remaining_balance = actual_balance - spend_amount;
                    inputs_with_remaining_balance
                        .insert(*address, (*expected_nonce, remaining_balance));

                    total_withdrawal_amount += spend_amount;
                }
                None => {
                    // Address does not exist
                    return ConsensusValidationResult::new_with_error(
                        AddressDoesNotExistError::new(*address).into(),
                    );
                }
            }
        }

        // Subtract the change output from withdrawal amount if present
        if let Some((_, change_amount)) = output {
            total_withdrawal_amount -= change_amount;
        }

        // Generate entropy from inputs for document ID
        // Use first input address and nonce as entropy source
        let mut entropy = Vec::new();
        if let Some((first_address, (first_nonce, _))) = inputs.first_key_value() {
            entropy.extend_from_slice(&first_nonce.to_be_bytes());
            entropy.extend_from_slice(first_address.to_bytes().as_slice());
        }
        entropy.extend_from_slice(output_script.as_bytes());

        // The owner_id is the contract owner
        let owner_id = withdrawals_contract::OWNER_ID;

        let document_id = Document::generate_document_id_v0(
            &withdrawals_contract::ID,
            &owner_id,
            withdrawal::NAME,
            &entropy,
        );

        let document_data = platform_value!({
            withdrawal::properties::AMOUNT: total_withdrawal_amount,
            withdrawal::properties::CORE_FEE_PER_BYTE: *core_fee_per_byte,
            withdrawal::properties::POOLING: *pooling,
            withdrawal::properties::OUTPUT_SCRIPT: output_script.as_bytes(),
            withdrawal::properties::STATUS: withdrawals_contract::WithdrawalStatus::QUEUED,
        });

        let withdrawal_document = DocumentV0 {
            id: document_id,
            owner_id,
            properties: document_data.into_btree_string_map().unwrap(),
            revision: Some(1),
            created_at: Some(creation_time_ms),
            updated_at: Some(creation_time_ms),
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        ConsensusValidationResult::new_with_data(AddressCreditWithdrawalTransitionActionV0 {
            inputs_with_remaining_balance,
            output: *output,
            fee_strategy: fee_strategy.clone(),
            user_fee_increase: *user_fee_increase,
            prepared_withdrawal_document: withdrawal_document,
            amount: total_withdrawal_amount,
        })
    }
}
