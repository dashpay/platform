use anyhow::{anyhow, bail};
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

pub async fn delete_withdrawal_data_trigger<'a, SR>(
    document_transition: &DocumentTransition,
    context: &DataTriggerExecutionContext<'a, SR>,
    _top_level_identity: Option<&Identifier>,
) -> Result<DataTriggerExecutionResult, anyhow::Error>
where
    SR: StateRepositoryLike,
{
    let mut result = DataTriggerExecutionResult::default();

    if context.state_transition_execution_context.is_dry_run() {
        return Ok(result);
    }

    let dt_delete = match document_transition {
        DocumentTransition::Delete(d) => d,
        _ => bail!(
            "the Document Transition {} isn't 'DELETE'",
            get_from_transition!(document_transition, id)
        ),
    };

    let withdrawals: Vec<Document> = context
        .state_repository
        .fetch_documents(
            &context.data_contract.id,
            withdrawals_contract::types::WITHDRAWAL,
            json!({
                "where" : [
                    ["$id", "==", dt_delete.base.id],
                ]
            }),
            context.state_transition_execution_context,
        )
        .await?;

    if withdrawals.is_empty() {
        let err = DataTriggerError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id.clone(),
            document_transition_id: dt_delete.base.id.clone(),
            message: "Withdrawal document was not found".to_string(),
            owner_id: Some(context.owner_id.clone()),
            document_transition: Some(DocumentTransition::Delete(dt_delete.clone())),
        };

        result.add_error(err.into());

        return Ok(result);
    }

    let withdrawal = withdrawals.get(0).unwrap();

    let status = withdrawal
        .get("status")
        .ok_or(anyhow!(
            "can't get withdrawal status property from the document"
        ))?
        .as_u64()
        .ok_or(anyhow!("can't convert withdrawal status to u64"))? as u8;

    if status != withdrawals_contract::statuses::COMPLETE
        || status != withdrawals_contract::statuses::EXPIRED
    {
        let err = DataTriggerError::DataTriggerConditionError {
            data_contract_id: context.data_contract.id.clone(),
            document_transition_id: dt_delete.base.id.clone(),
            message: "withdrawal deletion is allowed only for COMPLETE and EXPIRED statuses"
                .to_string(),
            owner_id: Some(context.owner_id.clone()),
            document_transition: Some(DocumentTransition::Delete(dt_delete.clone())),
        };

        result.add_error(err.into());

        return Ok(result);
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::delete_withdrawal_data_trigger;
    use crate::{
        data_trigger::DataTriggerExecutionContext,
        document::document_transition::DocumentTransition,
        state_repository::MockStateRepositoryLike,
        state_transition::state_transition_execution_context::StateTransitionExecutionContext,
        tests::fixtures::get_data_contract_fixture,
    };

    #[tokio::test]
    async fn should_successfully_execute_on_dry_run() {
        let transition_execution_context = StateTransitionExecutionContext::default();
        let state_repository = MockStateRepositoryLike::new();
        let data_contract = get_data_contract_fixture(None);
        let owner_id = data_contract.owner_id().to_owned();

        let document_transition = DocumentTransition::Create(Default::default());
        let data_trigger_context = DataTriggerExecutionContext {
            data_contract: &data_contract,
            owner_id: &owner_id,
            state_repository: &state_repository,
            state_transition_execution_context: &transition_execution_context,
        };

        transition_execution_context.enable_dry_run();

        let result =
            delete_withdrawal_data_trigger(&document_transition, &data_trigger_context, None)
                .await
                .expect("the execution result should be returned");

        assert!(result.is_ok());
    }
}
