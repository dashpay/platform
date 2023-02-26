use anyhow::bail;
use serde_json::json;

use crate::contracts::withdrawals_contract;
use crate::data_trigger::DataTriggerError;
use crate::data_trigger::DataTriggerExecutionContext;
use crate::data_trigger::DataTriggerExecutionResult;
use crate::document::Document;
use crate::get_from_transition;
use crate::prelude::DocumentTransition;
use crate::prelude::Identifier;
use crate::state_repository::StateRepositoryLike;
use platform_value::btreemap_extensions::BTreeValueMapHelper;

pub async fn delete_withdrawal_data_trigger<'a, SR>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<'a, SR>,
    _top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
{
    let mut result = DataTriggerExecutionResult::default();

    let DocumentTransition::Delete(dt_delete) = document_transition else {
        bail!(
            "the Document Transition {} isn't 'DELETE'",
            get_from_transition!(document_transition, id)
        );
    };

    let withdrawals: Vec<Document> = context
        .state_repository
        .fetch_documents(
            &context.data_contract.id,
            withdrawals_contract::document_types::WITHDRAWAL,
            json!({
                "where" : [
                    ["$id", "==", dt_delete.base.id],
                ]
            }),
            context.state_transition_execution_context,
        )
        .await?;

    let Some(withdrawal) = withdrawals.get(0) else {
        let err = DataTriggerError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id,
            document_transition_id: dt_delete.base.id,
            message: "Withdrawal document was not found".to_string(),
            owner_id: Some(*context.owner_id),
            document_transition: Some(DocumentTransition::Delete(dt_delete.clone())),
        };

        result.add_error(err.into());

        return Ok(result);
    };

    let status: u8 = withdrawal.properties.get_integer("status")?;

    if status != withdrawals_contract::WithdrawalStatus::COMPLETE as u8
        || status != withdrawals_contract::WithdrawalStatus::EXPIRED as u8
    {
        let err = DataTriggerError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id,
            document_transition_id: dt_delete.base.id,
            message: "withdrawal deletion is allowed only for COMPLETE and EXPIRED statuses"
                .to_string(),
            owner_id: Some(*context.owner_id),
            document_transition: Some(DocumentTransition::Delete(dt_delete.clone())),
        };

        result.add_error(err.into());

        return Ok(result);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
    use crate::state_repository::MockStateRepositoryLike;
    use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
    use crate::system_data_contracts::load_system_data_contract;
    use crate::tests::fixtures::{get_data_contract_fixture, get_withdrawal_document_fixture};

    #[tokio::test]
    async fn should_throw_error_if_withdrawal_not_found() {
        let transition_execution_context = StateTransitionExecutionContext::default();
        let mut state_repository = MockStateRepositoryLike::new();
        let data_contract = get_data_contract_fixture(None);
        let owner_id = &data_contract.owner_id;

        state_repository
            .expect_fetch_documents::<Document>()
            .returning(|_, _, _, _| Ok(vec![]));

        let document_transition = DocumentTransition::Delete(Default::default());
        let data_trigger_context = DataTriggerExecutionContext {
            data_contract: &data_contract,
            owner_id,
            state_repository: &state_repository,
            state_transition_execution_context: &transition_execution_context,
        };

        let result =
            delete_withdrawal_data_trigger(&document_transition, &data_trigger_context, None)
                .await
                .expect("the execution result should be returned");

        assert!(!result.is_ok());

        let error = result.get_errors().get(0).unwrap();

        assert_eq!(error.to_string(), "Withdrawal document was not found");
    }

    #[tokio::test]
    async fn should_throw_error_if_withdrawal_has_wrong_status() {
        let transition_execution_context = StateTransitionExecutionContext::default();
        let mut state_repository = MockStateRepositoryLike::new();
        let data_contract =
            load_system_data_contract(data_contracts::SystemDataContract::Withdrawals)
                .expect("to load system data contract");
        let owner_id = data_contract.owner_id().clone();

        let document = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            json!({
                "amount": 1000,
                "coreFeePerByte": 1,
                "pooling": Pooling::Never,
                "outputScript": (0..23).collect::<Vec<u8>>(),
                "status": withdrawals_contract::WithdrawalStatus::BROADCASTED,
                "transactionIndex": 1,
                "transactionSignHeight": 93,
                "transactionId": vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            }),
            None,
        ).expect("expected withdrawal document");

        state_repository
            .expect_fetch_documents::<Document>()
            .return_once(move |_, _, _, _| Ok(vec![document]));

        let document_transition = DocumentTransition::Delete(Default::default());
        let data_trigger_context = DataTriggerExecutionContext {
            data_contract: &data_contract,
            owner_id: &owner_id,
            state_repository: &state_repository,
            state_transition_execution_context: &transition_execution_context,
        };

        let result =
            delete_withdrawal_data_trigger(&document_transition, &data_trigger_context, None)
                .await
                .expect("the execution result should be returned");

        assert!(!result.is_ok());

        let error = result.get_errors().get(0).unwrap();

        assert_eq!(
            error.to_string(),
            "withdrawal deletion is allowed only for COMPLETE and EXPIRED statuses"
        );
    }
}
