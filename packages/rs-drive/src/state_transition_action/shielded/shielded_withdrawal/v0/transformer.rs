use crate::state_transition_action::shielded::shielded_withdrawal::v0::ShieldedWithdrawalTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::consensus::basic::identity::InvalidIdentityCreditWithdrawalTransitionAmountError;
use dpp::consensus::basic::state_transition::WithdrawalBelowMinAmountError;
use dpp::consensus::basic::BasicError;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::{Document, DocumentV0};
use dpp::fee::Credits;
use dpp::platform_value::platform_value;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::shielded::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use dpp::version::PlatformVersion;

impl ShieldedWithdrawalTransitionActionV0 {
    /// Transforms the shielded withdrawal transition into an action
    pub fn try_from_transition(
        value: &ShieldedWithdrawalTransitionV0,
        current_total_balance: Credits,
        creation_time_ms: u64,
        fee_amount: Credits,
        platform_version: &PlatformVersion,
    ) -> ConsensusValidationResult<Self> {
        let notes: Vec<ShieldedActionNote> =
            value.actions.iter().map(ShieldedActionNote::from).collect();

        // The withdrawal document records the NET amount actually leaving the platform
        // to Core, i.e. `unshielding_amount - fee_amount`. The fee is carved out of the
        // unshielding amount and stays in-platform (routed to the fee pools).
        //
        // That net amount becomes a Core `TxOut`, so it must fall within the same
        // `[min_withdrawal_amount, max_withdrawal_amount]` range the transparent withdrawal
        // paths enforce (dust floor and per-transition policy cap). Consensus validation
        // (`validate_minimum_shielded_fee`) already rejects any transition whose net falls
        // outside that range, so for validated input this `checked_sub` is always
        // `Some(net)` in range. We re-check here (rather than `saturating_sub`) so a direct
        // or future caller that bypasses validation fails loudly instead of silently
        // constructing an out-of-range withdrawal document.
        let min_withdrawal_amount = platform_version.system_limits.min_withdrawal_amount;
        let max_withdrawal_amount = platform_version.system_limits.max_withdrawal_amount;
        let net_withdrawal_amount = match value.unshielding_amount.checked_sub(fee_amount) {
            Some(net) if net >= min_withdrawal_amount && net <= max_withdrawal_amount => net,
            Some(net) if net > max_withdrawal_amount => {
                // Over the per-transition withdrawal cap.
                return ConsensusValidationResult::new_with_error(
                    InvalidIdentityCreditWithdrawalTransitionAmountError::new(
                        net,
                        min_withdrawal_amount,
                        max_withdrawal_amount,
                    )
                    .into(),
                );
            }
            net => {
                // Below the dust floor (or fee exceeds the gross — underflow).
                return ConsensusValidationResult::new_with_error(
                    BasicError::WithdrawalBelowMinAmountError(WithdrawalBelowMinAmountError::new(
                        net.unwrap_or(0),
                        min_withdrawal_amount,
                        max_withdrawal_amount,
                    ))
                    .into(),
                );
            }
        };

        // Generate entropy from first nullifier + output_script for document ID
        let mut entropy = Vec::new();
        if let Some(first_note) = notes.first() {
            entropy.extend_from_slice(&first_note.nullifier);
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
            withdrawal::properties::AMOUNT: net_withdrawal_amount,
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
            amount: value.unshielding_amount,
            notes,
            anchor: value.anchor,
            core_fee_per_byte: value.core_fee_per_byte,
            pooling: value.pooling,
            output_script: value.output_script.clone(),
            fee_amount,
            current_total_balance,
            prepared_withdrawal_document: withdrawal_document,
        })
    }
}
