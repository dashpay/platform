use anyhow::bail;
use std::convert::TryInto;

use crate::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use crate::contracts::withdrawals_contract;
use crate::data_trigger::DataTriggerExecutionContext;
use crate::data_trigger::DataTriggerExecutionResult;
use crate::document::Document;
use crate::get_from_transition;
use crate::prelude::DocumentTransition;
use crate::prelude::Identifier;
use crate::state_repository::StateRepositoryLike;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::platform_value;

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

    let withdrawals_not_converted = context
        .state_repository
        .fetch_documents(
            &context.data_contract.id(),
            withdrawals_contract::document_types::WITHDRAWAL,
            platform_value!({
                "where" : [
                    ["$id", "==", dt_delete.base.id.to_buffer()],
                ]
            }),
            Some(context.state_transition_execution_context),
        )
        .await?;

    let withdrawals: Vec<Document> = withdrawals_not_converted
        .into_iter()
        .map(|d| d.try_into().map_err(Into::<ProtocolError>::into))
        .collect::<Result<Vec<Document>, ProtocolError>>()?;

    let Some(withdrawal) = withdrawals.get(0) else {
        result.add_error(DataTriggerConditionError::new(
            context.data_contract.id(),
            dt_delete.base.id,
            "Withdrawal document was not found".to_string(),
        ).into());

        return Ok(result);
    };

    let status: u8 = withdrawal.properties.get_integer("status")?;

    if status != withdrawals_contract::WithdrawalStatus::COMPLETE as u8
        || status != withdrawals_contract::WithdrawalStatus::EXPIRED as u8
    {
        result.add_error(
            DataTriggerConditionError::new(
                context.data_contract.id(),
                dt_delete.base.id,
                "withdrawal deletion is allowed only for COMPLETE and EXPIRED statuses".to_string(),
            )
            .into(),
        );

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
    use platform_value::platform_value;

    #[tokio::test]
    async fn should_throw_error_if_withdrawal_not_found() {
        let transition_execution_context = StateTransitionExecutionContext::default();
        let mut state_repository = MockStateRepositoryLike::new();
        let data_contract = get_data_contract_fixture(None).data_contract;
        let owner_id = &data_contract.owner_id;

        state_repository
            .expect_fetch_documents()
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
        let owner_id = data_contract.owner_id;

        let document = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": (0..23).collect::<Vec<u8>>(),
                "status": withdrawals_contract::WithdrawalStatus::BROADCASTED as u8,
                "transactionIndex": 1u64,
                "transactionSignHeight": 93u64,
                "transactionId": vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            }),
            None,
        ).expect("expected withdrawal document");

        state_repository
            .expect_fetch_documents()
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
