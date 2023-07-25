use crate::contracts::withdrawals_contract;
use crate::document::{generate_document_id, Document, DocumentV0};
use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use crate::withdrawal::Pooling;
use platform_value::platform_value;

impl IdentityCreditWithdrawalTransitionActionV0 {
    pub fn from_identity_credit_withdrawal(
        identity_credit_withdrawal: &IdentityCreditWithdrawalTransitionV0,
        creation_time_ms: u64,
    ) -> Self {
        let document_id = Document::generate_document_id_v0(
            &withdrawals_contract::CONTRACT_ID,
            &identity_credit_withdrawal.identity_id,
            withdrawals_contract::document_types::WITHDRAWAL,
            identity_credit_withdrawal.output_script.as_bytes(),
        );

        let document_data = platform_value!({
            withdrawals_contract::property_names::AMOUNT: identity_credit_withdrawal.amount,
            withdrawals_contract::property_names::CORE_FEE_PER_BYTE: identity_credit_withdrawal.core_fee_per_byte,
            withdrawals_contract::property_names::POOLING: Pooling::Never,
            withdrawals_contract::property_names::OUTPUT_SCRIPT: identity_credit_withdrawal.output_script.as_bytes(),
            withdrawals_contract::property_names::STATUS: withdrawals_contract::WithdrawalStatus::QUEUED,
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
