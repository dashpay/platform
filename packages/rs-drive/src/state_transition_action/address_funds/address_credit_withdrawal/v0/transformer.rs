use crate::state_transition_action::address_funds::address_credit_withdrawal::v0::AddressCreditWithdrawalTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::{Document, DocumentV0};
use dpp::fee::Credits;
use dpp::platform_value::platform_value;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::state_transitions::address_funds::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use std::collections::BTreeMap;

impl AddressCreditWithdrawalTransitionActionV0 {
    /// Transforms the state transition into an action using pre-validated inputs with remaining balances.
    ///
    /// # Arguments
    /// * `value` - The state transition
    /// * `inputs_with_remaining_balance` - Pre-validated inputs with remaining balances after spending
    /// * `creation_time_ms` - The creation time for the withdrawal document
    pub fn try_from_transition(
        value: &AddressCreditWithdrawalTransitionV0,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        creation_time_ms: u64,
    ) -> ConsensusValidationResult<Self> {
        let AddressCreditWithdrawalTransitionV0 {
            output,
            fee_strategy,
            output_script,
            core_fee_per_byte,
            pooling,
            user_fee_increase,
            ..
        } = value;

        // Sum all remaining balances from inputs
        let total_remaining: Credits = inputs_with_remaining_balance
            .values()
            .map(|(_, balance)| *balance)
            .sum();

        // Calculate the withdrawal amount: total remaining minus output (if any)
        let amount = match output {
            Some((_, output_amount)) => total_remaining - output_amount,
            None => total_remaining,
        };

        // Generate entropy from inputs for document ID
        // Use first input address and nonce as entropy source
        let mut entropy = Vec::new();
        if let Some((first_address, (first_nonce, _))) =
            inputs_with_remaining_balance.first_key_value()
        {
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
            withdrawal::properties::AMOUNT: amount,
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
            amount,
        })
    }
}
