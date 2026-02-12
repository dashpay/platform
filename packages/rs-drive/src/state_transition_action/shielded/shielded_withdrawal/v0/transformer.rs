use crate::state_transition_action::shielded::shielded_withdrawal::v0::ShieldedWithdrawalTransitionActionV0;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::{Document, DocumentV0};
use dpp::fee::Credits;
use dpp::platform_value::platform_value;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;

impl ShieldedWithdrawalTransitionActionV0 {
    /// Transforms the shielded withdrawal transition into an action
    pub fn try_from_transition(
        value: &ShieldedWithdrawalTransitionV0,
        nullifiers: Vec<[u8; 32]>,
        note_commitments: Vec<[u8; 32]>,
        encrypted_notes: Vec<Vec<u8>>,
        anchor: [u8; 32],
        current_total_balance: Credits,
        creation_time_ms: u64,
    ) -> ConsensusValidationResult<Self> {
        // Generate entropy from first nullifier + output_script for document ID
        let mut entropy = Vec::new();
        if let Some(first_nullifier) = nullifiers.first() {
            entropy.extend_from_slice(first_nullifier);
        }
        entropy.extend_from_slice(value.output_script.as_bytes());

        // The owner_id is the contract owner
        let owner_id = withdrawals_contract::OWNER_ID;

        let document_id = Document::generate_document_id_v0(
            &withdrawals_contract::ID,
            &owner_id,
            withdrawal::NAME,
            &entropy,
        );

        let document_data = platform_value!({
            withdrawal::properties::AMOUNT: value.amount,
            withdrawal::properties::CORE_FEE_PER_BYTE: value.core_fee_per_byte,
            withdrawal::properties::POOLING: value.pooling,
            withdrawal::properties::OUTPUT_SCRIPT: value.output_script.as_bytes(),
            withdrawal::properties::STATUS: withdrawals_contract::WithdrawalStatus::QUEUED,
        });

        let withdrawal_document = DocumentV0 {
            id: document_id,
            owner_id,
            properties: document_data
                .into_btree_string_map()
                .expect("platform_value macro produces a map"),
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

        ConsensusValidationResult::new_with_data(ShieldedWithdrawalTransitionActionV0 {
            amount: value.amount,
            nullifiers,
            note_commitments,
            encrypted_notes,
            anchor,
            core_fee_per_byte: value.core_fee_per_byte,
            pooling: value.pooling,
            output_script: value.output_script.clone(),
            user_fee_increase: value.user_fee_increase,
            current_total_balance,
            prepared_withdrawal_document: withdrawal_document,
        })
    }
}
