use lazy_static::__Deref;
use serde_json::json;

use crate::{
    contracts::withdrawals_contract, document::Document,
    identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition,
    prelude::Identifier, state_repository::StateRepositoryLike, util::string_encoding::Encoding,
    NonConsensusError,
};

pub async fn validate_existing_withdrawal_id<SR>(
    state_repository: &SR,
    document_id: &Identifier,
    state_transition: &IdentityCreditWithdrawalTransition,
) -> Result<(), NonConsensusError>
where
    SR: StateRepositoryLike,
{
    let documents: Vec<Document> = state_repository
        .fetch_documents(
            withdrawals_contract::CONTRACT_ID.deref(),
            withdrawals_contract::types::WITHDRAWAL,
            json!({
                "where": [
                    ["$id", "==", document_id],
                ],
            }),
            &state_transition.execution_context,
        )
        .await?;

    if !documents.is_empty() {
        return Err(NonConsensusError::WithdrawalIdAlreadyExists {
            id_string: document_id.to_string(Encoding::Base58),
        });
    }

    Ok(())
}
