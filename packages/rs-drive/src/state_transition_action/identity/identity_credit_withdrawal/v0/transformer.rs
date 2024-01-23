use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::{Document, DocumentV0};
use dpp::platform_value::platform_value;
use dpp::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use dpp::withdrawal::Pooling;

impl IdentityCreditWithdrawalTransitionActionV0 {
    /// from identity credit withdrawal
    pub fn from_identity_credit_withdrawal(
        identity_credit_withdrawal: &IdentityCreditWithdrawalTransitionV0,
        creation_time_ms: u64,
    ) -> Self {
        let document_id = Document::generate_document_id_v0(
            &withdrawals_contract::ID,
            &identity_credit_withdrawal.identity_id,
            withdrawal::NAME,
            identity_credit_withdrawal.output_script.as_bytes(),
        );

        let document_data = platform_value!({
            withdrawal::properties::AMOUNT: identity_credit_withdrawal.amount,
            withdrawal::properties::CORE_FEE_PER_BYTE: identity_credit_withdrawal.core_fee_per_byte,
            withdrawal::properties::POOLING: Pooling::Never,
            withdrawal::properties::OUTPUT_SCRIPT: identity_credit_withdrawal.output_script.as_bytes(),
            withdrawal::properties::STATUS: withdrawals_contract::WithdrawalStatus::QUEUED,
        });

        let withdrawal_document = DocumentV0 {
            id: document_id,
            owner_id: identity_credit_withdrawal.identity_id,
            properties: document_data.into_btree_string_map().unwrap(),
            revision: Some(1),
            created_at: Some(creation_time_ms),
            updated_at: Some(creation_time_ms),
        }
        .into();

        IdentityCreditWithdrawalTransitionActionV0 {
            identity_id: identity_credit_withdrawal.identity_id,
            revision: identity_credit_withdrawal.revision,
            prepared_withdrawal_document: withdrawal_document,
        }
    }
}
